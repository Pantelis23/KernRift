use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use emit::{
    ContractsSchema as EmitContractsSchema, emit_caps_manifest_json, emit_contracts_json,
    emit_contracts_json_canonical, emit_contracts_json_canonical_with_schema,
    emit_contracts_json_with_schema, emit_krir_json, emit_lockgraph_json, emit_report_json,
};
use jsonschema::JSONSchema;
use kernriftc::{analyze, check_file, check_module, compile_file};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const CONTRACTS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v1.schema.json");
const CONTRACTS_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v2.schema.json");
const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");
const KERNEL_POLICY_PROFILE: &str = include_str!("../../../policies/kernel.toml");
const CONTRACTS_SCHEMA_VERSION: &str = "kernrift_contracts_v1";
const CONTRACTS_SCHEMA_VERSION_V2: &str = "kernrift_contracts_v2";
const VERIFY_REPORT_SCHEMA_VERSION: &str = "kernrift_verify_report_v1";
const EXIT_POLICY_VIOLATION: u8 = 1;
const EXIT_INVALID_INPUT: u8 = 2;

#[derive(Debug, Deserialize, Default)]
struct PolicyFile {
    #[serde(default)]
    limits: PolicyLimits,
    #[serde(default)]
    locks: PolicyLocks,
    #[serde(default)]
    caps: PolicyCaps,
    #[serde(default)]
    kernel: PolicyKernel,
}

#[derive(Debug, Deserialize, Default)]
struct PolicyLimits {
    #[serde(default)]
    max_lock_depth: Option<u64>,
    #[serde(default)]
    max_no_yield_span: Option<u64>,
    #[serde(default)]
    forbid_unbounded_no_yield: bool,
}

#[derive(Debug, Deserialize, Default)]
struct PolicyLocks {
    #[serde(default)]
    forbid_edges: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize, Default)]
struct PolicyCaps {
    #[serde(default)]
    allow_module: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PolicyKernel {
    #[serde(default)]
    forbid_alloc_in_irq: bool,
    #[serde(default)]
    forbid_block_in_irq: bool,
    #[serde(default)]
    forbid_yield_in_critical: bool,
}

#[derive(Debug, Deserialize)]
struct ContractsBundle {
    schema_version: String,
    capabilities: ContractsCapabilities,
    facts: ContractsFacts,
    lockgraph: ContractsLockgraph,
    report: ContractsReport,
}

#[derive(Debug, Deserialize)]
struct ContractsCapabilities {
    module_caps: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ContractsLockgraph {
    edges: Vec<ContractsLockEdge>,
}

#[derive(Debug, Deserialize)]
struct ContractsLockEdge {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct ContractsReport {
    max_lock_depth: u64,
    no_yield_spans: BTreeMap<String, ContractsNoYieldSpan>,
    #[serde(default)]
    contexts: ContractsReportContexts,
    #[serde(default)]
    effects: ContractsReportEffects,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsFacts {
    symbols: Vec<ContractsFactSymbol>,
}

#[derive(Debug, Deserialize)]
struct ContractsFactSymbol {
    name: String,
    ctx_ok: Vec<String>,
    eff_used: Vec<String>,
    attrs: ContractsFactAttrs,
}

impl ContractsFactSymbol {
    fn has_ctx(&self, ctx: &str) -> bool {
        self.ctx_ok.iter().any(|c| c == ctx)
    }

