use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub use hir::{
    AdaptiveFeaturePromotionPlan, AdaptiveFeaturePromotionReadiness, AdaptiveFeatureProposal,
    AdaptiveFeatureProposalSummary, AdaptiveFeatureStatus, AdaptiveMigrationPreviewEntry,
    AdaptiveSurfaceFeature, FrontendCanonicalEditPlanEntry, FrontendCanonicalFinding,
    FrontendCanonicalRewrite, FrontendMigrationFeature, FrontendMigrationPreviewEntry,
    SurfaceProfile, adaptive_feature_promotion_plan, adaptive_feature_promotion_readiness,
    adaptive_feature_proposal, adaptive_feature_proposal_summaries, adaptive_feature_proposals,
    adaptive_surface_features, adaptive_surface_features_for_profile,
    adaptive_surface_migration_preview, frontend_canonical_edit_plan, frontend_canonical_findings,
    frontend_canonical_rewrites, frontend_migration_features_for_profile,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFixResult {
    pub changed: bool,
    pub rewrites: Vec<FrontendCanonicalRewrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFixPreviewResult {
    pub would_change: bool,
    pub rewrites: Vec<FrontendCanonicalRewrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFixSourceResult {
    pub changed: bool,
    pub original_source: String,
    pub rewritten_source: String,
    pub rewrites: Vec<FrontendCanonicalRewrite>,
}

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

pub fn canonical_check_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendCanonicalFinding>, Vec<String>> {
    let ast = parse_module(src)?;
    Ok(frontend_canonical_findings(&ast, surface_profile))
}

pub fn canonical_check_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendCanonicalFinding>, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    canonical_check_source_with_surface(&src, surface_profile)
}

pub fn canonical_edit_plan_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendCanonicalEditPlanEntry>, Vec<String>> {
    let ast = parse_module(src)?;
    Ok(frontend_canonical_edit_plan(&ast, surface_profile))
}

pub fn canonical_edit_plan_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendCanonicalEditPlanEntry>, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    canonical_edit_plan_source_with_surface(&src, surface_profile)
}

pub fn canonical_fix_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalFixResult, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    let result = canonical_fix_source_text_with_surface(&src, surface_profile)?;

    if !result.changed {
        return Ok(CanonicalFixResult {
            changed: false,
            rewrites: Vec::new(),
        });
    }

    write_atomic_file(path, &result.rewritten_source)?;

    Ok(CanonicalFixResult {
        changed: true,
        rewrites: result.rewrites,
    })
}

pub fn canonical_fix_source_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalFixSourceResult, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    canonical_fix_source_text_with_surface(&src, surface_profile)
}

pub fn canonical_fix_preview_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalFixPreviewResult, Vec<String>> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read '{}': {}", path.display(), e)])?;
    canonical_fix_preview_source_with_surface(&src, surface_profile)
}

pub fn canonical_fix_preview_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalFixPreviewResult, Vec<String>> {
    let rewrites = canonical_rewrites_for_source_with_surface(src, surface_profile)?;
    Ok(CanonicalFixPreviewResult {
        would_change: !rewrites.is_empty(),
        rewrites,
    })
}

fn canonical_rewrites_for_source_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<Vec<FrontendCanonicalRewrite>, Vec<String>> {
    let ast = parse_module(src)?;
    Ok(frontend_canonical_rewrites(&ast, surface_profile))
}

