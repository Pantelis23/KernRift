use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hir::lower_to_krir_with_surface;
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
use krir::{
    BackendTargetContract, BackendTargetId, KrirModule, KrirOp, emit_compiler_owned_object_bytes,
    emit_x86_64_asm_text, emit_x86_64_object_bytes, lower_current_krir_to_executable_krir,
    lower_executable_krir_to_compiler_owned_object, lower_executable_krir_to_x86_64_asm,
    lower_executable_krir_to_x86_64_object,
};
pub use krir::BackendTargetId as CompilerBackendTargetId;
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
    ElfExecutable,
    Asm,
    StaticLib,
}

impl BackendArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Krbo => "krbo",
            Self::ElfObject => "elfobj",
            Self::ElfExecutable => "elfexe",
            Self::Asm => "asm",
            Self::StaticLib => "staticlib",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "krbo" => Ok(Self::Krbo),
            "elfobj" => Ok(Self::ElfObject),
            "elfexe" => Ok(Self::ElfExecutable),
            "asm" => Ok(Self::Asm),
            "staticlib" => Ok(Self::StaticLib),
            _ => Err(format!(
                "unsupported emit target '{}'; expected 'krbo', 'elfobj', 'elfexe', 'asm', or 'staticlib'",
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

pub fn emit_backend_artifact_file_with_surface_and_target(
    path: &Path,
    surface_profile: SurfaceProfile,
    kind: BackendArtifactKind,
    target_id: BackendTargetId,
) -> Result<Vec<u8>, Vec<String>> {
    let current = compile_file_with_surface(path, surface_profile)?;
    let executable = lower_current_krir_to_executable_krir(&current)?;
    let target = target_id.default_contract();

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
        BackendArtifactKind::ElfExecutable => {
            emit_x86_64_executable_bytes(&executable, &target).map_err(|err| vec![err])
        }
        BackendArtifactKind::Asm => {
            let asm = lower_executable_krir_to_x86_64_asm(&executable, &target)
                .map_err(|err| vec![err])?;
            Ok(emit_x86_64_asm_text(&asm).into_bytes())
        }
        BackendArtifactKind::StaticLib => {
            emit_x86_64_static_library(&executable, &target).map_err(|err| vec![err])
        }
    }
}

pub fn emit_backend_artifact_file_with_surface(
    path: &Path,
    surface_profile: SurfaceProfile,
    kind: BackendArtifactKind,
) -> Result<Vec<u8>, Vec<String>> {
    emit_backend_artifact_file_with_surface_and_target(
        path,
        surface_profile,
        kind,
        BackendTargetId::X86_64Sysv,
    )
}

pub fn emit_backend_artifact_file(
    path: &Path,
    kind: BackendArtifactKind,
) -> Result<Vec<u8>, Vec<String>> {
    emit_backend_artifact_file_with_surface(path, SurfaceProfile::Stable, kind)
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

fn emit_x86_64_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    if !cfg!(target_os = "linux") {
        return Err("final executable emit currently requires a Linux host".to_string());
    }

    if !executable.extern_declarations.is_empty() {
        let unresolved = executable
            .extern_declarations
            .iter()
            .map(|decl| decl.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "final executable emit currently requires no extern declarations; unresolved externs: {}",
            unresolved
        ));
    }

    if !executable
        .functions
        .iter()
        .any(|function| function.name == "entry")
    {
        return Err(
            "final executable emit currently requires a defined 'entry' function".to_string(),
        );
    }

    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    let object_bytes = emit_x86_64_object_bytes(&object);
    link_x86_64_linux_executable(&object_bytes)
}

