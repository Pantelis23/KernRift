use std::path::Path;

pub use hir::{
    AdaptiveFeaturePromotionPlan, AdaptiveFeaturePromotionReadiness, AdaptiveFeatureProposal,
    AdaptiveFeatureProposalSummary, AdaptiveFeatureStatus, AdaptiveMigrationPreviewEntry,
    AdaptiveSurfaceFeature, FrontendMigrationFeature, FrontendMigrationPreviewEntry,
    SurfaceProfile, adaptive_feature_promotion_plan, adaptive_feature_promotion_readiness,
    adaptive_feature_proposal, adaptive_feature_proposal_summaries, adaptive_feature_proposals,
    adaptive_surface_features, adaptive_surface_features_for_profile,
    adaptive_surface_migration_preview, frontend_migration_features_for_profile,
    frontend_migration_preview, irq_handler_alias_proposal, validate_adaptive_feature_governance,
};
use hir::{
    lower_canonical_executable_to_krir, lower_to_canonical_executable_with_surface,
    lower_to_krir_with_surface,
};
use krir::{
    BackendTargetContract, KrirModule, emit_compiler_owned_object_bytes, emit_x86_64_asm_text,
    emit_x86_64_object_bytes, export_compiler_owned_object_to_x86_64_asm,
    lower_executable_krir_to_compiler_owned_object, lower_executable_krir_to_x86_64_object,
};
use parser::parse_module;
pub use passes::{AnalysisReport, NoYieldSpan};
use passes::{CheckError, analyze_module, run_checks};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendArtifactKind {
    Krbo,
    ElfObject,
    Asm,
}

impl BackendArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Krbo => "krbo",
            Self::ElfObject => "elfobj",
            Self::Asm => "asm",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "krbo" => Ok(Self::Krbo),
            "elfobj" => Ok(Self::ElfObject),
            "asm" => Ok(Self::Asm),
            _ => Err(format!(
                "unsupported emit target '{}'; expected 'krbo', 'elfobj', or 'asm'",
                value
            )),
        }
    }
}

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

pub fn emit_backend_artifact_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
    kind: BackendArtifactKind,
) -> Result<Vec<u8>, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    let ast = parse_module(&src)?;
    let canonical = lower_to_canonical_executable_with_surface(&ast, surface_profile)?;
    let executable = lower_canonical_executable_to_krir(&canonical)?;
    let target = BackendTargetContract::x86_64_sysv();

    match kind {
        BackendArtifactKind::Krbo => {
            let object = lower_executable_krir_to_compiler_owned_object(&executable, &target)
                .map_err(|err| vec![err])?;
            Ok(emit_compiler_owned_object_bytes(&object))
        }
        BackendArtifactKind::ElfObject => {
            let object = lower_executable_krir_to_x86_64_object(&executable, &target)
                .map_err(|err| vec![err])?;
            Ok(emit_x86_64_object_bytes(&object))
        }
        BackendArtifactKind::Asm => {
            let object = lower_executable_krir_to_compiler_owned_object(&executable, &target)
                .map_err(|err| vec![err])?;
            let asm = export_compiler_owned_object_to_x86_64_asm(&object, &target)
                .map_err(|err| vec![err])?;
            Ok(emit_x86_64_asm_text(&asm).into_bytes())
        }
    }
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
) -> Result<Vec<FrontendMigrationPreviewEntry>, Vec<String>> {
    let ast = parse_module(src)?;
    Ok(frontend_migration_preview(&ast, surface_profile))
}

pub fn migrate_preview_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendMigrationPreviewEntry>, Vec<String>> {
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
