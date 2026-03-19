use std::process::ExitCode;

use super::args::VerifyArtifactMetaFormat;
use super::report::{VERIFY_ARTIFACT_META_SCHEMA_VERSION, VerifyArtifactMetaReport};

pub(crate) fn emit_verify_artifact_meta_result(
    format: VerifyArtifactMetaFormat,
    file: &str,
    result: &'static str,
    exit_code: u8,
    message: String,
) -> ExitCode {
    match format {
        VerifyArtifactMetaFormat::Text => {
            if result == "pass" {
                println!("{}", message);
            } else {
                eprintln!("{}", message);
            }
            ExitCode::from(exit_code)
        }
        VerifyArtifactMetaFormat::Json => {
            if result == "invalid_input" {
                eprintln!("{}", message);
                return ExitCode::from(exit_code);
            }
            let report = VerifyArtifactMetaReport {
                schema_version: VERIFY_ARTIFACT_META_SCHEMA_VERSION,
                file: file.to_string(),
                result,
                exit_code,
                message,
            };
            match serde_json::to_string_pretty(&report) {
                Ok(mut text) => {
                    text.push('\n');
                    print!("{}", text);
                    ExitCode::from(exit_code)
                }
                Err(err) => {
                    eprintln!("failed to serialize verify-artifact-meta JSON: {}", err);
                    ExitCode::from(super::EXIT_INVALID_INPUT)
                }
            }
        }
    }
}