    fn has_eff(&self, eff: &str) -> bool {
        self.eff_used.iter().any(|e| e == eff)
    }
}

#[derive(Debug, Deserialize)]
struct ContractsFactAttrs {
    noyield: bool,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsReportContexts {
    #[serde(default)]
    irq_functions: Vec<String>,
    #[serde(default)]
    critical_functions: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsReportEffects {
    #[serde(default)]
    yield_sites_count: u64,
    #[serde(default)]
    alloc_sites_count: u64,
    #[serde(default)]
    block_sites_count: u64,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
enum ContractsNoYieldSpan {
    Bounded(u64),
    Unbounded(String),
}

impl ContractsNoYieldSpan {
    fn is_unbounded(&self) -> bool {
        matches!(self, Self::Unbounded(v) if v == "unbounded")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PolicyViolation {
    code: &'static str,
    msg: String,
}

#[derive(Debug)]
struct CheckArgs {
    path: String,
    profile: Option<CheckProfile>,
    contracts_schema: Option<ContractsSchemaArg>,
    contracts_out: Option<String>,
    policy_path: Option<String>,
    hash_out: Option<String>,
    sign_key_path: Option<String>,
    sig_out: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckProfile {
    Kernel,
}

impl CheckProfile {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "kernel" => Ok(Self::Kernel),
            other => Err(format!(
                "invalid check mode: unknown profile '{}', expected 'kernel'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractsSchemaArg {
    V1,
    V2,
}

impl ContractsSchemaArg {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "v1" => Ok(Self::V1),
            "v2" => Ok(Self::V2),
            other => Err(format!(
                "invalid check mode: unknown contracts schema '{}', expected 'v1' or 'v2'",
                other
            )),
        }
    }

    fn to_emit_schema(self) -> EmitContractsSchema {
        match self {
            Self::V1 => EmitContractsSchema::V1,
            Self::V2 => EmitContractsSchema::V2,
        }
    }
}

#[derive(Debug)]
struct VerifyArgs {
    contracts_path: String,
    hash_path: String,
    sig_path: Option<String>,
    pubkey_path: Option<String>,
    report_path: Option<String>,
}

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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("kernriftc {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(2);
    }

    match args[1].as_str() {
        "check" => match parse_check_args(&args[2..]) {
            Ok(parsed) => run_check(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "verify" => match parse_verify_args(&args[2..]) {
            Ok(parsed) => run_verify(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "policy" => {
            if args.len() == 6 && args[2] == "--policy" && args[4] == "--contracts" {
                run_policy(&args[3], &args[5])
            } else {
                eprintln!("invalid policy mode");
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        }
        "--selftest" => {
            if args.len() != 2 {
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            run_selftest()
        }
        "--emit" => {
            if args.len() >= 3 && args[2] == "report" {
                if args.len() != 6 || args[3] != "--metrics" {
                    eprintln!("invalid emit report mode");
                    print_usage();
                    return ExitCode::from(EXIT_INVALID_INPUT);
                }
                run_report(&args[4], &args[5])
            } else {
                if args.len() != 4 {
                    eprintln!("invalid emit mode");
                    print_usage();
                    return ExitCode::from(EXIT_INVALID_INPUT);
                }
                run_emit(&args[2], &args[3])
            }
        }
        "--report" => {
            if args.len() != 4 {
                eprintln!("invalid report mode");
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            run_report(&args[2], &args[3])
        }
        _ => {
            print_usage();
            ExitCode::from(EXIT_INVALID_INPUT)
        }
    }
}

fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut profile = None::<CheckProfile>;
    let mut contracts_schema = None::<ContractsSchemaArg>;
    let mut contracts_out = None::<String>;
    let mut policy_path = None::<String>;
    let mut hash_out = None::<String>;
    let mut sign_key_path = None::<String>;
    let mut sig_out = None::<String>;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--profile" => {
                if profile.is_some() {
                    return Err("invalid check mode: duplicate --profile".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --profile requires a value".to_string());
                };
                profile = Some(CheckProfile::parse(value)?);
                idx += 2;
            }
            "--contracts-schema" => {
                if contracts_schema.is_some() {
                    return Err("invalid check mode: duplicate --contracts-schema".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-schema requires a value".to_string()
                    );
                };
                contracts_schema = Some(ContractsSchemaArg::parse(value)?);
                idx += 2;
            }
            "--contracts-out" => {
                if contracts_out.is_some() {
                    return Err("invalid check mode: duplicate --contracts-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-out requires a file path".to_string()
                    );
                };
                contracts_out = Some(value.clone());
                idx += 2;
            }
            "--policy" => {
                if policy_path.is_some() {
                    return Err("invalid check mode: duplicate --policy".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --policy requires a file path".to_string());
                };
                policy_path = Some(value.clone());
                idx += 2;
            }
            "--hash-out" => {
                if hash_out.is_some() {
                    return Err("invalid check mode: duplicate --hash-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --hash-out requires a file path".to_string());
                };
                hash_out = Some(value.clone());
                idx += 2;
            }
            "--sign-ed25519" => {
                if sign_key_path.is_some() {
                    return Err("invalid check mode: duplicate --sign-ed25519".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --sign-ed25519 requires a key file path".to_string(),
                    );
                };
                sign_key_path = Some(value.clone());
                idx += 2;
            }
            "--sig-out" => {
                if sig_out.is_some() {
                    return Err("invalid check mode: duplicate --sig-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --sig-out requires a file path".to_string());
                };
                sig_out = Some(value.clone());
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("invalid check mode: unknown flag '{}'", other));
            }
            _ => {
                positionals.push(args[idx].clone());
                idx += 1;
            }
        }
    }

    if positionals.len() != 1 {
        return Err("invalid check mode: expected exactly one <file.kr> input".to_string());
    }

    if sign_key_path.is_some() ^ sig_out.is_some() {
        return Err(
            "invalid check mode: --sign-ed25519 and --sig-out must be provided together"
                .to_string(),
        );
    }

    Ok(CheckArgs {
        path: positionals.remove(0),
        profile,
        contracts_schema,
        contracts_out,
        policy_path,
        hash_out,
        sign_key_path,
        sig_out,
    })
}

fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, String> {
    let mut contracts_path = None::<String>;
    let mut hash_path = None::<String>;
    let mut sig_path = None::<String>;
    let mut pubkey_path = None::<String>;
    let mut report_path = None::<String>;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid verify mode: duplicate --contracts".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --contracts requires a file path".to_string());
                };
                contracts_path = Some(value.clone());
                idx += 2;
            }
            "--hash" => {
                if hash_path.is_some() {
                    return Err("invalid verify mode: duplicate --hash".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --hash requires a file path".to_string());
                };
                hash_path = Some(value.clone());
                idx += 2;
            }
            "--sig" => {
                if sig_path.is_some() {
                    return Err("invalid verify mode: duplicate --sig".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --sig requires a file path".to_string());
                };
                sig_path = Some(value.clone());
                idx += 2;
            }
            "--pubkey" => {
                if pubkey_path.is_some() {
                    return Err("invalid verify mode: duplicate --pubkey".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --pubkey requires a file path".to_string());
                };
                pubkey_path = Some(value.clone());
                idx += 2;
            }
            "--report" => {
                if report_path.is_some() {
                    return Err("invalid verify mode: duplicate --report".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --report requires a file path".to_string());
                };
                report_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(format!("invalid verify mode: unknown token '{}'", other));
            }
        }
    }

    let Some(contracts_path) = contracts_path else {
        return Err("invalid verify mode: missing --contracts <contracts.json>".to_string());
    };
    let Some(hash_path) = hash_path else {
        return Err("invalid verify mode: missing --hash <contracts.sha256>".to_string());
    };
    if sig_path.is_some() ^ pubkey_path.is_some() {
        return Err(
            "invalid verify mode: --sig and --pubkey must be provided together".to_string(),
        );
    }

    Ok(VerifyArgs {
        contracts_path,
        hash_path,
        sig_path,
        pubkey_path,
        report_path,
    })
}

fn run_check(args: &CheckArgs) -> ExitCode {
    if args.profile.is_none()
        && args.contracts_schema.is_none()
        && args.contracts_out.is_none()
        && args.policy_path.is_none()
        && args.hash_out.is_none()
        && args.sign_key_path.is_none()
        && args.sig_out.is_none()
    {
        return match check_file(Path::new(&args.path)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(errs) => {
                print_errors(&errs);
                ExitCode::from(EXIT_POLICY_VIOLATION)
            }
        };
    }

    let module = match compile_file(Path::new(&args.path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(EXIT_POLICY_VIOLATION);
        }
    };
    if let Err(errs) = check_module(&module) {
        print_errors(&errs);
        return ExitCode::from(EXIT_POLICY_VIOLATION);
    }
    let (report, errs) = analyze(&module);
    if !errs.is_empty() {
        print_errors(&errs);
        return ExitCode::from(EXIT_POLICY_VIOLATION);
    }

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

    if !policy_violations.is_empty() {
        policy_violations.sort();
        policy_violations.dedup();
        print_policy_violations(&policy_violations);
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

fn run_verify(args: &VerifyArgs) -> ExitCode {
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

fn run_policy(policy_path: &str, contracts_path: &str) -> ExitCode {
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
        print_policy_violations(&violations);
        ExitCode::from(EXIT_POLICY_VIOLATION)
    }
}

fn run_selftest() -> ExitCode {
    let mut failures = Vec::<String>::new();

    if let Err(err) = selftest_fixture_suites() {
        failures.push(err);
    }
    if let Err(err) = selftest_exact_diagnostics() {
        failures.push(err);
    }
    if let Err(err) = selftest_json_schemas() {
        failures.push(err);
    }
    if let Err(err) = selftest_policy_engine() {
        failures.push(err);
    }

    if failures.is_empty() {
        println!("selftest: PASS");
        ExitCode::SUCCESS
    } else {
        for failure in failures {
            eprintln!("{}", failure);
        }
        ExitCode::from(1)
    }
}

fn run_emit(kind: &str, path: &str) -> ExitCode {
    let module = match compile_file(Path::new(path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    match kind {
        "krir" => print_json_result(emit_krir_json(&module), "KRIR JSON"),
        "caps" => print_json_result(emit_caps_manifest_json(&module), "caps manifest JSON"),
        "lockgraph" => {
            let (report, errs) = analyze(&module);
            if !errs.is_empty() {
                print_errors(&errs);
                return ExitCode::from(1);
            }
            print_json_result(emit_lockgraph_json(&report), "lockgraph JSON")
        }
        "contracts" => {
            let (report, errs) = analyze(&module);
            if !errs.is_empty() {
                print_errors(&errs);
                return ExitCode::from(1);
            }
            print_json_result(
                emit_contracts_json(&module, &report),
                "contracts bundle JSON",
            )
        }
        _ => {
            eprintln!("unsupported emit target '{}'", kind);
            print_usage();
            ExitCode::from(2)
        }
    }
}

fn run_report(metrics_csv: &str, path: &str) -> ExitCode {
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

fn selftest_fixture_suites() -> Result<(), String> {
    let must_pass = collect_kr_files(Path::new("tests").join("must_pass"))?;
    for file in must_pass {
        if let Err(errs) = check_file(&file) {
            return Err(format!(
                "selftest must_pass failed for '{}': {:?}",
                file.display(),
                errs
            ));
        }
    }

    let must_fail = collect_kr_files(Path::new("tests").join("must_fail"))?;
    for file in must_fail {
        match check_file(&file) {
            Ok(()) => {
                return Err(format!(
                    "selftest must_fail expected error for '{}', got success",
                    file.display()
                ));
            }
            Err(errs) if errs.is_empty() => {
                return Err(format!(
                    "selftest must_fail expected non-empty errors for '{}'",
                    file.display()
                ));
            }
            Err(_) => {}
        }
    }

    Ok(())
}

fn selftest_exact_diagnostics() -> Result<(), String> {
    let exact_cases: [(&str, &[&str]); 4] = [
        (
            "tests/must_fail/extern_missing_eff.kr",
            &["extern 'sleep' must declare @eff(...) facts explicitly"],
        ),
        (
            "tests/must_fail/release_mismatch_nested.kr",
            &[
                "lockgraph: function 'nested_release_mismatch' release mismatch: expected 'SchedLock' on top, found 'ConsoleLock'",
            ],
        ),
        (
            "tests/must_fail/yield_hidden_two_levels.kr",
            &["lockgraph: function 'outer' calls yielding callee 'mid' under lock(s): SchedLock"],
        ),
        (
            "tests/must_fail/yield_hidden_in_leaf_wrapper.kr",
            &[
                "lockgraph: function 'outer' calls yielding callee 'wrapper' under lock(s): SchedLock",
            ],
        ),
    ];

    for (fixture, expected) in exact_cases {
        let errs = match check_file(Path::new(fixture)) {
            Ok(()) => return Err(format!("selftest expected fixture '{}' to fail", fixture)),
            Err(errs) => errs,
        };

        let expected_vec = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        if errs != expected_vec {
            return Err(format!(
                "selftest exact diagnostic mismatch for '{}': got {:?}, expected {:?}",
                fixture, errs, expected_vec
            ));
        }
    }

    Ok(())
}

fn selftest_json_schemas() -> Result<(), String> {
    let callee_module =
        compile_file(Path::new("tests/must_pass/callee_acquires_lock.kr")).map_err(join_errs)?;
    let (report, errs) = analyze(&callee_module);
    if !errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for callee_acquires_lock.kr: {:?}",
            errs
        ));
    }

    let lockgraph_text = emit_lockgraph_json(&report).map_err(|e| e.to_string())?;
    let lockgraph: Value = serde_json::from_str(&lockgraph_text).map_err(|e| e.to_string())?;
    if object_keys(&lockgraph)?
        != BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()])
    {
        return Err("selftest lockgraph top-level keys mismatch".to_string());
    }
    let edges = lockgraph["edges"]
        .as_array()
        .ok_or_else(|| "selftest lockgraph edges must be array".to_string())?;
    if edges.len() != 1 {
        return Err("selftest lockgraph expected exactly one edge".to_string());
    }
    let edge_obj = edges[0]
        .as_object()
        .ok_or_else(|| "selftest lockgraph edge must be object".to_string())?;
    let edge_keys = edge_obj.keys().cloned().collect::<BTreeSet<_>>();
    if edge_keys != BTreeSet::from(["from".to_string(), "to".to_string()]) {
        return Err("selftest lockgraph edge keys mismatch".to_string());
    }
    if edge_obj.get("from").and_then(|v| v.as_str()).is_none()
        || edge_obj.get("to").and_then(|v| v.as_str()).is_none()
    {
        return Err("selftest lockgraph edge from/to must be strings".to_string());
    }

    let locks_module = compile_file(Path::new("tests/must_pass/locks_ok.kr")).map_err(join_errs)?;
    let (locks_report, locks_errs) = analyze(&locks_module);
    if !locks_errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for locks_ok.kr: {:?}",
            locks_errs
        ));
    }
    let report_text = emit_report_json(
        &locks_report,
        &["max_lock_depth".to_string(), "no_yield_spans".to_string()],
    )
    .map_err(|e| e.to_string())?;
    let report_json: Value = serde_json::from_str(&report_text).map_err(|e| e.to_string())?;
    if object_keys(&report_json)?
        != BTreeSet::from(["max_lock_depth".to_string(), "no_yield_spans".to_string()])
    {
        return Err("selftest report keys mismatch".to_string());
    }

    let basic_module = compile_file(Path::new("tests/must_pass/basic.kr")).map_err(join_errs)?;
    let caps_text = emit_caps_manifest_json(&basic_module).map_err(|e| e.to_string())?;
    let caps_json: Value = serde_json::from_str(&caps_text).map_err(|e| e.to_string())?;
    if object_keys(&caps_json)?
        != BTreeSet::from(["module_caps".to_string(), "symbols".to_string()])
    {
        return Err("selftest caps manifest keys mismatch".to_string());
    }

    let contracts_text =
        emit_contracts_json(&locks_module, &locks_report).map_err(|e| e.to_string())?;
    let contracts_json: Value = serde_json::from_str(&contracts_text).map_err(|e| e.to_string())?;
    if object_keys(&contracts_json)?
        != BTreeSet::from([
            "capabilities".to_string(),
            "facts".to_string(),
            "lockgraph".to_string(),
            "report".to_string(),
            "schema_version".to_string(),
        ])
    {
        return Err("selftest contracts keys mismatch".to_string());
    }
    if contracts_json["schema_version"].as_str() != Some("kernrift_contracts_v1") {
        return Err("selftest contracts schema_version mismatch".to_string());
    }
    if object_keys(&contracts_json["facts"])? != BTreeSet::from(["symbols".to_string()]) {
        return Err("selftest contracts facts keys mismatch".to_string());
    }
    let fact_symbols = contracts_json["facts"]["symbols"]
        .as_array()
        .ok_or_else(|| "selftest contracts facts symbols must be array".to_string())?;
    let first_fact = fact_symbols
        .first()
        .ok_or_else(|| "selftest contracts facts symbols must not be empty".to_string())?;
    if object_keys(first_fact)?
        != BTreeSet::from([
            "attrs".to_string(),
            "caps_req".to_string(),
            "ctx_ok".to_string(),
            "eff_used".to_string(),
            "is_extern".to_string(),
            "name".to_string(),
        ])
    {
        return Err("selftest contracts fact symbol keys mismatch".to_string());
    }
    if object_keys(&first_fact["attrs"])?
        != BTreeSet::from([
            "hotpath".to_string(),
            "leaf".to_string(),
            "lock_budget".to_string(),
            "noyield".to_string(),
        ])
    {
        return Err("selftest contracts attrs keys mismatch".to_string());
    }
    validate_json_against_schema_text(
        &contracts_json,
        CONTRACTS_SCHEMA_V1,
        "embedded contracts schema",
        "contracts",
    )?;

    let contracts_v2_text =
        emit_contracts_json_with_schema(&locks_module, &locks_report, EmitContractsSchema::V2)
            .map_err(|e| e.to_string())?;
    let contracts_v2_json: Value =
        serde_json::from_str(&contracts_v2_text).map_err(|e| e.to_string())?;
    if contracts_v2_json["schema_version"].as_str() != Some(CONTRACTS_SCHEMA_VERSION_V2) {
        return Err("selftest contracts v2 schema_version mismatch".to_string());
    }
    if object_keys(&contracts_v2_json["report"])?
        != BTreeSet::from([
            "contexts".to_string(),
            "effects".to_string(),
            "max_lock_depth".to_string(),
            "no_yield_spans".to_string(),
        ])
    {
        return Err("selftest contracts v2 report keys mismatch".to_string());
    }
    validate_json_against_schema_text(
        &contracts_v2_json,
        CONTRACTS_SCHEMA_V2,
        "embedded contracts schema v2",
        "contracts",
    )?;

    let krir_text = emit_krir_json(&basic_module).map_err(|e| e.to_string())?;
    let krir_json: Value = serde_json::from_str(&krir_text).map_err(|e| e.to_string())?;
    if object_keys(&krir_json)?
        != BTreeSet::from([
            "call_edges".to_string(),
            "functions".to_string(),
            "module_caps".to_string(),
        ])
    {
        return Err("selftest krir keys mismatch".to_string());
    }

    Ok(())
}

fn selftest_policy_engine() -> Result<(), String> {
    let module =
        compile_file(Path::new("tests/must_pass/callee_acquires_lock.kr")).map_err(join_errs)?;
    let (report, errs) = analyze(&module);
    if !errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for policy fixture callee_acquires_lock.kr: {:?}",
            errs
        ));
    }
    let contracts_text =
        emit_contracts_json_canonical(&module, &report).map_err(|e| e.to_string())?;
    let contracts_json: Value = serde_json::from_str(&contracts_text).map_err(|e| e.to_string())?;
    validate_json_against_schema_text(
        &contracts_json,
        CONTRACTS_SCHEMA_V1,
        "embedded contracts schema",
        "contracts",
    )?;
    let contracts: ContractsBundle = serde_json::from_value(contracts_json)
        .map_err(|e| format!("selftest failed to decode contracts bundle: {}", e))?;

    let pass_policy = parse_policy_text(
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]
"#,
        "selftest-pass-policy",
    )?;
    let pass_errs = evaluate_policy(&pass_policy, &contracts);
    if !pass_errs.is_empty() {
        return Err(format!(
            "selftest policy pass case should have no errors, got {:?}",
            pass_errs
        ));
    }

    let fail_policy = parse_policy_text(
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
        "selftest-fail-policy",
    )?;
    let fail_errs = evaluate_policy(&fail_policy, &contracts)
        .iter()
        .map(format_policy_violation)
        .collect::<Vec<_>>();
    let expected = vec![
        "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1".to_string(),
        "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present"
            .to_string(),
    ];
    if fail_errs != expected {
        return Err(format!(
            "selftest policy deterministic error mismatch: got {:?}, expected {:?}",
            fail_errs, expected
        ));
    }

    let no_yield_policy = parse_policy_text(
        r#"
[limits]
forbid_unbounded_no_yield = true
"#,
        "selftest-no-yield-policy",
    )?;
    let no_yield_errs = evaluate_policy(&no_yield_policy, &contracts)
        .iter()
        .map(format_policy_violation)
        .collect::<Vec<_>>();
    let expected_no_yield = vec![
        "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'inner' is unbounded".to_string(),
        "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'outer' is unbounded".to_string(),
    ];
    if no_yield_errs != expected_no_yield {
        return Err(format!(
            "selftest policy no_yield mismatch: got {:?}, expected {:?}",
            no_yield_errs, expected_no_yield
        ));
    }

    Ok(())
}

fn collect_kr_files(dir: impl AsRef<Path>) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = fs::read_dir(dir.as_ref())
        .map_err(|e| format!("failed to read '{}': {}", dir.as_ref().display(), e))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "failed to read entry in '{}': {}",
                dir.as_ref().display(),
                e
            )
        })?
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("kr"))
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}

