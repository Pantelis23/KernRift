use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use emit::{
    ContractsSchema as EmitContractsSchema, emit_contracts_json_canonical_with_schema,
    emit_report_json,
};
use kernriftc::{
    analyze, check_file_with_surface, check_module, compile_file, compile_file_with_surface,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::args::{CheckArgs, CheckProfile, ContractsSchemaArg, VerifyArgs};
use crate::{
    ContractsBundle, ContractsFactSymbol, EXIT_INVALID_INPUT, EXIT_POLICY_VIOLATION, PolicyFile,
    PolicyViolation, VERIFY_REPORT_SCHEMA_V1, VERIFY_REPORT_SCHEMA_VERSION,
    canonicalize_provenance_fields, decode_contracts_bundle, evaluate_policy, load_policy_file,
    materialize_kernel_profile_policy, print_errors, print_policy_violations, print_usage,
    validate_json_against_schema_text,
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
struct DecodedVerifyReport {
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

pub(crate) fn run_check(args: &CheckArgs) -> ExitCode {
    if args.profile.is_none()
        && args.contracts_schema.is_none()
        && args.contracts_out.is_none()
        && args.policy_path.is_none()
        && args.hash_out.is_none()
        && args.sign_key_path.is_none()
        && args.sig_out.is_none()
    {
        return match check_file_with_surface(Path::new(&args.path), args.surface) {
            Ok(()) => ExitCode::SUCCESS,
            Err(errs) => {
                print_errors(&errs);
                ExitCode::from(EXIT_POLICY_VIOLATION)
            }
        };
    }

    let module = match compile_file_with_surface(Path::new(&args.path), args.surface) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(EXIT_POLICY_VIOLATION);
        }
    };
    let check_errs = match check_module(&module) {
        Ok(()) => Vec::new(),
        Err(errs) => errs,
    };
    let (report, analysis_errs) = analyze(&module);
    let mut semantic_errs = check_errs.clone();
    semantic_errs.extend(analysis_errs.clone());
    semantic_errs.sort();
    semantic_errs.dedup();

    let contracts_schema = match resolve_contracts_schema(args.profile, args.contracts_schema) {
        Ok(schema) => schema,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let contracts =
        match emit_contracts_json_canonical_with_schema(&module, &report, contracts_schema) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
    let contracts_bundle = match decode_contracts_bundle(&contracts, "<generated>") {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let mut policy_violations = Vec::<PolicyViolation>::new();

    if let Some(profile) = args.profile {
        let profile_policy = match load_profile_policy(profile) {
            Ok(policy) => policy,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        policy_violations.extend(evaluate_policy(&profile_policy, &contracts_bundle));
    }

    if let Some(policy_path) = args.policy_path.as_deref() {
        let file_policy = match load_policy_file(policy_path) {
            Ok(policy) => policy,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        policy_violations.extend(evaluate_policy(&file_policy, &contracts_bundle));
    }

    if !semantic_errs.is_empty() {
        print_errors(&semantic_errs);
    }
    if !policy_violations.is_empty() {
        policy_violations.sort();
        policy_violations.dedup();
        print_policy_violations(&policy_violations, false);
    }
    if !semantic_errs.is_empty() || !policy_violations.is_empty() {
        return ExitCode::from(EXIT_POLICY_VIOLATION);
    }

    let hash_hex = sha256_hex(contracts.as_bytes());
    let mut signature_b64 = None::<String>;
    if let Some(key_path) = args.sign_key_path.as_deref() {
        let signing_key = match load_signing_key_hex(key_path) {
            Ok(key) => key,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        let sig = signing_key.sign(contracts.as_bytes());
        signature_b64 = Some(BASE64_STANDARD.encode(sig.to_bytes()));
    }

    let mut outputs = Vec::<(String, String)>::new();
    if let Some(path) = args.contracts_out.as_ref() {
        outputs.push((path.clone(), contracts));
    }
    if let Some(path) = args.hash_out.as_ref() {
        outputs.push((path.clone(), format!("{hash_hex}\n")));
    }
    if let Some(path) = args.sig_out.as_ref() {
        let Some(sig_b64) = signature_b64.as_ref() else {
            eprintln!("internal error: missing signature text for --sig-out");
            return ExitCode::from(EXIT_INVALID_INPUT);
        };
        outputs.push((path.clone(), format!("{sig_b64}\n")));
    }

    if let Err(err) = write_output_files(&outputs) {
        eprintln!("{}", err);
        return ExitCode::from(EXIT_INVALID_INPUT);
    }

    ExitCode::SUCCESS
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
        report.contracts.schema_version = Some(contracts_bundle.schema_version.clone());

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

fn resolve_contracts_schema(
    profile: Option<CheckProfile>,
    requested: Option<ContractsSchemaArg>,
) -> Result<EmitContractsSchema, String> {
    if profile == Some(CheckProfile::Kernel) {
        if requested == Some(ContractsSchemaArg::V1) {
            return Err(
                "invalid check mode: --profile kernel requires contracts schema v2 (omit --contracts-schema or use --contracts-schema v2)"
                    .to_string(),
            );
        }
        return Ok(EmitContractsSchema::V2);
    }

    Ok(requested.unwrap_or(ContractsSchemaArg::V1).to_emit_schema())
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

fn decode_verify_report(
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

pub(crate) fn run_policy(policy_path: &str, contracts_path: &str, evidence: bool) -> ExitCode {
    let policy = match load_policy_file(policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let contracts_text = match fs::read_to_string(Path::new(contracts_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read contracts '{}': {}", contracts_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let contracts = match decode_contracts_bundle(&contracts_text, contracts_path) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let violations = evaluate_policy(&policy, &contracts);
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        print_policy_violations(&violations, evidence);
        ExitCode::from(EXIT_POLICY_VIOLATION)
    }
}

pub(crate) fn run_inspect(contracts_path: &str) -> ExitCode {
    let contracts_text = match fs::read_to_string(Path::new(contracts_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read contracts '{}': {}", contracts_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let contracts = match decode_contracts_bundle(&contracts_text, contracts_path) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    println!("{}", format_contracts_inspect_summary(&contracts));
    ExitCode::SUCCESS
}

pub(crate) fn run_inspect_report(report_path: &str) -> ExitCode {
    let report_text = match fs::read_to_string(Path::new(report_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read verify report '{}': {}", report_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let report = match decode_verify_report(&report_text, report_path) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    println!("{}", format_verify_report_inspect_summary(&report));
    ExitCode::SUCCESS
}

pub(crate) fn run_report(metrics_csv: &str, path: &str) -> ExitCode {
    let metrics = metrics_csv
        .split(',')
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>();

    if metrics.is_empty() {
        eprintln!("report metric list is empty");
        print_usage();
        return ExitCode::from(2);
    }

    for metric in &metrics {
        if metric != "max_lock_depth" && metric != "no_yield_spans" {
            eprintln!("unsupported report metric '{}'", metric);
            print_usage();
            return ExitCode::from(2);
        }
    }

    let module = match compile_file(Path::new(path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    let (report, errs) = analyze(&module);
    if !errs.is_empty() {
        print_errors(&errs);
        return ExitCode::from(1);
    }

    match emit_report_json(&report, &metrics) {
        Ok(text) => {
            println!("{}", text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::from(1)
        }
    }
}

fn write_output_files(outputs: &[(String, String)]) -> Result<(), String> {
    if outputs.is_empty() {
        return Ok(());
    }

    let mut final_paths = BTreeSet::<&str>::new();
    for (path, _) in outputs {
        if !final_paths.insert(path.as_str()) {
            return Err(format!("duplicate output path '{}'", path));
        }
        if Path::new(path).exists() {
            return Err(format!(
                "refusing to overwrite existing output '{}'; remove it first",
                path
            ));
        }
    }

    let mut staged = Vec::<(String, String)>::new();
    for (idx, (path, payload)) in outputs.iter().enumerate() {
        let tmp = format!("{}.kernriftc.tmp.{}.{}", path, std::process::id(), idx);
        fs::write(Path::new(&tmp), payload).map_err(|e| {
            cleanup_temp_paths(&staged);
            format!("failed to stage output '{}': {}", path, e)
        })?;
        staged.push((tmp, path.clone()));
    }

    let mut committed = Vec::<String>::new();
    for (tmp, final_path) in &staged {
        if let Err(err) = fs::rename(Path::new(tmp), Path::new(final_path)) {
            cleanup_temp_paths(&staged);
            cleanup_final_paths(&committed);
            return Err(format!(
                "failed to commit output '{}' from '{}': {}",
                final_path, tmp, err
            ));
        }
        committed.push(final_path.clone());
    }

    Ok(())
}

fn cleanup_temp_paths(staged: &[(String, String)]) {
    for (tmp, _) in staged {
        let _ = fs::remove_file(Path::new(tmp));
    }
}

fn cleanup_final_paths(paths: &[String]) {
    for path in paths {
        let _ = fs::remove_file(Path::new(path));
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

fn nibble_to_hex(n: u8) -> char {
    debug_assert!(n < 16);
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

fn normalize_hex(text: &str, expected_len: usize, label: &str) -> Result<String, String> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.len() != expected_len {
        return Err(format!(
            "invalid hex in '{}': expected {} hex chars, got {}",
            label,
            expected_len,
            normalized.len()
        ));
    }
    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "invalid hex in '{}': value contains non-hex characters",
            label
        ));
    }
    Ok(normalized)
}

fn decode_hex_fixed<const N: usize>(text: &str, label: &str) -> Result<[u8; N], String> {
    let normalized = normalize_hex(text, N * 2, label)?;
    let mut out = [0_u8; N];
    let bytes = normalized.as_bytes();
    for i in 0..N {
        let hi = hex_char_to_nibble(bytes[i * 2] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        let lo = hex_char_to_nibble(bytes[i * 2 + 1] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_char_to_nibble(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

fn load_signing_key_hex(path: &str) -> Result<SigningKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read signing key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

fn load_verifying_key_hex(path: &str) -> Result<VerifyingKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read public key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid public key '{}': {}", path, e))
}

fn load_profile_policy(profile: CheckProfile) -> Result<PolicyFile, String> {
    match profile {
        CheckProfile::Kernel => materialize_kernel_profile_policy(),
    }
}

fn format_contracts_inspect_summary(contracts: &ContractsBundle) -> String {
    let irq_reachable = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_ctx_reachable("irq")
    });
    let critical_functions =
        collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| symbol.attrs.critical);
    let alloc_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("alloc")
    });
    let block_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("block")
    });
    let yield_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("yield")
    });
    let cap_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        !symbol.caps_transitive.is_empty()
    });

    let mut critical_violations = contracts.report.critical.violations.clone();
    critical_violations.sort();
    critical_violations.dedup();

    let mut lines = vec![
        format!("schema: {}", contracts.schema_version),
        format!("symbols: total={}", contracts.facts.symbols.len()),
        "contexts:".to_string(),
        format!(
            "irq_reachable: {} {}",
            irq_reachable.len(),
            format_list(&irq_reachable)
        ),
        format!(
            "critical_functions: {} {}",
            critical_functions.len(),
            format_list(&critical_functions)
        ),
        "effects:".to_string(),
        format!(
            "alloc: {} {}",
            alloc_symbols.len(),
            format_list(&alloc_symbols)
        ),
        format!(
            "block: {} {}",
            block_symbols.len(),
            format_list(&block_symbols)
        ),
        format!(
            "yield: {} {}",
            yield_symbols.len(),
            format_list(&yield_symbols)
        ),
        "capabilities:".to_string(),
        format!(
            "symbols_with_caps: {} {}",
            cap_symbols.len(),
            format_list(&cap_symbols)
        ),
        "critical_report:".to_string(),
        format!("violations: {}", critical_violations.len()),
    ];

    for violation in critical_violations {
        let (direct, via_callee, via_extern) =
            canonicalize_provenance_fields(Some(&violation.provenance));
        lines.push(format!(
            "violation: function={} effect={} direct={} via_callee={} via_extern={}",
            violation.function,
            violation.effect,
            direct,
            format_list(&via_callee),
            format_list(&via_extern)
        ));
    }

    lines.join("\n")
}

fn format_verify_report_inspect_summary(report: &DecodedVerifyReport) -> String {
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

fn collect_sorted_symbol_names_by<F>(symbols: &[ContractsFactSymbol], predicate: F) -> Vec<String>
where
    F: Fn(&ContractsFactSymbol) -> bool,
{
    let mut out = symbols
        .iter()
        .filter(|symbol| predicate(symbol))
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn format_list(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

fn format_option_value(value: Option<&str>) -> &str {
    value.unwrap_or("<none>")
}

fn format_option_bool(value: Option<bool>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}
