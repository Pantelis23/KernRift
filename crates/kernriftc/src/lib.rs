use std::path::Path;

use hir::lower_to_krir_with_surface;
pub use hir::{
    AdaptiveFeatureProposal, AdaptiveFeatureProposalSummary, AdaptiveFeatureStatus,
    AdaptiveMigrationPreviewEntry, AdaptiveSurfaceFeature, SurfaceProfile,
    adaptive_feature_proposal, adaptive_feature_proposal_summaries, adaptive_feature_proposals,
    adaptive_surface_features, adaptive_surface_features_for_profile,
    adaptive_surface_migration_preview, irq_handler_alias_proposal,
    validate_adaptive_feature_governance,
};
use krir::KrirModule;
use parser::parse_module;
pub use passes::{AnalysisReport, NoYieldSpan};
use passes::{CheckError, analyze_module, run_checks};

pub fn compile_source(src: &str) -> Result<KrirModule, Vec<String>> {
    compile_source_with_surface(src, SurfaceProfile::Stable)
}

pub fn compile_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<KrirModule, Vec<String>> {
    let ast = parse_module(src)?;
    lower_to_krir_with_surface(&ast, surface_profile)
}

pub fn compile_file(path: &Path) -> Result<KrirModule, Vec<String>> {
    compile_file_with_surface(path, SurfaceProfile::Stable)
}

pub fn compile_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<KrirModule, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    compile_source_with_surface(&src, surface_profile)
}

pub fn check_module(module: &KrirModule) -> Result<(), Vec<String>> {
    run_checks(module).map_err(format_check_errors)
}

pub fn check_file(path: &Path) -> Result<(), Vec<String>> {
    check_file_with_surface(path, SurfaceProfile::Stable)
}

pub fn check_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<(), Vec<String>> {
    let module = compile_file_with_surface(path, surface_profile)?;
    check_module(&module)
}

pub fn analyze(module: &KrirModule) -> (AnalysisReport, Vec<String>) {
    let (report, errs) = analyze_module(module);
    let formatted = format_check_errors(errs);
    (report, formatted)
}

pub fn analyze_file(path: &Path) -> Result<(AnalysisReport, Vec<String>), Vec<String>> {
    let module = compile_file(path)?;
    Ok(analyze(&module))
}

pub fn migrate_preview_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<Vec<AdaptiveMigrationPreviewEntry>, Vec<String>> {
    let ast = parse_module(src)?;
    Ok(adaptive_surface_migration_preview(&ast, surface_profile))
}

pub fn migrate_preview_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<Vec<AdaptiveMigrationPreviewEntry>, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    migrate_preview_source_with_surface(&src, surface_profile)
}

fn format_check_errors(mut errs: Vec<CheckError>) -> Vec<String> {
    errs.sort_by(|a, b| (a.pass, a.message.as_str()).cmp(&(b.pass, b.message.as_str())));
    errs.iter().map(format_check_error).collect()
}

fn format_check_error(err: &CheckError) -> String {
    format!("{}: {}", err.pass, err.message)
}