fn link_x86_64_linux_executable(object_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let linker = find_host_tool(&["ld.lld", "ld"])
        .ok_or_else(|| "final executable emit requires a host linker (ld.lld or ld)".to_string())?;
    let asm_compiler = find_host_tool(&["cc", "clang", "gcc", "as"]).ok_or_else(|| {
        "final executable emit requires a host assembler/compiler (cc, clang, gcc, or as)"
            .to_string()
    })?;

    let temp_dir = unique_temp_dir("elfexe");
    fs::create_dir_all(&temp_dir).map_err(|err| {
        format!(
            "failed to create temporary link directory '{}': {}",
            temp_dir.display(),
            err
        )
    })?;

    let cleanup = TempArtifactDir {
        path: temp_dir.clone(),
    };
    let input_object = temp_dir.join("input.o");
    let startup_source = temp_dir.join("startup.s");
    let startup_object = temp_dir.join("startup.o");
    let output_path = temp_dir.join("output.elf");

    fs::write(&input_object, object_bytes).map_err(|err| {
        format!(
            "failed to write temporary object '{}': {}",
            input_object.display(),
            err
        )
    })?;

    fs::write(&startup_source, hosted_startup_stub_asm()).map_err(|err| {
        format!(
            "failed to write startup stub '{}': {}",
            startup_source.display(),
            err
        )
    })?;

    let asm_output = match asm_compiler.as_str() {
        "as" => Command::new(&asm_compiler)
            .arg("-o")
            .arg(&startup_object)
            .arg(&startup_source)
            .output(),
        _ => Command::new(&asm_compiler)
            .arg("-c")
            .arg(&startup_source)
            .arg("-o")
            .arg(&startup_object)
            .output(),
    }
    .map_err(|err| {
        format!(
            "failed to run assembler/compiler '{}': {}",
            asm_compiler, err
        )
    })?;

    if !asm_output.status.success() {
        return Err(format!(
            "final executable emit failed while assembling startup stub with '{}'\nstdout:\n{}\nstderr:\n{}",
            asm_compiler,
            String::from_utf8_lossy(&asm_output.stdout),
            String::from_utf8_lossy(&asm_output.stderr)
        ));
    }

    let link_output = Command::new(&linker)
        .arg("-m")
        .arg("elf_x86_64")
        .arg("-e")
        .arg("_start")
        .arg("-o")
        .arg(&output_path)
        .arg(&input_object)
        .arg(&startup_object)
        .output()
        .map_err(|err| format!("failed to run linker '{}': {}", linker, err))?;

    if !link_output.status.success() {
        return Err(format!(
            "final executable emit failed while linking with '{}'\nstdout:\n{}\nstderr:\n{}",
            linker,
            String::from_utf8_lossy(&link_output.stdout),
            String::from_utf8_lossy(&link_output.stderr)
        ));
    }

    let bytes = fs::read(&output_path).map_err(|err| {
        format!(
            "failed to read linked executable '{}': {}",
            output_path.display(),
            err
        )
    })?;
    drop(cleanup);
    Ok(bytes)
}

fn emit_x86_64_static_library(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let ar = find_host_tool(&["ar"])
        .ok_or_else(|| "staticlib emit requires ar (from binutils)".to_string())?;

    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    let object_bytes = emit_x86_64_object_bytes(&object);

    let temp_dir = unique_temp_dir("staticlib");
    fs::create_dir_all(&temp_dir).map_err(|err| {
        format!(
            "failed to create temporary dir '{}': {}",
            temp_dir.display(),
            err
        )
    })?;

    let cleanup = TempArtifactDir {
        path: temp_dir.clone(),
    };
    let object_path = temp_dir.join("input.o");
    let archive_path = temp_dir.join("output.a");

    fs::write(&object_path, &object_bytes).map_err(|err| {
        format!(
            "failed to write temporary object '{}': {}",
            object_path.display(),
            err
        )
    })?;

    let ar_output = Command::new(&ar)
        .arg("rcs")
        .arg(&archive_path)
        .arg(&object_path)
        .output()
        .map_err(|err| format!("failed to run ar '{}': {}", ar, err))?;

    if !ar_output.status.success() {
        return Err(format!(
            "staticlib emit failed while archiving with '{}'\nstdout:\n{}\nstderr:\n{}",
            ar,
            String::from_utf8_lossy(&ar_output.stdout),
            String::from_utf8_lossy(&ar_output.stderr)
        ));
    }

    let bytes = fs::read(&archive_path).map_err(|err| {
        format!(
            "failed to read archive '{}': {}",
            archive_path.display(),
            err
        )
    })?;
    drop(cleanup);
    Ok(bytes)
}

