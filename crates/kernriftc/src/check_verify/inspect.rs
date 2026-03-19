use std::fs;
use std::path::Path;
use std::process::ExitCode;

use emit::emit_report_json;
use kernriftc::{analyze, compile_file};

use super::PolicyOutputFormat;
use super::verify::{
    decode_verify_report, emit_verify_report_inspect_json, format_verify_report_inspect_summary,
};
use crate::policy_engine::{
    decode_contracts_bundle, emit_policy_violations_json, evaluate_policy,
    format_contracts_inspect_summary, load_policy_file, print_policy_violations,
};
use crate::{EXIT_INVALID_INPUT, EXIT_POLICY_VIOLATION, print_errors, print_usage};

pub(crate) fn run_policy(
    policy_path: &str,
    contracts_path: &str,
    evidence: bool,
    format: PolicyOutputFormat,
) -> ExitCode {
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
        match format {
            PolicyOutputFormat::Text => ExitCode::SUCCESS,
            PolicyOutputFormat::Json => match emit_policy_violations_json(&violations, 0) {
                Ok(text) => {
                    print!("{}", text);
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("failed to serialize policy JSON: {}", err);
                    ExitCode::from(EXIT_INVALID_INPUT)
                }
            },
        }
    } else {
        match format {
            PolicyOutputFormat::Text => {
                print_policy_violations(&violations, evidence);
                ExitCode::from(EXIT_POLICY_VIOLATION)
            }
            PolicyOutputFormat::Json => {
                match emit_policy_violations_json(&violations, EXIT_POLICY_VIOLATION) {
                    Ok(text) => {
                        print!("{}", text);
                        ExitCode::from(EXIT_POLICY_VIOLATION)
                    }
                    Err(err) => {
                        eprintln!("failed to serialize policy JSON: {}", err);
                        ExitCode::from(EXIT_INVALID_INPUT)
                    }
                }
            }
        }
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

pub(crate) fn run_inspect_report(report_path: &str, format: PolicyOutputFormat) -> ExitCode {
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

    match format {
        PolicyOutputFormat::Text => {
            println!("{}", format_verify_report_inspect_summary(&report));
            ExitCode::SUCCESS
        }
        PolicyOutputFormat::Json => match emit_verify_report_inspect_json(&report, report_path) {
            Ok(text) => {
                print!("{}", text);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("{}", err);
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
    }
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
