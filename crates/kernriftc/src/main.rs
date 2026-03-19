use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use emit::{
    ContractsSchema as EmitContractsSchema, emit_caps_manifest_json, emit_contracts_json,
    emit_contracts_json_canonical, emit_contracts_json_with_schema, emit_krir_json,
    emit_lockgraph_json, emit_report_json,
};
use kernriftc::{
    CanonicalFixPreviewResult, CanonicalFixResult, CanonicalFixSourceResult, SurfaceProfile,
    analyze, canonical_edit_plan_source_with_surface, canonical_fix_file_with_surface,
    canonical_fix_preview_source_with_surface, canonical_fix_source_text_with_surface, check_file,
    compile_file, frontend_migration_features_for_profile, migrate_preview_file_with_surface,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod artifact_inspect;
mod artifact_meta;
mod backend_emit;
mod canonical_input;
mod canonical_text;
mod check_verify;
mod policy_engine;
mod proposals;
mod verify_artifact_meta;

use crate::artifact_inspect::{
    format_artifact_inspection_report_text, inspect_artifact_from_bytes,
};
use crate::backend_emit::{parse_backend_emit_args, run_backend_emit};
use crate::canonical_input::CanonicalInput;
use crate::canonical_text::{
    print_edit_entry, print_file_label, print_rewrite_entry, print_surface_and_count,
};
use crate::check_verify::{
    parse_check_args, parse_inspect_args, parse_inspect_report_args, parse_policy_args,
    parse_verify_args, run_check, run_inspect, run_inspect_report, run_policy, run_report,
    run_verify,
};
use crate::policy_engine::{
    CONTRACTS_SCHEMA_V1, CONTRACTS_SCHEMA_V2, CONTRACTS_SCHEMA_VERSION_V2, ContractsBundle,
    evaluate_policy, format_policy_violation, parse_policy_text, validate_json_against_schema_text,
};
use crate::proposals::{parse_proposals_args, run_proposals};
use crate::verify_artifact_meta::{parse_verify_artifact_meta_args, run_verify_artifact_meta};

const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");
const VERIFY_REPORT_SCHEMA_VERSION: &str = "kernrift_verify_report_v1";
const EXIT_POLICY_VIOLATION: u8 = 1;
const EXIT_INVALID_INPUT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectArtifactFormat {
    Text,
    Json,
}

impl InspectArtifactFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid inspect-artifact mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct InspectArtifactArgs {
    artifact_path: String,
    format: InspectArtifactFormat,
}

struct FeaturesArgs {
    surface: SurfaceProfile,
}

#[derive(Debug)]
struct MigratePreviewArgs {
    surface: SurfaceProfile,
    canonical_edits: bool,
    stdin: bool,
    format: MigratePreviewFormat,
    input_path: Option<String>,
}

#[derive(Debug)]
struct FixArgs {
    surface: SurfaceProfile,
    canonical: bool,
    write: bool,
    dry_run: bool,
    stdout: bool,
    diff: bool,
    stdin: bool,
    format: FixFormat,
    input_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigratePreviewFormat {
    Text,
    Json,
}

impl MigratePreviewFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid migrate-preview mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixFormat {
    Text,
    Json,
}

impl FixFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid fix mode: unsupported --format '{}' (expected 'text' or 'json')",
                other
            )),
        }
    }
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
        "--surface" => {
            if args.len() < 4 {
                eprintln!("invalid emit mode: --surface requires a value");
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            let surface = match SurfaceProfile::parse(&args[2]) {
                Ok(surface) => surface,
                Err(err) => {
                    eprintln!("invalid emit mode: {}", err);
                    print_usage();
                    return ExitCode::from(EXIT_INVALID_INPUT);
                }
            };
            let emit_arg = &args[3];
            if !emit_arg.starts_with("--emit=") {
                eprintln!("invalid emit mode: --surface must be followed by --emit=<target>");
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            match parse_backend_emit_args(&emit_arg["--emit=".len()..], &args[4..], surface) {
                Ok(parsed) => run_backend_emit(&parsed),
                Err(err) => {
                    eprintln!("{}", err);
                    print_usage();
                    ExitCode::from(EXIT_INVALID_INPUT)
                }
            }
        }
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
        "verify-artifact-meta" => match parse_verify_artifact_meta_args(&args[2..]) {
            Ok(parsed) => run_verify_artifact_meta(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "policy" => match parse_policy_args(&args[2..]) {
            Ok(parsed) => run_policy(
                &parsed.policy_path,
                &parsed.contracts_path,
                parsed.evidence,
                parsed.format,
            ),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "inspect" => match parse_inspect_args(&args[2..]) {
            Ok(parsed) => run_inspect(&parsed.contracts_path),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "inspect-report" => match parse_inspect_report_args(&args[2..]) {
            Ok(parsed) => run_inspect_report(&parsed.report_path),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "inspect-artifact" => match parse_inspect_artifact_args(&args[2..]) {
            Ok(parsed) => run_inspect_artifact(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "features" => match parse_features_args(&args[2..]) {
            Ok(parsed) => run_features(parsed.surface),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "proposals" => match parse_proposals_args(&args[2..]) {
            Ok(parsed) => run_proposals(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "migrate-preview" => match parse_migrate_preview_args(&args[2..]) {
            Ok(parsed) => run_migrate_preview(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "fix" => match parse_fix_args(&args[2..]) {
            Ok(parsed) => run_fix(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "--selftest" => {
            if args.len() != 2 {
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            run_selftest()
        }
        arg if arg.starts_with("--emit=") => {
            match parse_backend_emit_args(
                &arg["--emit=".len()..],
                &args[2..],
                SurfaceProfile::Stable,
            ) {
                Ok(parsed) => run_backend_emit(&parsed),
                Err(err) => {
                    eprintln!("{}", err);
                    print_usage();
                    ExitCode::from(EXIT_INVALID_INPUT)
                }
            }
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

fn parse_inspect_artifact_args(args: &[String]) -> Result<InspectArtifactArgs, String> {
    let mut format = InspectArtifactFormat::Text;
    let mut format_set = false;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format" => {
                if format_set {
                    return Err("invalid inspect-artifact mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect-artifact mode: --format requires 'text' or 'json'"
                            .to_string(),
                    );
                }
                format = InspectArtifactFormat::parse(&args[idx])?;
                format_set = true;
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "invalid inspect-artifact mode: unexpected argument '{}'",
                    other
                ));
            }
            other => {
                positionals.push(other.to_string());
            }
        }
        idx += 1;
    }

    if positionals.len() != 1 {
        return Err(
            "invalid inspect-artifact mode: expected exactly one <artifact-path>".to_string(),
        );
    }

    Ok(InspectArtifactArgs {
        artifact_path: positionals.remove(0),
        format,
    })
}

fn parse_features_args(args: &[String]) -> Result<FeaturesArgs, String> {
    let mut surface = None::<SurfaceProfile>;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--surface" => {
                if surface.is_some() {
                    return Err("invalid features mode: duplicate --surface".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid features mode: --surface requires a value".to_string());
                }
                surface = Some(
                    SurfaceProfile::parse(&args[idx])
                        .map_err(|err| format!("invalid features mode: {}", err))?,
                );
            }
            other => {
                return Err(format!(
                    "invalid features mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    let Some(surface) = surface else {
        return Err("invalid features mode: missing --surface".to_string());
    };

    Ok(FeaturesArgs { surface })
}

fn parse_migrate_preview_args(args: &[String]) -> Result<MigratePreviewArgs, String> {
    let mut surface = None::<SurfaceProfile>;
    let mut canonical_edits = false;
    let mut stdin = false;
    let mut format = MigratePreviewFormat::Text;
    let mut format_set = false;
    let mut input_path = None::<String>;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--surface" => {
                if surface.is_some() {
                    return Err("invalid migrate-preview mode: duplicate --surface".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid migrate-preview mode: --surface requires a value".to_string()
                    );
                }
                surface = Some(
                    SurfaceProfile::parse(&args[idx])
                        .map_err(|err| format!("invalid migrate-preview mode: {}", err))?,
                );
            }
            "--canonical-edits" => {
                if canonical_edits {
                    return Err(
                        "invalid migrate-preview mode: duplicate --canonical-edits".to_string()
                    );
                }
                canonical_edits = true;
            }
            "--stdin" => {
                if stdin {
                    return Err("invalid migrate-preview mode: duplicate --stdin".to_string());
                }
                stdin = true;
            }
            "--format" => {
                if format_set {
                    return Err("invalid migrate-preview mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid migrate-preview mode: --format requires 'text' or 'json'"
                            .to_string(),
                    );
                }
                format = MigratePreviewFormat::parse(&args[idx])?;
                format_set = true;
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "invalid migrate-preview mode: unexpected argument '{}'",
                    other
                ));
            }
            other => {
                if input_path.is_some() {
                    return Err(format!(
                        "invalid migrate-preview mode: unexpected argument '{}'",
                        other
                    ));
                }
                input_path = Some(other.to_string());
            }
        }
        idx += 1;
    }

    let surface = if let Some(surface) = surface {
        surface
    } else if canonical_edits {
        SurfaceProfile::Stable
    } else {
        return Err("invalid migrate-preview mode: missing --surface".to_string());
    };
    if stdin && input_path.is_some() {
        return Err(
            "invalid migrate-preview mode: --stdin cannot be combined with an input file"
                .to_string(),
        );
    }
    if stdin && !canonical_edits {
        return Err(
            "invalid migrate-preview mode: --stdin is only supported with --canonical-edits"
                .to_string(),
        );
    }
    if !stdin && input_path.is_none() {
        return Err("invalid migrate-preview mode: missing input file".to_string());
    }

    if !canonical_edits && format_set {
        return Err(
            "invalid migrate-preview mode: --format is only supported with --canonical-edits"
                .to_string(),
        );
    }

    if canonical_edits && !stdin && !format_set {
        return Err(
            "invalid migrate-preview mode: --canonical-edits requires --format json".to_string(),
        );
    }

    Ok(MigratePreviewArgs {
        surface,
        canonical_edits,
        stdin,
        format,
        input_path,
    })
}

fn parse_fix_args(args: &[String]) -> Result<FixArgs, String> {
    let mut surface = SurfaceProfile::Stable;
    let mut surface_set = false;
    let mut canonical = false;
    let mut write = false;
    let mut dry_run = false;
    let mut stdout = false;
    let mut diff = false;
    let mut stdin = false;
    let mut format = FixFormat::Text;
    let mut format_set = false;
    let mut input_path = None::<String>;
    let mut idx = 0usize;

    while idx < args.len() {
        match args[idx].as_str() {
            "--surface" => {
                if surface_set {
                    return Err("invalid fix mode: duplicate --surface".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid fix mode: --surface requires a value".to_string());
                }
                surface = SurfaceProfile::parse(&args[idx])
                    .map_err(|err| format!("invalid fix mode: {}", err))?;
                surface_set = true;
            }
            "--format" => {
                if format_set {
                    return Err("invalid fix mode: duplicate --format".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid fix mode: --format requires 'text' or 'json'".to_string());
                }
                format = FixFormat::parse(&args[idx])?;
                format_set = true;
            }
            "--canonical" => {
                if canonical {
                    return Err("invalid fix mode: duplicate --canonical".to_string());
                }
                canonical = true;
            }
            "--write" => {
                if write {
                    return Err("invalid fix mode: duplicate --write".to_string());
                }
                write = true;
            }
            "--dry-run" => {
                if dry_run {
                    return Err("invalid fix mode: duplicate --dry-run".to_string());
                }
                dry_run = true;
            }
            "--stdout" => {
                if stdout {
                    return Err("invalid fix mode: duplicate --stdout".to_string());
                }
                stdout = true;
            }
            "--diff" => {
                if diff {
                    return Err("invalid fix mode: duplicate --diff".to_string());
                }
                diff = true;
            }
            "--stdin" => {
                if stdin {
                    return Err("invalid fix mode: duplicate --stdin".to_string());
                }
                stdin = true;
            }
            other if other.starts_with('-') => {
                return Err(format!("invalid fix mode: unexpected argument '{}'", other));
            }
            other => {
                if input_path.is_some() {
                    return Err(format!("invalid fix mode: unexpected argument '{}'", other));
                }
                input_path = Some(other.to_string());
            }
        }
        idx += 1;
    }

    if !canonical {
        return Err("invalid fix mode: missing --canonical".to_string());
    }
    if write && dry_run {
        return Err(
            "invalid fix mode: exactly one of --write or --dry-run must be specified".to_string(),
        );
    }
    if stdout && write {
        return Err("invalid fix mode: --stdout cannot be combined with --write".to_string());
    }
    if stdout && dry_run {
        return Err("invalid fix mode: --stdout cannot be combined with --dry-run".to_string());
    }
    if diff && write {
        return Err("invalid fix mode: --diff cannot be combined with --write".to_string());
    }
    if diff && dry_run {
        return Err("invalid fix mode: --diff cannot be combined with --dry-run".to_string());
    }
    if diff && stdout {
        return Err("invalid fix mode: --diff cannot be combined with --stdout".to_string());
    }
    if stdin && write {
        return Err("invalid fix mode: --stdin cannot be combined with --write".to_string());
    }
    if !write && !dry_run && !stdout && !diff {
        return Err(
            "invalid fix mode: exactly one of --write, --dry-run, --stdout, or --diff must be specified".to_string(),
        );
    }
    if stdout && format_set {
        return Err("invalid fix mode: --stdout does not accept --format".to_string());
    }
    if diff && format_set {
        return Err("invalid fix mode: --diff does not accept --format".to_string());
    }
    if stdin && input_path.is_some() {
        return Err("invalid fix mode: --stdin cannot be combined with an input file".to_string());
    }
    if !stdin && input_path.is_none() {
        return Err("invalid fix mode: missing input file".to_string());
    }

    Ok(FixArgs {
        surface,
        canonical,
        write,
        dry_run,
        stdout,
        diff,
        stdin,
        format,
        input_path,
    })
}

fn run_inspect_artifact(args: &InspectArtifactArgs) -> ExitCode {
    let artifact_bytes = match fs::read(Path::new(&args.artifact_path)) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("failed to read artifact '{}': {}", args.artifact_path, err);
            return ExitCode::from(1);
        }
    };
    let mut report = match inspect_artifact_from_bytes(&artifact_bytes) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(1);
        }
    };
    report.file = args.artifact_path.clone();

    match args.format {
        InspectArtifactFormat::Text => {
            println!("{}", format_artifact_inspection_report_text(&report));
            ExitCode::SUCCESS
        }
        InspectArtifactFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(mut text) => {
                text.push('\n');
                print!("{}", text);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("failed to serialize artifact inspection JSON: {}", err);
                ExitCode::from(1)
            }
        },
    }
}

fn run_features(surface: SurfaceProfile) -> ExitCode {
    let features = frontend_migration_features_for_profile(surface);
    println!("surface: {}", surface.as_str());
    println!("features: {}", features.len());
    for feature in features {
        println!("feature: {}", feature.id);
        println!("status: {}", feature.status.as_str());
        println!("classification: {}", feature.classification.as_str());
        println!("surface_form: @{}", feature.surface_form);
        println!("lowering_target: {}", feature.lowering_target);
        println!("proposal_id: {}", feature.proposal_id);
        println!("migration_safe: {}", feature.migration_safe);
        println!("canonical_replacement: {}", feature.canonical_replacement);
        println!("rewrite_intent: {}", feature.rewrite_intent);
    }
    ExitCode::SUCCESS
}

fn run_migrate_preview(args: &MigratePreviewArgs) -> ExitCode {
    if args.canonical_edits {
        return run_canonical_edit_preview(args);
    }

    let entries = match migrate_preview_file_with_surface(
        Path::new(args.input_path.as_deref().expect("input path")),
        args.surface,
    ) {
        Ok(entries) => entries,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    println!("surface: {}", args.surface.as_str());
    println!("migrations: {}", entries.len());
    for entry in entries {
        println!("function: {}", entry.function_name);
        println!("surface_form: @{}", entry.feature.surface_form);
        println!("feature: {}", entry.feature.id);
        println!("status: {}", entry.feature.status.as_str());
        println!("classification: {}", entry.feature.classification.as_str());
        println!("enabled_under_surface: {}", entry.enabled_under_surface);
        println!(
            "canonical_replacement: {}",
            entry.feature.canonical_replacement
        );
        println!("migration_safe: {}", entry.feature.migration_safe);
        println!("rewrite_intent: {}", entry.feature.rewrite_intent);
    }
    ExitCode::SUCCESS
}

#[derive(Debug, serde::Serialize)]
struct CanonicalEditPlanJsonReport<'a> {
    schema_version: &'static str,
    surface: &'static str,
    file: &'a str,
    edits_count: usize,
    edits: Vec<CanonicalEditJson<'a>>,
}

#[derive(Debug, serde::Serialize)]
struct CanonicalEditJson<'a> {
    function: &'a str,
    classification: &'a str,
    surface_form: String,
    canonical_replacement: &'a str,
    migration_safe: bool,
    rewrite_intent: &'a str,
}

const CANONICAL_EDIT_PLAN_SCHEMA_VERSION: &str = "kernrift_canonical_edit_plan_v2";
const CANONICAL_FIX_SCHEMA_VERSION: &str = "kernrift_canonical_fix_result_v1";
const CANONICAL_FIX_PREVIEW_SCHEMA_VERSION: &str = "kernrift_canonical_fix_preview_v1";

fn run_canonical_edit_preview(args: &MigratePreviewArgs) -> ExitCode {
    let input = CanonicalInput::from_optional_path(args.stdin, args.input_path.as_deref());
    let edits = match canonical_edit_plan_for_input(input, args.surface) {
        Ok(edits) => edits,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    match args.format {
        MigratePreviewFormat::Text => {
            print_surface_and_count(args.surface, "edits_count", edits.len());
            print_file_label(input.label());
            for edit in edits {
                print_edit_entry(
                    edit.function_name.as_str(),
                    edit.classification.as_str(),
                    edit.surface_form,
                    edit.canonical_replacement,
                    edit.migration_safe,
                    edit.rewrite_intent,
                );
            }
            ExitCode::SUCCESS
        }
        MigratePreviewFormat::Json => {
            match emit_canonical_edit_plan_json(args.surface, input.label(), &edits) {
                Ok(text) => {
                    print!("{}", text);
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("failed to serialize canonical edit-plan JSON: {}", err);
                    ExitCode::from(EXIT_INVALID_INPUT)
                }
            }
        }
    }
}

fn canonical_edit_plan_for_input(
    input: CanonicalInput<'_>,
    surface: SurfaceProfile,
) -> Result<Vec<kernriftc::FrontendCanonicalEditPlanEntry>, Vec<String>> {
    let src = input.read_to_string()?;
    canonical_edit_plan_source_with_surface(&src, surface)
}

fn run_fix(args: &FixArgs) -> ExitCode {
    debug_assert!(args.canonical);
    debug_assert_eq!(
        [args.write, args.dry_run, args.stdout, args.diff]
            .into_iter()
            .filter(|enabled| *enabled)
            .count(),
        1
    );

    if args.dry_run {
        return run_fix_dry_run(args);
    }
    if args.stdout {
        return run_fix_stdout(args);
    }
    if args.diff {
        return run_fix_diff(args);
    }

    let result = match canonical_fix_file_with_surface(
        Path::new(args.input_path.as_deref().expect("input path")),
        args.surface,
    ) {
        Ok(result) => result,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    match args.format {
        FixFormat::Text => {
            print_surface_and_count(args.surface, "rewrites_applied", result.rewrites.len());
            print_file_label(args.input_path.as_deref().unwrap_or("<stdin>"));
            for rewrite in result.rewrites {
                print_rewrite_entry(
                    rewrite.function_name.as_str(),
                    rewrite.surface_form,
                    rewrite.canonical_replacement,
                );
            }
            ExitCode::SUCCESS
        }
        FixFormat::Json => match emit_canonical_fix_json(
            args.surface,
            args.input_path.as_deref().expect("input path"),
            &result,
        ) {
            Ok(text) => {
                print!("{}", text);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("failed to serialize canonical fix JSON: {}", err);
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
    }
}

fn run_fix_stdout(args: &FixArgs) -> ExitCode {
    debug_assert!(args.stdout);
    let result = match canonical_fix_source_result_for_args(args) {
        Ok(result) => result,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    emit_canonical_fix_stdout(&result);
    ExitCode::SUCCESS
}

fn run_fix_diff(args: &FixArgs) -> ExitCode {
    debug_assert!(args.diff);
    let result = match canonical_fix_source_result_for_args(args) {
        Ok(result) => result,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    emit_canonical_fix_diff(&result);
    ExitCode::SUCCESS
}

fn run_fix_dry_run(args: &FixArgs) -> ExitCode {
    let result = match canonical_fix_preview_result_for_args(args) {
        Ok(result) => result,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    match args.format {
        FixFormat::Text => {
            let input = canonical_input_for_fix_args(args);
            print_surface_and_count(args.surface, "rewrites_planned", result.rewrites.len());
            print_file_label(input.label());
            for rewrite in result.rewrites {
                print_rewrite_entry(
                    rewrite.function_name.as_str(),
                    rewrite.surface_form,
                    rewrite.canonical_replacement,
                );
            }
            ExitCode::SUCCESS
        }
        FixFormat::Json => {
            let input = canonical_input_for_fix_args(args);
            match emit_canonical_fix_preview_json(args.surface, input.label(), &result) {
                Ok(text) => {
                    print!("{}", text);
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("failed to serialize canonical fix preview JSON: {}", err);
                    ExitCode::from(EXIT_INVALID_INPUT)
                }
            }
        }
    }
}

fn emit_canonical_edit_plan_json(
    surface: SurfaceProfile,
    file: &str,
    edits: &[kernriftc::FrontendCanonicalEditPlanEntry],
) -> Result<String, serde_json::Error> {
    let report = CanonicalEditPlanJsonReport {
        schema_version: CANONICAL_EDIT_PLAN_SCHEMA_VERSION,
        surface: surface.as_str(),
        file,
        edits_count: edits.len(),
        edits: edits
            .iter()
            .map(|edit| CanonicalEditJson {
                function: &edit.function_name,
                classification: edit.classification.as_str(),
                surface_form: format!("@{}", edit.surface_form),
                canonical_replacement: edit.canonical_replacement,
                migration_safe: edit.migration_safe,
                rewrite_intent: edit.rewrite_intent,
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

#[derive(Debug, serde::Serialize)]
struct CanonicalFixJsonReport<'a> {
    schema_version: &'static str,
    surface: &'static str,
    file: &'a str,
    rewrites_applied: usize,
    changed: bool,
    rewrites: Vec<CanonicalFixJson<'a>>,
}

#[derive(Debug, serde::Serialize)]
struct CanonicalFixJson<'a> {
    function: &'a str,
    classification: &'a str,
    surface_form: String,
    canonical_replacement: &'a str,
    migration_safe: bool,
}

#[derive(Debug, serde::Serialize)]
struct CanonicalFixPreviewJsonReport<'a> {
    schema_version: &'static str,
    surface: &'static str,
    file: &'a str,
    rewrites_planned: usize,
    would_change: bool,
    rewrites: Vec<CanonicalFixJson<'a>>,
}

fn emit_canonical_fix_json(
    surface: SurfaceProfile,
    file: &str,
    result: &CanonicalFixResult,
) -> Result<String, serde_json::Error> {
    let report = CanonicalFixJsonReport {
        schema_version: CANONICAL_FIX_SCHEMA_VERSION,
        surface: surface.as_str(),
        file,
        rewrites_applied: result.rewrites.len(),
        changed: result.changed,
        rewrites: result
            .rewrites
            .iter()
            .map(|rewrite| CanonicalFixJson {
                function: &rewrite.function_name,
                classification: rewrite.classification.as_str(),
                surface_form: format!("@{}", rewrite.surface_form),
                canonical_replacement: rewrite.canonical_replacement,
                migration_safe: rewrite.migration_safe,
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

fn emit_canonical_fix_preview_json(
    surface: SurfaceProfile,
    file: &str,
    result: &CanonicalFixPreviewResult,
) -> Result<String, serde_json::Error> {
    let report = CanonicalFixPreviewJsonReport {
        schema_version: CANONICAL_FIX_PREVIEW_SCHEMA_VERSION,
        surface: surface.as_str(),
        file,
        rewrites_planned: result.rewrites.len(),
        would_change: result.would_change,
        rewrites: result
            .rewrites
            .iter()
            .map(|rewrite| CanonicalFixJson {
                function: &rewrite.function_name,
                classification: rewrite.classification.as_str(),
                surface_form: format!("@{}", rewrite.surface_form),
                canonical_replacement: rewrite.canonical_replacement,
                migration_safe: rewrite.migration_safe,
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    Ok(text)
}

fn emit_canonical_fix_stdout(result: &CanonicalFixSourceResult) {
    print!("{}", result.rewritten_source);
}

fn emit_canonical_fix_diff(result: &CanonicalFixSourceResult) {
    if !result.changed {
        return;
    }

    let diff = render_full_file_unified_diff(&result.original_source, &result.rewritten_source);
    print!("{}", diff);
}

fn render_full_file_unified_diff(original_source: &str, rewritten_source: &str) -> String {
    let old_lines = split_diff_lines(original_source);
    let new_lines = split_diff_lines(rewritten_source);

    let old_count = old_lines.len();
    let new_count = new_lines.len();
    let old_start = if old_count == 0 { 0 } else { 1 };
    let new_start = if new_count == 0 { 0 } else { 1 };

    let mut out = String::new();
    out.push_str("--- original\n");
    out.push_str("+++ canonical\n");
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        old_start, old_count, new_start, new_count
    ));
    for line in old_lines {
        out.push('-');
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push('\n');
        }
    }
    for line in new_lines {
        out.push('+');
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn split_diff_lines(src: &str) -> Vec<&str> {
    if src.is_empty() {
        Vec::new()
    } else {
        src.split_inclusive('\n').collect()
    }
}

fn canonical_fix_source_result_for_args(
    args: &FixArgs,
) -> Result<CanonicalFixSourceResult, Vec<String>> {
    let input = canonical_input_for_fix_args(args);
    let src = input.read_to_string()?;
    canonical_fix_source_text_with_surface(&src, args.surface)
}

fn canonical_fix_preview_result_for_args(
    args: &FixArgs,
) -> Result<CanonicalFixPreviewResult, Vec<String>> {
    let input = canonical_input_for_fix_args(args);
    let src = input.read_to_string()?;
    canonical_fix_preview_source_with_surface(&src, args.surface)
}

fn canonical_input_for_fix_args(args: &FixArgs) -> CanonicalInput<'_> {
    CanonicalInput::from_optional_path(args.stdin, args.input_path.as_deref())
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
    let exact_cases: [(&str, &[&str]); 6] = [
        (
            "tests/must_fail/extern_missing_eff.kr",
            &[
                "extern 'sleep' must declare @eff(...) facts explicitly at 2:1\n  2 | extern @ctx(thread) @caps() fn sleep();\n  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();",
            ],
        ),
        (
            "tests/must_fail/extern_missing_caps.kr",
            &[
                "EXTERN_CAPS_CONTRACT_REQUIRED: extern 'sleep' must declare @caps(...) facts explicitly at 1:1\n  1 | extern @ctx(thread) @eff(block) fn sleep();\n  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();",
            ],
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
        (
            "tests/must_fail/legacy_yieldpoint_attr.kr",
            &[
                "legacy spelling '@yieldpoint' is non-canonical and is not accepted on function 'pump' at 1:1\n  1 | @yieldpoint\n  = help: did you mean the canonical spelling yieldpoint()? control-point markers use statement form, not attributes.",
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
            "critical".to_string(),
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
    eprintln!("  kernriftc check --canonical <file.kr>");
    eprintln!("  kernriftc check --canonical --stdin");
    eprintln!("  kernriftc check --canonical --format json <file.kr>");
    eprintln!("  kernriftc check --canonical --stdin --format json");
    eprintln!("  kernriftc check --surface stable <file.kr>");
    eprintln!("  kernriftc check --surface experimental <file.kr>");
    eprintln!("  kernriftc check --profile kernel <file.kr>");
    eprintln!("  kernriftc check --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --profile kernel --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --policy <policy.toml> <file.kr>");
    eprintln!("  kernriftc check --format json --policy <policy.toml> <file.kr>");
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
    eprintln!("  kernriftc policy --evidence --policy <policy.toml> --contracts <contracts.json>");
    eprintln!(
        "  kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>"
    );
    eprintln!("  kernriftc features --surface stable");
    eprintln!("  kernriftc features --surface experimental");
    eprintln!("  kernriftc proposals");
    eprintln!("  kernriftc proposals --validate");
    eprintln!("  kernriftc proposals --promotion-readiness");
    eprintln!("  kernriftc proposals --promote <feature-id>");
    eprintln!("  kernriftc proposals --promote <feature-id> --dry-run");
    eprintln!("  kernriftc proposals --promote <feature-id> --diff");
    eprintln!("  kernriftc proposals --promote <feature-id> --dry-run --diff");
    eprintln!("  kernriftc migrate-preview --surface stable <file.kr>");
    eprintln!("  kernriftc migrate-preview --surface experimental <file.kr>");
    eprintln!("  kernriftc migrate-preview --canonical-edits --format text <file.kr>");
    eprintln!("  kernriftc migrate-preview --canonical-edits --format json <file.kr>");
    eprintln!(
        "  kernriftc migrate-preview --canonical-edits --format text --surface stable <file.kr>"
    );
    eprintln!(
        "  kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>"
    );
    eprintln!(
        "  kernriftc migrate-preview --canonical-edits --format text --surface experimental <file.kr>"
    );
    eprintln!("  kernriftc migrate-preview --canonical-edits --stdin");
    eprintln!("  kernriftc migrate-preview --canonical-edits --stdin --format json");
    eprintln!(
        "  kernriftc migrate-preview --canonical-edits --format json --surface experimental <file.kr>"
    );
    eprintln!("  kernriftc fix --canonical --write <file.kr>");
    eprintln!("  kernriftc fix --canonical --write --surface experimental <file.kr>");
    eprintln!("  kernriftc fix --canonical --write --format json <file.kr>");
    eprintln!("  kernriftc fix --canonical --dry-run <file.kr>");
    eprintln!("  kernriftc fix --canonical --dry-run --surface experimental <file.kr>");
    eprintln!("  kernriftc fix --canonical --dry-run --format json <file.kr>");
    eprintln!("  kernriftc fix --canonical --dry-run --stdin");
    eprintln!("  kernriftc fix --canonical --dry-run --stdin --format json");
    eprintln!("  kernriftc fix --canonical --stdout <file.kr>");
    eprintln!("  kernriftc fix --canonical --stdout --surface experimental <file.kr>");
    eprintln!("  kernriftc fix --canonical --stdout --stdin");
    eprintln!("  kernriftc fix --canonical --diff <file.kr>");
    eprintln!("  kernriftc fix --canonical --diff --surface experimental <file.kr>");
    eprintln!("  kernriftc fix --canonical --diff --stdin");
    eprintln!("  kernriftc inspect --contracts <contracts.json>");
    eprintln!("  kernriftc inspect-report --report <verify-report.json>");
    eprintln!("  kernriftc inspect-artifact <artifact-path>");
    eprintln!("  kernriftc inspect-artifact <artifact-path> --format json");
    eprintln!("  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256>");
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --sig <contracts.sig> --pubkey <pubkey.hex>"
    );
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --report <verify-report.json>"
    );
    eprintln!("  kernriftc verify-artifact-meta <artifact> <meta.json>");
    eprintln!("  kernriftc verify-artifact-meta --format json <artifact> <meta.json>");
    eprintln!("  kernriftc --selftest");
    eprintln!(
        "  kernriftc --surface stable --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>"
    );
    eprintln!(
        "  kernriftc --surface stable --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>"
    );
    eprintln!("  kernriftc --surface stable --emit=asm -o <output.s> <file.kr>");
    eprintln!("  kernriftc --surface stable --emit=krbo -o <output.krbo> <file.kr>");
    eprintln!("  kernriftc --surface stable --emit=elfobj -o <output.o> <file.kr>");
    eprintln!("  kernriftc --emit=asm -o <output.s> <file.kr>");
    eprintln!("  kernriftc --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>");
    eprintln!("  kernriftc --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>");
    eprintln!("  kernriftc --emit=krbo -o <output.krbo> <file.kr>");
    eprintln!("  kernriftc --emit=elfobj -o <output.o> <file.kr>");
    eprintln!("  kernriftc --emit krir <file.kr>");
    eprintln!("  kernriftc --emit lockgraph <file.kr>");
    eprintln!("  kernriftc --emit caps <file.kr>");
    eprintln!("  kernriftc --emit contracts <file.kr>");
    eprintln!("  kernriftc --emit report --metrics max_lock_depth,no_yield_spans <file.kr>");
    eprintln!("  kernriftc --report max_lock_depth,no_yield_spans <file.kr>");
}