fn object_keys(value: &Value) -> Result<BTreeSet<String>, String> {
    Ok(value
        .as_object()
        .ok_or_else(|| "expected json object".to_string())?
        .keys()
        .cloned()
        .collect())
}

fn join_errs(errs: Vec<String>) -> String {
    errs.join("; ")
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

fn load_policy_file(policy_path: &str) -> Result<PolicyFile, String> {
    let policy_text = fs::read_to_string(Path::new(policy_path))
        .map_err(|e| format!("failed to read policy '{}': {}", policy_path, e))?;
    parse_policy_text(&policy_text, policy_path)
}

fn load_profile_policy(profile: CheckProfile) -> Result<PolicyFile, String> {
    match profile {
        CheckProfile::Kernel => {
            parse_policy_text(KERNEL_POLICY_PROFILE, "<embedded-kernel-profile>")
        }
    }
}

fn parse_policy_text(text: &str, source_name: &str) -> Result<PolicyFile, String> {
    let mut policy: PolicyFile = toml::from_str(text)
        .map_err(|e| format!("failed to parse policy '{}': {}", source_name, e))?;

    for edge in &mut policy.locks.forbid_edges {
        edge[0] = edge[0].trim().to_string();
        edge[1] = edge[1].trim().to_string();
        if edge[0].is_empty() || edge[1].is_empty() {
            return Err(format!(
                "invalid policy '{}': forbid_edges entries must contain non-empty lock class names",
                source_name
            ));
        }
    }
    policy.locks.forbid_edges.sort();
    policy.locks.forbid_edges.dedup();

    for cap in &mut policy.caps.allow_module {
        *cap = cap.trim().to_string();
        if cap.is_empty() {
            return Err(format!(
                "invalid policy '{}': allow_module entries must be non-empty strings",
                source_name
            ));
        }
    }
    policy.caps.allow_module.sort();
    policy.caps.allow_module.dedup();

    Ok(policy)
}

fn decode_contracts_bundle(
    contracts_text: &str,
    source_name: &str,
) -> Result<ContractsBundle, String> {
    let contracts_json: Value = serde_json::from_str(contracts_text)
        .map_err(|e| format!("failed to parse contracts JSON '{}': {}", source_name, e))?;
    let schema_version = contracts_json
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "failed to decode contracts bundle '{}': missing string field 'schema_version'",
                source_name
            )
        })?;
    let (schema_text, schema_name) = match schema_version {
        CONTRACTS_SCHEMA_VERSION => (CONTRACTS_SCHEMA_V1, "embedded contracts schema v1"),
        CONTRACTS_SCHEMA_VERSION_V2 => (CONTRACTS_SCHEMA_V2, "embedded contracts schema v2"),
        other => {
            return Err(format!(
                "unsupported contracts schema_version '{}', expected '{}' or '{}'",
                other, CONTRACTS_SCHEMA_VERSION, CONTRACTS_SCHEMA_VERSION_V2
            ));
        }
    };
    validate_json_against_schema_text(&contracts_json, schema_text, schema_name, "contracts")?;

    let contracts: ContractsBundle = serde_json::from_value(contracts_json).map_err(|e| {
        format!(
            "failed to decode contracts bundle '{}' into policy model: {}",
            source_name, e
        )
    })?;
    if contracts.schema_version != CONTRACTS_SCHEMA_VERSION
        && contracts.schema_version != CONTRACTS_SCHEMA_VERSION_V2
    {
        return Err(format!(
            "unsupported contracts schema_version '{}', expected '{}' or '{}'",
            contracts.schema_version, CONTRACTS_SCHEMA_VERSION, CONTRACTS_SCHEMA_VERSION_V2
        ));
    }

    Ok(contracts)
}

