mod args;
mod format;
mod report;

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use kernriftc::BackendArtifactKind;
use serde::Deserialize;
use serde_json::Value;

use super::{
    ElfObjectArtifactMetadata, KrboArtifactMetadata, parse_elf_object_artifact_metadata,
    parse_krbo_artifact_metadata, sha256_hex,
};

pub(crate) use args::{VerifyArtifactMetaArgs, parse_verify_artifact_meta_args};

const EXIT_POLICY_VIOLATION: u8 = super::EXIT_POLICY_VIOLATION;
const EXIT_INVALID_INPUT: u8 = super::EXIT_INVALID_INPUT;

#[derive(Debug, Clone, Deserialize)]
struct BackendArtifactMetadataInput {
    #[serde(rename = "schema_version")]
    _schema_version: String,
    emit_kind: String,
    #[serde(rename = "surface")]
    _surface: String,
    byte_len: usize,
    sha256: String,
    #[serde(rename = "input_path")]
    _input_path: String,
    #[serde(rename = "input_path_kind")]
    _input_path_kind: String,
    #[serde(default)]
    krbo: Option<KrboArtifactMetadataInput>,
    #[serde(default)]
    elfobj: Option<ElfObjectArtifactMetadataInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct KrboArtifactMetadataInput {
    magic: String,
    version_major: u8,
    version_minor: u8,
    format_revision: u16,
    target_tag: u8,
    target_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ElfObjectArtifactMetadataInput {
    magic: String,
    class: String,
    endianness: String,
    elf_type: String,
    machine: String,
}

enum VerifyArtifactMetaError {
    InvalidInput(String),
    Mismatch(String),
}

pub(crate) fn run_verify_artifact_meta(args: &VerifyArtifactMetaArgs) -> ExitCode {
    let artifact_bytes = match fs::read(Path::new(&args.artifact_path)) {
        Ok(bytes) => bytes,
        Err(err) => {
            return format::emit_verify_artifact_meta_result(
                args.format,
                "invalid_input",
                EXIT_INVALID_INPUT,
                format!("failed to read artifact '{}': {}", args.artifact_path, err),
            );
        }
    };

    let metadata_bytes = match fs::read(Path::new(&args.metadata_path)) {
        Ok(bytes) => bytes,
        Err(err) => {
            return format::emit_verify_artifact_meta_result(
                args.format,
                "invalid_input",
                EXIT_INVALID_INPUT,
                format!(
                    "failed to read artifact metadata '{}': {}",
                    args.metadata_path, err
                ),
            );
        }
    };

    let metadata = match decode_backend_artifact_metadata(&metadata_bytes, &args.metadata_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            return format::emit_verify_artifact_meta_result(
                args.format,
                "invalid_input",
                EXIT_INVALID_INPUT,
                err,
            );
        }
    };

    match verify_backend_artifact_metadata(&artifact_bytes, &metadata) {
        Ok(()) => format::emit_verify_artifact_meta_result(
            args.format,
            "pass",
            0,
            "verify-artifact-meta: PASS".to_string(),
        ),
        Err(VerifyArtifactMetaError::InvalidInput(err)) => {
            format::emit_verify_artifact_meta_result(
                args.format,
                "invalid_input",
                EXIT_INVALID_INPUT,
                err,
            )
        }
        Err(VerifyArtifactMetaError::Mismatch(err)) => format::emit_verify_artifact_meta_result(
            args.format,
            "mismatch",
            EXIT_POLICY_VIOLATION,
            err,
        ),
    }
}

fn decode_backend_artifact_metadata(
    bytes: &[u8],
    path: &str,
) -> Result<BackendArtifactMetadataInput, String> {
    let metadata_json: Value = serde_json::from_slice(bytes)
        .map_err(|err| format!("failed to decode artifact metadata '{}': {}", path, err))?;
    let schema_version = metadata_json
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "failed to decode artifact metadata '{}': missing string field 'schema_version'",
                path
            )
        })?;
    if schema_version != "kernrift_artifact_meta_v1" {
        return Err(format!(
            "unsupported artifact metadata schema_version '{}', expected 'kernrift_artifact_meta_v1'",
            schema_version
        ));
    }
    serde_json::from_value(metadata_json)
        .map_err(|err| format!("failed to decode artifact metadata '{}': {}", path, err))
}

fn infer_backend_artifact_kind(
    bytes: &[u8],
) -> Result<BackendArtifactKind, VerifyArtifactMetaError> {
    if bytes.len() >= 4 && &bytes[0..4] == b"KRBO" {
        Ok(BackendArtifactKind::Krbo)
    } else if bytes.len() >= 4 && &bytes[0..4] == b"\x7fELF" {
        Ok(BackendArtifactKind::ElfObject)
    } else {
        Err(VerifyArtifactMetaError::InvalidInput(
            "verify-artifact-meta: unsupported artifact bytes".to_string(),
        ))
    }
}