pub fn canonical_fix_source_text_with_surface(
    src: &str,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalFixSourceResult, Vec<String>> {
    let rewrites = canonical_rewrites_for_source_with_surface(src, surface_profile)?;

    if rewrites.is_empty() {
        return Ok(CanonicalFixSourceResult {
            changed: false,
            original_source: src.to_string(),
            rewritten_source: src.to_string(),
            rewrites: Vec::new(),
        });
    }

    let rewritten = apply_canonical_rewrites(src, &rewrites)?;
    let rewritten_ast = parse_module(&rewritten)?;
    let remaining_findings = frontend_canonical_findings(&rewritten_ast, surface_profile);
    if !remaining_findings.is_empty() {
        return Err(vec![format!(
            "canonical fix validation failed: rewritten file still contains {} canonical finding(s)",
            remaining_findings.len()
        )]);
    }

    Ok(CanonicalFixSourceResult {
        changed: true,
        original_source: src.to_string(),
        rewritten_source: rewritten,
        rewrites,
    })
}

fn apply_canonical_rewrites(
    src: &str,
    rewrites: &[FrontendCanonicalRewrite],
) -> Result<String, Vec<String>> {
    let mut rewritten = src.to_string();

    for rewrite in rewrites.iter().rev() {
        let start = rewrite.byte_offset;
        let old_text = format!("@{}", rewrite.surface_form);
        let end = start + old_text.len();
        if end > rewritten.len() {
            return Err(vec![format!(
                "canonical fix validation failed: rewrite for '{}' points outside the source buffer",
                rewrite.function_name
            )]);
        }
        if &rewritten[start..end] != old_text.as_str() {
            return Err(vec![format!(
                "canonical fix validation failed: expected '{}' at byte offset {} for function '{}'",
                old_text, start, rewrite.function_name
            )]);
        }
        rewritten.replace_range(start..end, rewrite.canonical_replacement);
    }

    Ok(rewritten)
}

fn write_atomic_file(path: &Path, contents: &str) -> Result<(), Vec<String>> {
    let temp_path = atomic_temp_path_for(path);

    let mut temp_file = File::create(&temp_path).map_err(|err| {
        vec![format!(
            "failed to write temporary file '{}' before replacing '{}': {}",
            temp_path.display(),
            path.display(),
            err
        )]
    })?;

    temp_file.write_all(contents.as_bytes()).map_err(|err| {
        fs::remove_file(&temp_path).ok();
        vec![format!(
            "failed to write temporary file '{}' before replacing '{}': {}",
            temp_path.display(),
            path.display(),
            err
        )]
    })?;

    temp_file.sync_all().map_err(|err| {
        fs::remove_file(&temp_path).ok();
        vec![format!(
            "failed to flush temporary file '{}' before replacing '{}': {}",
            temp_path.display(),
            path.display(),
            err
        )]
    })?;

    drop(temp_file);

    if let Err(err) = fs::rename(&temp_path, path) {
        fs::remove_file(&temp_path).ok();
        return Err(vec![format!(
            "failed to atomically replace '{}' with '{}': {}",
            path.display(),
            temp_path.display(),
            err
        )]);
    }

    sync_parent_directory_best_effort(path)?;

    Ok(())
}

fn atomic_temp_path_for(path: &Path) -> PathBuf {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("kernrift-fix-target");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    parent.join(format!(".{}.kernrift-fix-{}.tmp", file_name, timestamp))
}

#[cfg(unix)]
fn sync_parent_directory_best_effort(path: &Path) -> Result<(), Vec<String>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let dir = File::open(parent).map_err(|err| {
        vec![format!(
            "failed to open parent directory '{}' for sync after replacing '{}': {}",
            parent.display(),
            path.display(),
            err
        )]
    })?;
    dir.sync_all().map_err(|err| {
        vec![format!(
            "failed to sync parent directory '{}' after replacing '{}': {}",
            parent.display(),
            path.display(),
            err
        )]
    })
}

#[cfg(not(unix))]
fn sync_parent_directory_best_effort(_path: &Path) -> Result<(), Vec<String>> {
    Ok(())
}

fn format_check_errors(mut errs: Vec<CheckError>) -> Vec<String> {
    errs.sort_by(|a, b| (a.pass, a.message.as_str()).cmp(&(b.pass, b.message.as_str())));
    errs.iter().map(format_check_error).collect()
}

fn format_check_error(err: &CheckError) -> String {
    format!("{}: {}", err.pass, err.message)
}

#[cfg(test)]
mod tests {
    use super::{atomic_temp_path_for, write_atomic_file};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kernrift-{}-{}", label, ts));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn atomic_temp_path_stays_in_target_directory() {
        let dir = unique_temp_dir("canonical-fix-path");
        let target = dir.join("sample.kr");
        let temp = atomic_temp_path_for(&target);

        assert_eq!(temp.parent(), Some(dir.as_path()));
        assert_ne!(temp, target);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_file_replaces_contents_without_leaving_temp_file() {
        let dir = unique_temp_dir("canonical-fix-write");
        let target = dir.join("sample.kr");
        fs::write(&target, "@irq\nfn entry() { }\n").expect("seed target");

        write_atomic_file(&target, "@ctx(irq)\nfn entry() { }\n").expect("atomic rewrite");

        assert_eq!(
            fs::read_to_string(&target).expect("read replaced target"),
            "@ctx(irq)\nfn entry() { }\n"
        );

        let leftovers = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".sample.kr.kernrift-fix-"))
            .collect::<Vec<_>>();
        assert!(
            leftovers.is_empty(),
            "atomic fix helper should not leave temp files behind: {:?}",
            leftovers
        );

        fs::remove_dir_all(&dir).ok();
    }
}