fn evaluate_policy(policy: &PolicyFile, contracts: &ContractsBundle) -> Vec<PolicyViolation> {
    let mut violations = Vec::<PolicyViolation>::new();
    let symbol_by_name = contracts
        .facts
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol))
        .collect::<BTreeMap<_, _>>();

    // Parsed for deterministic v2 policy/report compatibility even when
    // current rule-set only consumes a subset of effect counters.
    let _effect_counters = (
        contracts.report.effects.yield_sites_count,
        contracts.report.effects.alloc_sites_count,
        contracts.report.effects.block_sites_count,
    );

    if let Some(limit) = policy.limits.max_lock_depth
        && contracts.report.max_lock_depth > limit
    {
        violations.push(PolicyViolation {
            code: "LIMIT_MAX_LOCK_DEPTH",
            msg: format!(
                "max_lock_depth {} exceeds limit {}",
                contracts.report.max_lock_depth, limit
            ),
        });
    }

    if let Some(limit) = policy.limits.max_no_yield_span {
        for (symbol, span) in &contracts.report.no_yield_spans {
            match span {
                ContractsNoYieldSpan::Bounded(v) if *v > limit => {
                    violations.push(PolicyViolation {
                        code: "NO_YIELD_SPAN_LIMIT",
                        msg: format!(
                            "no_yield_spans '{}' has span {} above limit {}",
                            symbol, v, limit
                        ),
                    });
                }
                ContractsNoYieldSpan::Unbounded(v) if v == "unbounded" => {
                    violations.push(PolicyViolation {
                        code: "NO_YIELD_UNBOUNDED",
                        msg: format!(
                            "no_yield_spans '{}' is unbounded and violates max_no_yield_span {}",
                            symbol, limit
                        ),
                    });
                }
                _ => {}
            }
        }
    }

    if policy.limits.forbid_unbounded_no_yield {
        for (symbol, span) in &contracts.report.no_yield_spans {
            if span.is_unbounded() {
                violations.push(PolicyViolation {
                    code: "NO_YIELD_UNBOUNDED",
                    msg: format!("no_yield_spans '{}' is unbounded", symbol),
                });
            }
        }
    }

    let observed_edges = contracts
        .lockgraph
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect::<BTreeSet<_>>();
    for edge in &policy.locks.forbid_edges {
        if observed_edges.contains(&(edge[0].as_str(), edge[1].as_str())) {
            violations.push(PolicyViolation {
                code: "LOCK_FORBID_EDGE",
                msg: format!(
                    "forbidden lock edge '{} -> {}' is present",
                    edge[0], edge[1]
                ),
            });
        }
    }

    if !policy.caps.allow_module.is_empty() {
        let allowed_caps = policy
            .caps
            .allow_module
            .iter()
            .map(|c| c.as_str())
            .collect::<BTreeSet<_>>();
        let mut disallowed = contracts
            .capabilities
            .module_caps
            .iter()
            .filter(|cap| !allowed_caps.contains(cap.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        disallowed.sort();
        disallowed.dedup();

        for cap in disallowed {
            violations.push(PolicyViolation {
                code: "CAP_MODULE_ALLOWLIST",
                msg: format!("module capability '{}' is not in allow_module", cap),
            });
        }
    }

    let mut irq_functions = contracts.report.contexts.irq_functions.clone();
    if irq_functions.is_empty() {
        irq_functions = contracts
            .facts
            .symbols
            .iter()
            .filter(|symbol| symbol.has_ctx("irq"))
            .map(|symbol| symbol.name.clone())
            .collect::<Vec<_>>();
    }
    irq_functions.sort();
    irq_functions.dedup();

    if policy.kernel.forbid_alloc_in_irq {
        for symbol_name in &irq_functions {
            if let Some(symbol) = symbol_by_name.get(symbol_name.as_str())
                && symbol.has_eff("alloc")
            {
                violations.push(PolicyViolation {
                    code: "KERNEL_IRQ_ALLOC",
                    msg: format!(
                        "function '{}' is irq-reachable and uses alloc effect",
                        symbol_name
                    ),
                });
            }
        }
    }

    if policy.kernel.forbid_block_in_irq {
        for symbol_name in &irq_functions {
            if let Some(symbol) = symbol_by_name.get(symbol_name.as_str())
                && symbol.has_eff("block")
            {
                violations.push(PolicyViolation {
                    code: "KERNEL_IRQ_BLOCK",
                    msg: format!(
                        "function '{}' is irq-reachable and uses block effect",
                        symbol_name
                    ),
                });
            }
        }
    }

    if policy.kernel.forbid_yield_in_critical {
        let mut critical = contracts.report.contexts.critical_functions.clone();
        if critical.is_empty() {
            critical = contracts
                .facts
                .symbols
                .iter()
                .filter(|symbol| symbol.attrs.noyield)
                .map(|symbol| symbol.name.clone())
                .collect::<Vec<_>>();
        }
        critical.sort();
        critical.dedup();
        for symbol in critical {
            if let Some(ContractsNoYieldSpan::Bounded(span)) =
                contracts.report.no_yield_spans.get(&symbol)
            {
                violations.push(PolicyViolation {
                    code: "KERNEL_CRITICAL_YIELD",
                    msg: format!(
                        "critical function '{}' may yield (max span {})",
                        symbol, span
                    ),
                });
            }
        }
    }

    violations.sort();
    violations.dedup();
    violations
}

fn format_policy_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn print_policy_violations(violations: &[PolicyViolation]) {
    for violation in violations {
        eprintln!("{}", format_policy_violation(violation));
    }
}

fn validate_json_against_schema_text(
    instance: &Value,
    schema_text: &str,
    schema_name: &str,
    label: &str,
) -> Result<(), String> {
    let schema_json: Value = serde_json::from_str(schema_text)
        .map_err(|e| format!("failed to parse schema '{}': {}", schema_name, e))?;
    let compiled = JSONSchema::compile(&schema_json)
        .map_err(|e| format!("failed to compile schema '{}': {}", schema_name, e))?;

    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        return Err(format!(
            "{} schema validation failed: {}",
            label,
            details.join(" | ")
        ));
    }

    Ok(())
}

