use std::fs;
use std::path::Path;
use std::process::ExitCode;

use kernriftc::{
    BackendArtifactKind, CompilerBackendTargetId, compile_file_with_surface,
    emit_backend_artifact_file_with_surface_and_target,
};
use krir::TargetArch;

use super::args::BackendEmitArgs;

pub(crate) fn run_backend_emit(args: &BackendEmitArgs) -> ExitCode {
    let t0 = std::time::Instant::now();
    // Determine the effective kind and target based on --arch.
    let (effective_kind, effective_target_id) = resolve_kind_and_target(args);

    let bytes = match emit_backend_artifact_file_with_surface_and_target(
        Path::new(&args.input_path),
        args.surface,
        effective_kind,
        effective_target_id,
    ) {
        Ok(bytes) => bytes,
        Err(errs) => {
            crate::print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    if let Err(err) = fs::write(&args.output_path, &bytes) {
        eprintln!("failed to write '{}': {}", args.output_path, err);
        return ExitCode::from(1);
    }

    if let Some(meta_output_path) = &args.meta_output_path
        && let Err(err) = crate::artifact_meta::write_backend_artifact_sidecar(
            effective_kind,
            args.surface,
            &args.input_path,
            meta_output_path,
            &bytes,
        )
    {
        eprintln!("{}", err);
        return ExitCode::from(1);
    }

    if let Some(telemetry_path) = &args.telemetry_output_path {
        let module = match compile_file_with_surface(Path::new(&args.input_path), args.surface) {
            Ok(m) => m,
            Err(errs) => {
                crate::print_errors(&errs);
                return ExitCode::from(1);
            }
        };
        let report = crate::telemetry::collect(&module, args.surface);
        let json = match serde_json::to_string_pretty(&report) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("failed to serialize telemetry: {}", err);
                return ExitCode::from(1);
            }
        };
        if let Err(err) = fs::write(telemetry_path, json.as_bytes()) {
            eprintln!("failed to write telemetry '{}': {}", telemetry_path, err);
            return ExitCode::from(1);
        }
    }

    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("  finished in {:.1} ms", ms);
    ExitCode::SUCCESS
}

/// Resolve the effective artifact kind and target id given the --arch flag.
fn resolve_kind_and_target(
    args: &BackendEmitArgs,
) -> (BackendArtifactKind, CompilerBackendTargetId) {
    match (args.kind, args.arch) {
        // --arch arm64 overrides target to AArch64 SysV
        (kind, Some(TargetArch::AArch64)) => (kind, CompilerBackendTargetId::Aarch64Sysv),
        // All other combinations: use kind and target_id as-is
        (kind, _) => (kind, args.target_id),
    }
}