fn hosted_startup_stub_asm() -> &'static str {
    // KernRift programs write output to a "UART" mapped at KERN_UART_BASE (0x10000000).
    // The startup stub:
    //   1. mmaps 4 KB of anonymous read/write memory at KERN_UART_BASE
    //   2. calls entry()
    //   3. null-scans the buffer and write(1, buf, len)
    //   4. exit(0)
    //
    // This lets pure KernRift programs produce visible output on Linux without a C shim.
    // The UART address matches the constant used in hello.kr examples (0x10000000).
    //
    // mmap(2) flags: MAP_PRIVATE(0x02)|MAP_FIXED(0x10)|MAP_ANONYMOUS(0x20) = 0x32
    concat!(
        ".text\n",
        ".globl _start\n",
        "_start:\n",
        // mmap(0x10000000, 0x1000, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_FIXED|MAP_ANON, -1, 0)
        "    mov $9, %rax\n",
        "    mov $0x10000000, %rdi\n",
        "    mov $0x1000, %rsi\n",
        "    mov $3, %rdx\n",
        "    mov $0x32, %r10\n",
        "    mov $-1, %r8\n",
        "    xor %r9d, %r9d\n",
        "    syscall\n",
        // call entry()
        "    call entry\n",
        // scan buf[0..4096] for null terminator, length in %rdx
        "    mov $0x10000000, %rdi\n",
        "    xor %rdx, %rdx\n",
        ".Lscan:\n",
        "    cmpb $0, (%rdi,%rdx)\n",
        "    je .Lwrite\n",
        "    inc %rdx\n",
        "    cmp $0x1000, %rdx\n",
        "    jl .Lscan\n",
        // write(1, 0x10000000, len)
        ".Lwrite:\n",
        "    test %rdx, %rdx\n",
        "    jz .Lexit\n",
        "    mov $1, %rax\n",
        "    mov $1, %rdi\n",
        "    mov $0x10000000, %rsi\n",
        "    syscall\n",
        // exit(0)
        ".Lexit:\n",
        "    mov $60, %rax\n",
        "    xor %edi, %edi\n",
        "    syscall\n",
    )
}

fn find_host_tool(candidates: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        candidates.iter().find_map(|candidate| {
            let full = dir.join(candidate);
            if full.is_file() {
                Some(candidate.to_string())
            } else {
                None
            }
        })
    })
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kernrift-{}-{}-{}",
        label,
        std::process::id(),
        timestamp
    ))
}

struct TempArtifactDir {
    path: PathBuf,
}

impl Drop for TempArtifactDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
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

// ── Telemetry ─────────────────────────────────────────────────────────────────

/// Telemetry snapshot collected from a successfully compiled [`KrirModule`].
///
/// Written as JSON to `--telemetry-out <path>` after each successful
/// `--emit=*` invocation. Intended as a machine-readable feed for
/// PR-7's Living Compiler pattern detector.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TelemetryReport {
    /// The `--surface` profile active during compilation.
    pub surface: &'static str,
    pub function_count: usize,
    pub extern_function_count: usize,
    pub call_edge_count: usize,
    pub mmio_base_count: usize,
    pub mmio_register_count: usize,
    pub lock_class_count: usize,
    pub percpu_var_count: usize,
    /// Total number of ops across all non-extern functions.
    pub total_ops: usize,
    /// Per-op-kind usage counts, keyed by the op discriminant name.
    pub op_counts: std::collections::BTreeMap<String, usize>,
    /// Experimental features present in the module (empty on stable surface).
    pub experimental_features: Vec<&'static str>,
    /// Number of functions that declare each `ctx` annotation.
    pub ctx_distribution: std::collections::BTreeMap<String, usize>,
    /// Number of functions that declare each `eff` annotation.
    pub eff_distribution: std::collections::BTreeMap<String, usize>,
    /// Capability strings declared at the module level.
    pub module_caps: Vec<String>,
}

