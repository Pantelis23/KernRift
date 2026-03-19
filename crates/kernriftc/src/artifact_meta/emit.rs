use std::fs;

use crate::sha256_hex;
use kernriftc::{BackendArtifactKind, SurfaceProfile};

use super::elf::parse_elf_object_artifact_metadata;
use super::krbo::parse_krbo_artifact_metadata;
use super::path::normalize_backend_artifact_input_path;
use super::report::{BACKEND_ARTIFACT_META_SCHEMA_VERSION, BackendArtifactMetadata};

pub(crate) fn write_backend_artifact_sidecar(
    kind: BackendArtifactKind,
    surface: SurfaceProfile,
    input_path: &str,
    meta_output_path: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let metadata = build_backend_artifact_metadata(kind, surface, input_path, bytes)?;
    let mut text = serde_json::to_string_pretty(&metadata)
        .map_err(|err| format!("failed to serialize '{}': {}", meta_output_path, err))?;
    text.push('\n');
    fs::write(meta_output_path, text)
        .map_err(|err| format!("failed to write '{}': {}", meta_output_path, err))
}

fn build_backend_artifact_metadata(
    kind: BackendArtifactKind,
    surface: SurfaceProfile,
    input_path: &str,
    bytes: &[u8],
) -> Result<BackendArtifactMetadata, String> {
    let (krbo, elfobj) = match kind {
        BackendArtifactKind::Krbo => (Some(parse_krbo_artifact_metadata(bytes)?), None),
        BackendArtifactKind::ElfObject => (None, Some(parse_elf_object_artifact_metadata(bytes)?)),
        BackendArtifactKind::ElfExecutable => {
            return Err("invalid emit mode: --meta-out is unsupported for 'elfexe'".to_string());
        }
        BackendArtifactKind::Asm => {
            return Err("invalid emit mode: --meta-out is unsupported for 'asm'".to_string());
        }
    };
    let (normalized_input_path, input_path_kind) =
        normalize_backend_artifact_input_path(input_path);

    Ok(BackendArtifactMetadata {
        schema_version: BACKEND_ARTIFACT_META_SCHEMA_VERSION,
        emit_kind: kind.as_str(),
        surface: surface.as_str(),
        byte_len: bytes.len(),
        sha256: sha256_hex(bytes),
        input_path: normalized_input_path,
        input_path_kind,
        krbo,
        elfobj,
    })
}