fn verify_backend_artifact_metadata(
    bytes: &[u8],
    metadata: &BackendArtifactMetadataInput,
) -> Result<(), VerifyArtifactMetaError> {
    let kind = infer_backend_artifact_kind(bytes)?;
    if metadata.emit_kind != kind.as_str() {
        return Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: emit_kind mismatch: metadata '{}', artifact '{}'",
            metadata.emit_kind,
            kind.as_str()
        )));
    }
    if metadata.byte_len != bytes.len() {
        return Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: byte_len mismatch: metadata {}, artifact {}",
            metadata.byte_len,
            bytes.len()
        )));
    }

    let actual_sha256 = sha256_hex(bytes);
    if metadata.sha256 != actual_sha256 {
        return Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: sha256 mismatch: metadata {}, artifact {}",
            metadata.sha256, actual_sha256
        )));
    }

    match kind {
        BackendArtifactKind::Krbo => {
            let expected = metadata.krbo.as_ref().ok_or_else(|| {
                VerifyArtifactMetaError::InvalidInput(
                    "verify-artifact-meta: metadata missing krbo block".to_string(),
                )
            })?;
            let actual = parse_krbo_artifact_metadata(bytes).map_err(|err| {
                VerifyArtifactMetaError::InvalidInput(format!("verify-artifact-meta: {}", err))
            })?;
            verify_krbo_artifact_metadata(expected, &actual)
        }
        BackendArtifactKind::ElfObject => {
            let expected = metadata.elfobj.as_ref().ok_or_else(|| {
                VerifyArtifactMetaError::InvalidInput(
                    "verify-artifact-meta: metadata missing elfobj block".to_string(),
                )
            })?;
            let actual = parse_elf_object_artifact_metadata(bytes).map_err(|err| {
                VerifyArtifactMetaError::InvalidInput(format!("verify-artifact-meta: {}", err))
            })?;
            verify_elf_object_artifact_metadata(expected, &actual)
        }
        BackendArtifactKind::Asm => Err(VerifyArtifactMetaError::InvalidInput(
            "verify-artifact-meta: unsupported artifact bytes".to_string(),
        )),
    }
}

fn verify_krbo_artifact_metadata(
    expected: &KrboArtifactMetadataInput,
    actual: &KrboArtifactMetadata,
) -> Result<(), VerifyArtifactMetaError> {
    verify_string_artifact_field("krbo.magic", &expected.magic, &actual.magic)?;
    verify_u8_artifact_field(
        "krbo.version_major",
        expected.version_major,
        actual.version_major,
    )?;
    verify_u8_artifact_field(
        "krbo.version_minor",
        expected.version_minor,
        actual.version_minor,
    )?;
    verify_u16_artifact_field(
        "krbo.format_revision",
        expected.format_revision,
        actual.format_revision,
    )?;
    verify_u8_artifact_field("krbo.target_tag", expected.target_tag, actual.target_tag)?;
    verify_string_artifact_field(
        "krbo.target_name",
        &expected.target_name,
        actual.target_name,
    )?;
    Ok(())
}

fn verify_elf_object_artifact_metadata(
    expected: &ElfObjectArtifactMetadataInput,
    actual: &ElfObjectArtifactMetadata,
) -> Result<(), VerifyArtifactMetaError> {
    verify_string_artifact_field("elfobj.magic", &expected.magic, &actual.magic)?;
    verify_string_artifact_field("elfobj.class", &expected.class, actual.class)?;
    verify_string_artifact_field("elfobj.endianness", &expected.endianness, actual.endianness)?;
    verify_string_artifact_field("elfobj.elf_type", &expected.elf_type, actual.elf_type)?;
    verify_string_artifact_field("elfobj.machine", &expected.machine, actual.machine)?;
    Ok(())
}

fn verify_string_artifact_field(
    field: &str,
    expected: &str,
    actual: &str,
) -> Result<(), VerifyArtifactMetaError> {
    if expected == actual {
        Ok(())
    } else {
        Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: {} mismatch: metadata '{}', artifact '{}'",
            field, expected, actual
        )))
    }
}

fn verify_u8_artifact_field(
    field: &str,
    expected: u8,
    actual: u8,
) -> Result<(), VerifyArtifactMetaError> {
    if expected == actual {
        Ok(())
    } else {
        Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: {} mismatch: metadata {}, artifact {}",
            field, expected, actual
        )))
    }
}

fn verify_u16_artifact_field(
    field: &str,
    expected: u16,
    actual: u16,
) -> Result<(), VerifyArtifactMetaError> {
    if expected == actual {
        Ok(())
    } else {
        Err(VerifyArtifactMetaError::Mismatch(format!(
            "verify-artifact-meta: {} mismatch: metadata {}, artifact {}",
            field, expected, actual
        )))
    }
}