/// Collect a [`TelemetryReport`] from a compiled module.
pub fn collect_telemetry(module: &KrirModule, surface: SurfaceProfile) -> TelemetryReport {
    let surface_str = match surface {
        SurfaceProfile::Stable => "stable",
        SurfaceProfile::Experimental => "experimental",
    };

    let mut op_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut total_ops: usize = 0;
    let mut has_call_with_args = false;
    let mut has_tail_call = false;
    let mut has_cell_arith_imm = false;
    let mut ctx_distribution: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut eff_distribution: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for func in &module.functions {
        for ctx in &func.ctx_ok {
            *ctx_distribution.entry(ctx.as_str().to_string()).or_insert(0) += 1;
        }
        for eff in &func.eff_used {
            *eff_distribution.entry(eff.as_str().to_string()).or_insert(0) += 1;
        }
        for op in &func.ops {
            total_ops += 1;
            let key = telemetry_op_discriminant(op);
            *op_counts.entry(key).or_insert(0) += 1;
            match op {
                KrirOp::CallWithArgs { .. } => has_call_with_args = true,
                KrirOp::TailCall { .. } => has_tail_call = true,
                KrirOp::CellArithImm { .. } => has_cell_arith_imm = true,
                _ => {}
            }
        }
    }

    let mut experimental_features: Vec<&'static str> = Vec::new();
    if has_call_with_args {
        experimental_features.push("call_with_args");
    }
    if has_tail_call {
        experimental_features.push("tail_call");
    }
    if has_cell_arith_imm {
        experimental_features.push("cell_arith_imm");
    }

    TelemetryReport {
        surface: surface_str,
        function_count: module.functions.len(),
        extern_function_count: module.functions.iter().filter(|f| f.is_extern).count(),
        call_edge_count: module.call_edges.len(),
        mmio_base_count: module.mmio_bases.len(),
        mmio_register_count: module.mmio_registers.len(),
        lock_class_count: module.lock_classes.len(),
        percpu_var_count: module.percpu_vars.len(),
        total_ops,
        op_counts,
        experimental_features,
        ctx_distribution,
        eff_distribution,
        module_caps: module.module_caps.clone(),
    }
}

fn telemetry_op_discriminant(op: &KrirOp) -> String {
    match op {
        KrirOp::Call { .. } => "call",
        KrirOp::CallWithArgs { .. } => "call_with_args",
        KrirOp::TailCall { .. } => "tail_call",
        KrirOp::CallCapture { .. } => "call_capture",
        KrirOp::BranchIfZero { .. } => "branch_if_zero",
        KrirOp::BranchIfEq { .. } => "branch_if_eq",
        KrirOp::BranchIfMaskNonZero { .. } => "branch_if_mask_non_zero",
        KrirOp::CriticalEnter => "critical_enter",
        KrirOp::CriticalExit => "critical_exit",
        KrirOp::YieldPoint => "yield_point",
        KrirOp::AllocPoint => "alloc_point",
        KrirOp::BlockPoint => "block_point",
        KrirOp::Acquire { .. } => "acquire",
        KrirOp::Release { .. } => "release",
        KrirOp::ReturnSlot { .. } => "return_slot",
        KrirOp::StackCell { .. } => "stack_cell",
        KrirOp::StackStore { .. } => "stack_store",
        KrirOp::StackLoad { .. } => "stack_load",
        KrirOp::CellArithImm { .. } => "cell_arith_imm",
        KrirOp::SlotArith { .. } => "slot_arith",
        KrirOp::MmioRead { .. } => "mmio_read",
        KrirOp::MmioWrite { .. } => "mmio_write",
        KrirOp::RawMmioRead { .. } => "raw_mmio_read",
        KrirOp::RawMmioWrite { .. } => "raw_mmio_write",
        KrirOp::SliceLen { .. } => "slice_len",
        KrirOp::SlicePtr { .. } => "slice_ptr",
        KrirOp::PercpuRead { .. } => "percpu_read",
        KrirOp::PercpuWrite { .. } => "percpu_write",
        KrirOp::CompareIntoSlot { .. } => "compare_into_slot",
        KrirOp::LoopBegin => "loop_begin",
        KrirOp::LoopEnd => "loop_end",
        KrirOp::LoopBreak => "loop_break",
        KrirOp::LoopContinue => "loop_continue",
        KrirOp::BranchIfZeroLoopBreak { .. } => "branch_if_zero_loop_break",
        KrirOp::BranchIfNonZeroLoopBreak { .. } => "branch_if_nonzero_loop_break",
        KrirOp::FloatArith { .. } => "float_arith",
    }
    .to_string()
}

// ── Living Compiler ───────────────────────────────────────────────────────────

/// A pattern match produced by [`detect_patterns`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PatternMatch {
    /// Stable machine-readable identifier.
    pub id: &'static str,
    /// Short human-readable title.
    pub title: &'static str,
    /// Dynamic signal text — explains which counts triggered the pattern.
    pub signal: String,
    /// Actionable suggestion text.
    pub suggestion: &'static str,
    /// Fitness score 0–100: higher means the pattern applies more strongly.
    pub fitness: u8,
    /// Whether acting on this suggestion requires `--surface experimental`.
    pub requires_experimental: bool,
}

