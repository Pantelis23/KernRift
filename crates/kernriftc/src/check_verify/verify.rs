use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signature, Verifier};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::args::VerifyArgs;
use super::crypto::{load_verifying_key_hex, normalize_hex, sha256_hex};
use super::output::write_output_files;
use crate::policy_engine::{
    contracts_bundle_schema_version, decode_contracts_bundle, validate_json_against_schema_text,
};
use crate::{
    EXIT_INVALID_INPUT, EXIT_POLICY_VIOLATION, VERIFY_REPORT_SCHEMA_V1,
    VERIFY_REPORT_SCHEMA_VERSION, print_errors,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerifyStatus {
    Pass,
    Deny,
    InvalidInput,
}

impl VerifyStatus {
    fn as_exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Deny => EXIT_POLICY_VIOLATION,
            Self::InvalidInput => EXIT_INVALID_INPUT,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Deny => "deny",
            Self::InvalidInput => "invalid_input",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReport {
    schema_version: &'static str,
    result: &'static str,
    inputs: VerifyReportInputs,
    hash: VerifyReportHash,
    contracts: VerifyReportContracts,
    signature: VerifyReportSignature,
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DecodedVerifyReport {
    schema_version: String,
    result: String,
    inputs: DecodedVerifyReportInputs,
    hash: DecodedVerifyReportHash,
    contracts: DecodedVerifyReportContracts,
    signature: DecodedVerifyReportSignature,
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportInputs {
    contracts: String,
    hash: String,
    sig: Option<String>,
    pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportHash {
    expected_sha256: Option<String>,
    computed_sha256: Option<String>,
    matched: bool,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportContracts {
    utf8_valid: bool,
    schema_valid: bool,
    schema_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportSignature {
    checked: bool,
    valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportInputs {
    contracts: String,
    hash: String,
    sig: Option<String>,
    pubkey: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportHash {
    expected_sha256: Option<String>,
    computed_sha256: Option<String>,
    matched: bool,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportContracts {
    utf8_valid: bool,
    schema_valid: bool,
    schema_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportSignature {
    checked: bool,
    valid: Option<bool>,
}

pub(crate) fn run_verify(args: &VerifyArgs) -> ExitCode {
    let mut status = VerifyStatus::Pass;
    let mut report = new_verify_report(args);

    'verify: {
        let contracts_bytes = match fs::read(Path::new(&args.contracts_path)) {
            Ok(bytes) => bytes,
            Err(err) => {
                report.diagnostics.push(format!(
                    "failed to read contracts '{}': {}",
                    args.contracts_path, err
                ));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        let computed_hash = sha256_hex(&contracts_bytes);
        report.hash.computed_sha256 = Some(computed_hash.clone());

        let hash_text = match fs::read_to_string(Path::new(&args.hash_path)) {
            Ok(text) => text,
            Err(err) => {
                report
                    .diagnostics
                    .push(format!("failed to read hash '{}': {}", args.hash_path, err));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        let expected_hash = match normalize_hex(&hash_text, 64, &args.hash_path) {
            Ok(hex) => hex,
            Err(err) => {
                report.diagnostics.push(err);
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.hash.expected_sha256 = Some(expected_hash.clone());
        if computed_hash != expected_hash {
            report.diagnostics.push(format!(
                "verify: HASH_MISMATCH: expected {}, got {}",
                expected_hash, computed_hash
            ));
            status = VerifyStatus::Deny;
            break 'verify;
        }
        report.hash.matched = true;

        let contracts_text = match std::str::from_utf8(&contracts_bytes) {
            Ok(text) => text,
            Err(err) => {
                report.diagnostics.push(format!(
                    "failed to decode contracts '{}' as UTF-8: {}",
                    args.contracts_path, err
                ));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.contracts.utf8_valid = true;

        let contracts_bundle = match decode_contracts_bundle(contracts_text, &args.contracts_path) {
            Ok(bundle) => bundle,
            Err(err) => {
                report.diagnostics.push(err);
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.contracts.schema_valid = true;
        report.contracts.schema_version =
            Some(contracts_bundle_schema_version(&contracts_bundle).to_string());

        if let (Some(sig_path), Some(pubkey_path)) =
            (args.sig_path.as_ref(), args.pubkey_path.as_ref())
        {
            report.signature.checked = true;
            let sig_text = match fs::read_to_string(Path::new(sig_path)) {
                Ok(text) => text,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("failed to read signature '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let sig_bytes = match BASE64_STANDARD.decode(sig_text.trim()) {
                Ok(bytes) => bytes,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("invalid base64 signature '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let sig = match Signature::from_slice(&sig_bytes) {
                Ok(sig) => sig,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("invalid signature bytes '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let verifying_key = match load_verifying_key_hex(pubkey_path) {
                Ok(key) => key,
                Err(err) => {
                    report.diagnostics.push(err);
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            if let Err(err) = verifying_key.verify(&contracts_bytes, &sig) {
                report
                    .diagnostics
                    .push(format!("verify: SIG_MISMATCH: {}", err));
                report.signature.valid = Some(false);
                status = VerifyStatus::Deny;
                break 'verify;
            }
            report.signature.valid = Some(true);
        }
    }

    report.result = status.as_str();

    let mut stderr_diagnostics = report.diagnostics.clone();
    stderr_diagnostics.sort();
    stderr_diagnostics.dedup();

    report.diagnostics = stderr_diagnostics
        .iter()
        .map(|diag| normalize_verify_diagnostic_for_report(diag, args))
        .collect();
    report.diagnostics.sort();
    report.diagnostics.dedup();

    if let Some(report_path) = args.report_path.as_ref() {
        let report_json = match emit_verify_report_json(&report) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        if let Err(err) = write_output_files(&[(report_path.clone(), report_json)]) {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    }

    print_errors(&stderr_diagnostics);

    ExitCode::from(status.as_exit_code())
}

fn new_verify_report(args: &VerifyArgs) -> VerifyReport {
    VerifyReport {
        schema_version: VERIFY_REPORT_SCHEMA_VERSION,
        result: VerifyStatus::InvalidInput.as_str(),
        inputs: VerifyReportInputs {
            contracts: stable_display_path(&args.contracts_path),
            hash: stable_display_path(&args.hash_path),
            sig: args.sig_path.as_deref().map(stable_display_path),
            pubkey: args.pubkey_path.as_deref().map(stable_display_path),
        },
        hash: VerifyReportHash {
            expected_sha256: None,
            computed_sha256: None,
            matched: false,
        },
        contracts: VerifyReportContracts {
            utf8_valid: false,
            schema_valid: false,
            schema_version: None,
        },
        signature: VerifyReportSignature {
            checked: false,
            valid: None,
        },
        diagnostics: Vec::new(),
    }
}

fn emit_verify_report_json(report: &VerifyReport) -> Result<String, String> {
    let value = serde_json::to_value(report)
        .map_err(|e| format!("failed to serialize verify report JSON: {}", e))?;
    let canonical = canonicalize_json_value(&value);
    validate_json_against_schema_text(
        &canonical,
        VERIFY_REPORT_SCHEMA_V1,
        "embedded verify report schema",
        "verify report",
    )?;
    serde_json::to_string_pretty(&canonical)
        .map_err(|e| format!("failed to format verify report JSON: {}", e))
}

pub(super) fn decode_verify_report(
    report_text: &str,
    source_name: &str,
) -> Result<DecodedVerifyReport, String> {
    let report_json: Value = serde_json::from_str(report_text).map_err(|e| {
        format!(
            "failed to parse verify report JSON '{}': {}",
            source_name, e
        )
    })?;
    let schema_version = report_json
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "failed to decode verify report '{}': missing string field 'schema_version'",
                source_name
            )
        })?;
    if schema_version != VERIFY_REPORT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported verify report schema_version '{}', expected '{}'",
            schema_version, VERIFY_REPORT_SCHEMA_VERSION
        ));
    }
    validate_json_against_schema_text(
        &report_json,
        VERIFY_REPORT_SCHEMA_V1,
        "embedded verify report schema",
        "verify report",
    )?;

    serde_json::from_value(report_json).map_err(|e| {
        format!(
            "failed to decode verify report '{}' into inspect model: {}",
            source_name, e
        )
    })
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize_json_value(v)))
                .collect::<BTreeMap<_, _>>();
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn stable_display_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let p = Path::new(path);
    if p.is_absolute() {
        p.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| normalized)
    } else {
        normalized
    }
}

fn normalize_verify_diagnostic_for_report(diag: &str, args: &VerifyArgs) -> String {
    let mut out = diag.replace('\\', "/");
    let mut replacements = vec![
        (
            args.contracts_path.as_str(),
            stable_display_path(&args.contracts_path),
        ),
        (
            args.hash_path.as_str(),
            stable_display_path(&args.hash_path),
        ),
    ];
    if let Some(sig) = args.sig_path.as_deref() {
        replacements.push((sig, stable_display_path(sig)));
    }
    if let Some(pubkey) = args.pubkey_path.as_deref() {
        replacements.push((pubkey, stable_display_path(pubkey)));
    }

    for (raw, stable) in replacements {
        let normalized = raw.replace('\\', "/");
        out = out.replace(&normalized, &stable);
    }

    out
}

pub(super) fn format_verify_report_inspect_summary(report: &DecodedVerifyReport) -> String {
    let mut lines = vec![
        format!("schema: {}", report.schema_version),
        format!("result: {}", report.result),
        "inputs:".to_string(),
        format!("contracts: {}", report.inputs.contracts),
        format!("hash: {}", report.inputs.hash),
        format!("sig: {}", format_option_value(report.inputs.sig.as_deref())),
        format!(
            "pubkey: {}",
            format_option_value(report.inputs.pubkey.as_deref())
        ),
        "hash_status:".to_string(),
        format!("matched: {}", report.hash.matched),
        format!(
            "expected_sha256: {}",
            format_option_value(report.hash.expected_sha256.as_deref())
        ),
        format!(
            "computed_sha256: {}",
            format_option_value(report.hash.computed_sha256.as_deref())
        ),
        "contracts_status:".to_string(),
        format!("utf8_valid: {}", report.contracts.utf8_valid),
        format!("schema_valid: {}", report.contracts.schema_valid),
        format!(
            "schema_version: {}",
            format_option_value(report.contracts.schema_version.as_deref())
        ),
        "signature_status:".to_string(),
        format!("checked: {}", report.signature.checked),
        format!("valid: {}", format_option_bool(report.signature.valid)),
        format!("diagnostics: {}", report.diagnostics.len()),
    ];

    for diagnostic in &report.diagnostics {
        lines.push(format!("diagnostic: {}", diagnostic));
    }

    lines.join("\n")
}

fn format_option_value(value: Option<&str>) -> &str {
    value.unwrap_or("<none>")
}

fn format_option_bool(value: Option<bool>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}