fn print_json_result<E: std::fmt::Display>(result: Result<String, E>, label: &str) -> ExitCode {
    match result {
        Ok(text) => {
            println!("{}", text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to serialize {}: {}", label, err);
            ExitCode::from(1)
        }
    }
}

fn print_errors(errs: &[String]) {
    for err in errs {
        eprintln!("{}", err);
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  kernriftc --version");
    eprintln!("  kernriftc check <file.kr>");
    eprintln!("  kernriftc check --profile kernel <file.kr>");
    eprintln!("  kernriftc check --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --profile kernel --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --policy <policy.toml> <file.kr>");
    eprintln!("  kernriftc check --contracts-out <contracts.json> <file.kr>");
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>"
    );
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> <file.kr>"
    );
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> --sign-ed25519 <secret.hex> --sig-out <contracts.sig> <file.kr>"
    );
    eprintln!("  kernriftc policy --policy <policy.toml> --contracts <contracts.json>");
    eprintln!("  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256>");
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --sig <contracts.sig> --pubkey <pubkey.hex>"
    );
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --report <verify-report.json>"
    );
    eprintln!("  kernriftc --selftest");
    eprintln!("  kernriftc --emit krir <file.kr>");
    eprintln!("  kernriftc --emit lockgraph <file.kr>");
    eprintln!("  kernriftc --emit caps <file.kr>");
    eprintln!("  kernriftc --emit contracts <file.kr>");
    eprintln!("  kernriftc --emit report --metrics max_lock_depth,no_yield_spans <file.kr>");
    eprintln!("  kernriftc --report max_lock_depth,no_yield_spans <file.kr>");
}