/// Analyse a [`TelemetryReport`] and return a ranked list of [`PatternMatch`]
/// entries, sorted by `fitness` descending then `id` ascending for stability.
///
/// Returns an empty `Vec` when the module is already well-optimised.
pub fn detect_patterns(report: &TelemetryReport) -> Vec<PatternMatch> {
    let call_count = *report.op_counts.get("call").unwrap_or(&0);
    let tail_call_count = *report.op_counts.get("tail_call").unwrap_or(&0);
    let call_with_args_count = *report.op_counts.get("call_with_args").unwrap_or(&0);
    let stack_cell_count = *report.op_counts.get("stack_cell").unwrap_or(&0);
    let cell_arith_count = *report.op_counts.get("cell_arith_imm").unwrap_or(&0);
    let on_stable = report.surface == "stable";

    let mut matches: Vec<PatternMatch> = Vec::new();

    // ── Pattern: try_tail_call ────────────────────────────────────────────────
    // Fire when the module has plain `call` ops but no `tail_call`.
    // Tail-calls eliminate stack growth in loop-back patterns — a common kernel
    // pattern (IRQ poll loops, ring buffers, state machines).
    if call_count > 0 && tail_call_count == 0 {
        let fitness = ((call_count * 15) as u8).min(100);
        matches.push(PatternMatch {
            id: "try_tail_call",
            title: "Tail-call opportunity",
            signal: format!(
                "{} call op(s) detected; no tail_call present.",
                call_count
            ),
            suggestion: "Replace loop-back call() with tail_call(callee, args...) for zero stack growth.",
            fitness,
            requires_experimental: on_stable,
        });
    }

    // ── Pattern: try_call_with_args ───────────────────────────────────────────
    // Fire when the module has plain `call` ops but no `call_with_args`.
    // call_with_args passes values via SysV argument registers without extra
    // stack marshalling, enabling leaner C ABI boundaries.
    if call_count > 0 && call_with_args_count == 0 {
        let fitness = ((call_count * 12) as u8).min(100);
        matches.push(PatternMatch {
            id: "try_call_with_args",
            title: "Value-passing opportunity",
            signal: format!(
                "{} call op(s) detected; no call_with_args present.",
                call_count
            ),
            suggestion: "Use call_with_args(callee, args...) to pass values directly to callees.",
            fitness,
            requires_experimental: on_stable,
        });
    }

    // ── Pattern: try_cell_arith ───────────────────────────────────────────────
    // Fire when the module has multiple stack cells but no arithmetic ops.
    // cell_add/cell_and/cell_sub/cell_or perform in-place arithmetic without
    // loading into a temporary slot — common for index arithmetic in drivers.
    if stack_cell_count >= 2 && cell_arith_count == 0 {
        let fitness = ((stack_cell_count * 10) as u8).min(100);
        matches.push(PatternMatch {
            id: "try_cell_arith",
            title: "Cell arithmetic opportunity",
            signal: format!(
                "{} stack_cell op(s) detected; no cell_arith_imm present.",
                stack_cell_count
            ),
            suggestion: "Use cell_add/cell_and/cell_sub/cell_or for in-place arithmetic on stack cells.",
            fitness,
            requires_experimental: on_stable,
        });
    }

    // ── Pattern: high_extern_ratio ────────────────────────────────────────────
    // Fire when more than half of all functions are extern declarations.
    // High extern density means the module relies heavily on C-side contracts —
    // worth verifying that @ctx/@eff/@caps annotations match the C reality.
    let function_count = report.function_count;
    let extern_count = report.extern_function_count;
    if extern_count > 0 && function_count > 0 && extern_count * 2 >= function_count {
        let fitness = ((extern_count * 20) as u8).min(100);
        matches.push(PatternMatch {
            id: "high_extern_ratio",
            title: "High extern density",
            signal: format!(
                "{} of {} function(s) are extern declarations.",
                extern_count, function_count
            ),
            suggestion: "Verify that all extern @ctx/@eff/@caps annotations match the C-side contracts.",
            fitness,
            requires_experimental: false,
        });
    }

    // Sort: fitness descending, then id ascending for deterministic output.
    matches.sort_by(|a, b| b.fitness.cmp(&a.fitness).then(a.id.cmp(&b.id)));
    matches
}
