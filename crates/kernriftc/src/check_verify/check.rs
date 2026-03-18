use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use ed25519_dalek::Signer;
use emit::{ContractsSchema as EmitContractsSchema, emit_contracts_json_canonical_with_schema};
use kernriftc::{
    FrontendCanonicalFinding, SurfaceProfile, analyze, canonical_check_file_with_surface,
    check_file_with_surface, check_module, compile_file_with_surface,
};
use serde::Serialize;

use super::args::{CheckArgs, CheckProfile, ContractsSchemaArg, PolicyOutputFormat};
use super::crypto::{load_signing_key_hex, sha256_hex};
use super::output::write_output_files;
use crate::policy_engine::{
    decode_contracts_bundle, emit_policy_violations_json, evaluate_policy, load_policy_file,
    materialize_kernel_profile_policy, print_policy_violations,
};
use crate::{EXIT_INVALID_INPUT, EXIT_POLICY_VIOLATION, print_errors};

pub(crate) fn run_check(args: &CheckArgs) -> ExitCode {
    if args.canonical {
        return run_canonical_check(args);
    }

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
    let mut policy_violations = Vec::new();

    if let Some(profile) = args.profile {
        let profile_policy = match profile {
            CheckProfile::Kernel => materialize_kernel_profile_policy(),
        };
        let profile_policy = match profile_policy {
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
        match args.format {
            PolicyOutputFormat::Text => print_policy_violations(&policy_violations, false),
            PolicyOutputFormat::Json => {
                match emit_policy_violations_json(&policy_violations, EXIT_POLICY_VIOLATION) {
                    Ok(text) => print!("{}", text),
                    Err(err) => {
                        eprintln!("failed to serialize policy JSON: {}", err);
                        return ExitCode::from(EXIT_INVALID_INPUT);
                    }
                }
            }
        }
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
        signature_b64 = Some(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()));
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

fn run_canonical_check(args: &CheckArgs) -> ExitCode {
    let path = Path::new(&args.path);
    if let Err(errs) = check_file_with_surface(path, args.surface) {
        print_errors(&errs);
        return ExitCode::from(EXIT_POLICY_VIOLATION);
    }

    let findings = match canonical_check_file_with_surface(path, args.surface) {
        Ok(findings) => findings,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(EXIT_POLICY_VIOLATION);
        }
    };

    let finding_count = findings.len();
    match args.format {
        PolicyOutputFormat::Text => print_canonical_findings_text(args.surface, &findings),
        PolicyOutputFormat::Json => match emit_canonical_findings_json(args.surface, &findings) {
            Ok(text) => print!("{}", text),
            Err(err) => {
                eprintln!("failed to serialize canonical findings JSON: {}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        },
    }

    if finding_count == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_POLICY_VIOLATION)
    }
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalFindingsJsonReport<'a> {
    schema_version: &'static str,
    surface: &'static str,
    canonical_findings: usize,
    findings: Vec<CanonicalFindingJson<'a>>,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalFindingJson<'a> {
    function: &'a str,
    classification: &'a str,
    surface_form: String,
    canonical_replacement: &'a str,
    migration_safe: bool,
}

const CANONICAL_FINDINGS_SCHEMA_VERSION: &str = "kernrift_canonical_findings_v1";

fn print_canonical_findings_text(surface: SurfaceProfile, findings: &[FrontendCanonicalFinding]) {
    println!("surface: {}", surface.as_str());
    println!("canonical_findings: {}", findings.len());
    for finding in findings {
        println!("function: {}", finding.function_name);
        println!("classification: {}", finding.classification.as_str());
        println!("surface_form: @{}", finding.surface_form);
        println!("canonical_replacement: {}", finding.canonical_replacement);
        println!("migration_safe: {}", finding.migration_safe);
    }
}

fn emit_canonical_findings_json(
    surface: SurfaceProfile,
    findings: &[FrontendCanonicalFinding],
) -> Result<String, serde_json::Error> {
    let report = CanonicalFindingsJsonReport {
        schema_version: CANONICAL_FINDINGS_SCHEMA_VERSION,
        surface: surface.as_str(),
        canonical_findings: findings.len(),
        findings: findings
            .iter()
            .map(|finding| CanonicalFindingJson {
                function: &finding.function_name,
                classification: finding.classification.as_str(),
                surface_form: format!("@{}", finding.surface_form),
                canonical_replacement: finding.canonical_replacement,
                migration_safe: finding.migration_safe,
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
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
