pub mod runtime;

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
#[allow(unused_imports)] // Used by cc-fallback paths on platforms without native hostexe
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
pub use krir::BackendTargetId as CompilerBackendTargetId;
use krir::{
    BackendTargetContract, BackendTargetId, KRBO_FAT_ARCH_AARCH64, KRBO_FAT_ARCH_X86_64,
    KrirModule, KrirOp, TargetArch, emit_aarch64_asm_text, emit_aarch64_coff_object_bytes,
    emit_aarch64_elf_object_bytes, emit_aarch64_executable_bytes, emit_compiler_owned_object_bytes,
    emit_krbo_bytes, emit_krbofat_bytes, emit_x86_64_asm_text, emit_x86_64_object_bytes,
    lower_current_krir_to_executable_krir, lower_executable_krir_to_aarch64_asm,
    lower_executable_krir_to_aarch64_object_inner,
    lower_executable_krir_to_compiler_owned_object, lower_executable_krir_to_x86_64_asm,
    lower_executable_krir_to_x86_64_object,
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
    /// PE/COFF relocatable object.  Required for linking into UEFI images
    /// (`aarch64-unknown-uefi`) because rust-lld operates in PE/COFF mode
    /// for that target and rejects ELF input files.
    CoffObject,
    ElfExecutable,
    KrboExecutable,
    KrboFat,
    Asm,
    StaticLib,
    /// Host-native executable: links via `cc`/`gcc`, allows extern libc symbols.
    /// Used for host-side build tooling written in KernRift (`build.kr` etc.).
    HostExecutable,
}

impl BackendArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Krbo => "krbo",
            Self::ElfObject => "elfobj",
            Self::CoffObject => "coffobj",
            Self::ElfExecutable => "elfexe",
            Self::KrboExecutable => "krboexe",
            Self::KrboFat => "krbofat",
            Self::Asm => "asm",
            Self::StaticLib => "staticlib",
            Self::HostExecutable => "hostexe",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "krbo" => Ok(Self::Krbo),
            "elfobj" => Ok(Self::ElfObject),
            "coffobj" => Ok(Self::CoffObject),
            "elfexe" => Ok(Self::ElfExecutable),
            "krboexe" => Ok(Self::KrboExecutable),
            "krbofat" => Ok(Self::KrboFat),
            "asm" => Ok(Self::Asm),
            "staticlib" => Ok(Self::StaticLib),
            "hostexe" => Ok(Self::HostExecutable),
            _ => Err(format!(
                "unsupported emit target '{}'; expected 'krbo', 'elfobj', 'coffobj', 'elfexe', 'krboexe', 'krbofat', 'asm', 'staticlib', or 'hostexe'",
                value
            )),
        }
    }
}

pub fn compile_source(src: &str) -> Result<KrirModule, Vec<String>> {
    let module = compile_source_with_surface(src, SurfaceProfile::Stable)?;
    check_module(&module)?;
    Ok(module)
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
        BackendArtifactKind::KrboFat => {
            let x86_target = BackendTargetId::X86_64Sysv.default_contract();
            // When the module has unresolved extern declarations, emit the x86_64 slice
            // in compiler-owned object format (version=0.1, carries fixup records for
            // each unresolved symbol).  When all symbols are self-contained, keep the
            // simple executable slice format (version=1) so existing runtimes can run
            // the artifact directly without a linker step.
            let x86_bytes = if executable.extern_declarations.is_empty() {
                emit_x86_64_executable_bytes(&executable, &x86_target).map_err(|e| vec![e])?
            } else {
                let object =
                    lower_executable_krir_to_compiler_owned_object(&executable, &x86_target)
                        .map_err(|e| vec![e])?;
                emit_compiler_owned_object_bytes(&object)
            };

            // AArch64 encoding supports a subset of instructions (linear MVP).
            // If encoding fails (e.g. unresolved externs or unsupported ops),
            // omit the arm64 slice rather than failing the build.
            let arm_target = BackendTargetId::Aarch64Sysv.default_contract();
            let mut slices = vec![(KRBO_FAT_ARCH_X86_64, x86_bytes)];
            if let Ok(arm_bytes) = emit_aarch64_executable_bytes(&executable, &arm_target) {
                slices.push((KRBO_FAT_ARCH_AARCH64, arm_bytes));
            }

            emit_krbofat_bytes(&slices).map_err(|e| vec![e])
        }
        BackendArtifactKind::ElfObject => match target.arch {
            TargetArch::X86_64 => {
                let object = lower_executable_krir_to_x86_64_object(&executable, &target)
                    .map_err(|err| vec![err])?;
                Ok(emit_x86_64_object_bytes(&object))
            }
            TargetArch::AArch64 => {
                emit_aarch64_elf_object_bytes(&executable, &target).map_err(|err| vec![err])
            }
        },
        BackendArtifactKind::CoffObject => match target.arch {
            TargetArch::X86_64 => Err(vec![
                "coffobj is only supported for arm64; x86_64 COFF is not yet implemented"
                    .to_string(),
            ]),
            TargetArch::AArch64 => {
                // Use the aarch64-win target contract so the emitted COFF carries
                // IMAGE_FILE_MACHINE_ARM64 (0xAA64) and IMAGE_REL_ARM64_BRANCH26
                // relocations that rust-lld accepts in aarch64-unknown-uefi mode.
                let win_target = BackendTargetId::Aarch64Win.default_contract();
                emit_aarch64_coff_object_bytes(&executable, &win_target).map_err(|err| vec![err])
            }
        },
        BackendArtifactKind::ElfExecutable => match target.arch {
            TargetArch::X86_64 => {
                emit_x86_64_elf_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
            TargetArch::AArch64 => {
                emit_aarch64_elf_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
        },
        BackendArtifactKind::KrboExecutable => match target.arch {
            TargetArch::X86_64 => {
                emit_x86_64_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
            TargetArch::AArch64 => {
                emit_aarch64_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
        },
        BackendArtifactKind::Asm => match target.arch {
            TargetArch::X86_64 => {
                let asm = lower_executable_krir_to_x86_64_asm(&executable, &target)
                    .map_err(|err| vec![err])?;
                Ok(emit_x86_64_asm_text(&asm).into_bytes())
            }
            TargetArch::AArch64 => {
                let asm = lower_executable_krir_to_aarch64_asm(&executable, &target)
                    .map_err(|err| vec![err])?;
                Ok(emit_aarch64_asm_text(&asm).into_bytes())
            }
        },
        BackendArtifactKind::StaticLib => match target.arch {
            TargetArch::X86_64 => {
                emit_x86_64_static_library(&executable, &target).map_err(|err| vec![err])
            }
            TargetArch::AArch64 => {
                emit_aarch64_static_library(&executable, &target).map_err(|err| vec![err])
            }
        },
        BackendArtifactKind::HostExecutable => match target.arch {
            TargetArch::X86_64 => {
                emit_x86_64_host_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
            TargetArch::AArch64 => {
                emit_aarch64_host_executable_bytes(&executable, &target).map_err(|err| vec![err])
            }
        },
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
        .map(|s| s.replace("\r\n", "\n"))
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
    _target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
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

    // Lower to x86-64 object using SysV64 ABI
    let object =
        lower_executable_krir_to_x86_64_object(executable, &BackendTargetContract::x86_64_sysv())?;

    // Find entry function offset
    let entry_sym = object
        .function_symbols
        .iter()
        .find(|s| s.name == "entry")
        .ok_or_else(|| "no 'entry' function found in module".to_string())?;
    let entry_offset = u32::try_from(entry_sym.offset)
        .map_err(|_| "entry function offset does not fit in u32".to_string())?;

    // Emit KRBO container
    Ok(emit_krbo_bytes(&object, entry_offset))
}

fn emit_x86_64_elf_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    krir::emit_x86_64_elf_executable(&object)
}

/// Produce a host-native executable.
///
/// On Linux/macOS/Windows, uses the native hostexe path: lower to an x86_64
/// object, concatenate the pre-assembled runtime blob, resolve relocations,
/// and emit ELF/Mach-O/PE directly — no external compiler, assembler, or linker.
///
/// On unsupported platforms, falls back to the cc-based path.
fn emit_x86_64_host_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "linux")]
    {
        return emit_native_hostexe_linux_x86_64(executable, target);
    }

    #[cfg(target_os = "macos")]
    {
        return emit_native_hostexe_macos_x86_64(executable, target);
    }

    #[cfg(target_os = "windows")]
    {
        return emit_native_hostexe_windows_x86_64(executable, target);
    }

    // Fallback for unsupported OSes: shell out to a C compiler driver.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let cc = find_host_tool(&["cc", "gcc", "clang"]).ok_or_else(|| {
            "hostexe emit requires a C compiler driver (cc, gcc, or clang)"
                .to_string()
        })?;

        let asm_module = lower_executable_krir_to_x86_64_asm(executable, target)?;
        let asm_text = emit_x86_64_asm_text(&asm_module);

        let temp_dir = unique_temp_dir("hostexe");
        fs::create_dir_all(&temp_dir).map_err(|err| {
            format!(
                "failed to create temp dir '{}': {}",
                temp_dir.display(),
                err
            )
        })?;

        let cleanup = TempArtifactDir {
            path: temp_dir.clone(),
        };
        let asm_path = temp_dir.join("input.s");
        let object_path = temp_dir.join("input.o");
        let output_path = temp_dir.join("output");

        let asm_final = if !asm_text.contains("\nmain:\n") {
            format!("{}\n.globl main\nmain:\n    jmp entry\n", asm_text)
        } else {
            asm_text
        };

        fs::write(&asm_path, asm_final.as_bytes())
            .map_err(|err| format!("failed to write temp ASM '{}': {}", asm_path.display(), err))?;

        let as_output = Command::new(&cc)
            .arg("-c")
            .arg(&asm_path)
            .arg("-o")
            .arg(&object_path)
            .output()
            .map_err(|err| format!("failed to assemble with '{}': {}", cc, err))?;

        if !as_output.status.success() {
            return Err(format!(
                "hostexe assemble failed with '{}':\nstdout:\n{}\nstderr:\n{}",
                cc,
                String::from_utf8_lossy(&as_output.stdout),
                String::from_utf8_lossy(&as_output.stderr)
            ));
        }

        let mut link_cmd = Command::new(&cc);
        link_cmd.arg(&object_path).arg("-o").arg(&output_path);
        let cc_output = link_cmd
            .output()
            .map_err(|err| format!("failed to link with '{}': {}", cc, err))?;

        if !cc_output.status.success() {
            return Err(format!(
                "hostexe link failed with '{}':\nstdout:\n{}\nstderr:\n{}",
                cc,
                String::from_utf8_lossy(&cc_output.stdout),
                String::from_utf8_lossy(&cc_output.stderr)
            ));
        }

        let bytes = fs::read(&output_path).map_err(|err| {
            format!(
                "failed to read linked output '{}': {}",
                output_path.display(),
                err
            )
        })?;
        drop(cleanup);
        Ok(bytes)
    }
}

/// Native hostexe emitter for Linux x86_64.
///
/// Lowers the KRIR module to an x86_64 relocatable object, concatenates the
/// pre-assembled runtime blob, resolves all relocations (user-to-user and
/// user-to-runtime), patches the runtime's `call main` fixup, and emits a
/// minimal static ELF executable.
#[cfg(target_os = "linux")]
fn emit_native_hostexe_linux_x86_64(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::linux_x86_64::BLOB as RT;

    // 1. Lower user code to x86_64 object
    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    let user_len = object.text_bytes.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&object.text_bytes);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset maps
    let mut user_syms: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for sym in &object.function_symbols {
        user_syms.insert(&sym.name, sym.offset);
    }

    // 4. Resolve relocations
    for reloc in &object.relocations {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(&reloc.target_symbol) {
            // User calls runtime function
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(reloc.target_symbol.as_str()) {
            // User calls user function
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", reloc.target_symbol));
        };

        let value = target_offset - reloc.offset as i64 + reloc.addend;
        let off = reloc.offset as usize;
        if off + 4 > text.len() {
            return Err(format!("hostexe: relocation at {} out of bounds", off));
        }
        text[off..off + 4].copy_from_slice(&(value as i32).to_le_bytes());
    }

    // 5. Patch runtime's "call main" to reach user's main/entry
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let fixup_abs = user_len as u32 + RT.main_call_fixup;
    let displacement = main_offset as i32 - (fixup_abs as i32 + 4);
    let fixup_off = fixup_abs as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&displacement.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Produce ELF executable
    krir::emit_x86_64_elf_executable_for_hostexe(&text, entry_in_text)
}

/// Native hostexe emitter for Linux AArch64.
///
/// Lowers KRIR to an AArch64 relocatable object, concatenates the pre-assembled
/// runtime blob, resolves relocations (BL imm26 patching), patches the runtime's
/// `BL main` fixup, and emits a minimal static ELF executable.
#[cfg(target_os = "linux")]
fn emit_native_hostexe_linux_aarch64(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::linux_aarch64::BLOB as RT;

    // 1. Lower user code to AArch64 object (internal BL/B already resolved)
    let (user_text, sym_table, external_relocs) =
        lower_executable_krir_to_aarch64_object_inner(executable, target)?;
    let user_len = user_text.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&user_text);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset map from user symbols
    let mut user_syms: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    for &(ref name, offset, _size) in &sym_table {
        user_syms.insert(name.as_str(), offset);
    }

    // 4. Resolve external relocations (user code → runtime functions)
    for &(patch_offset, ref target_sym) in &external_relocs {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(target_sym) {
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(target_sym.as_str()) {
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", target_sym));
        };

        // AArch64 BL/B fixup: imm26 field
        let patch_pc = patch_offset as i64;
        let disp = target_offset - patch_pc;
        if disp % 4 != 0 {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' not 4-byte aligned", target_sym
            ));
        }
        let imm26 = (disp / 4) as i32;
        if !(-(1 << 25)..(1 << 25)).contains(&imm26) {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' exceeds imm26 range", target_sym
            ));
        }
        let idx = patch_offset as usize;
        let existing = u32::from_le_bytes(text[idx..idx + 4].try_into().unwrap());
        let word = (existing & 0xFC00_0000) | ((imm26 as u32) & 0x03FF_FFFF);
        text[idx..idx + 4].copy_from_slice(&word.to_le_bytes());
    }

    // 5. Patch runtime's BL main to reach user's main/entry
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let bl_offset = user_len as i64 + RT.main_call_fixup as i64;
    let disp = main_offset as i64 - bl_offset;
    let imm26 = ((disp >> 2) as u32) & 0x03FF_FFFF;
    let bl_instr = 0x94000000u32 | imm26;
    let fixup_off = (user_len as u32 + RT.main_call_fixup) as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&bl_instr.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Produce ELF executable
    krir::emit_aarch64_elf_executable_for_hostexe(&text, entry_in_text)
}

/// Native hostexe emitter for macOS x86_64.
///
/// Same x86_64 encoding as Linux, but uses the macOS runtime blob and emits
/// a Mach-O binary via `krir::emit_macho_executable`.
#[cfg(target_os = "macos")]
fn emit_native_hostexe_macos_x86_64(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::macos_x86_64::BLOB as RT;

    // 1. Lower user code to x86_64 object
    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    let user_len = object.text_bytes.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&object.text_bytes);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset map
    let mut user_syms: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for sym in &object.function_symbols {
        user_syms.insert(&sym.name, sym.offset);
    }

    // 4. Resolve relocations (x86_64 rel32 patching)
    for reloc in &object.relocations {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(&reloc.target_symbol) {
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(reloc.target_symbol.as_str()) {
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", reloc.target_symbol));
        };

        let value = target_offset - reloc.offset as i64 + reloc.addend;
        let off = reloc.offset as usize;
        if off + 4 > text.len() {
            return Err(format!("hostexe: relocation at {} out of bounds", off));
        }
        text[off..off + 4].copy_from_slice(&(value as i32).to_le_bytes());
    }

    // 5. Patch runtime's "call main"
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let fixup_abs = user_len as u32 + RT.main_call_fixup;
    let displacement = main_offset as i32 - (fixup_abs as i32 + 4);
    let fixup_off = fixup_abs as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&displacement.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Produce Mach-O executable (writable text for runtime data area)
    Ok(krir::emit_macho_executable(&text, entry_in_text, false, true))
}

/// Native hostexe emitter for macOS AArch64.
///
/// AArch64 encoding with BL imm26 patching, emits a Mach-O binary.
#[cfg(target_os = "macos")]
fn emit_native_hostexe_macos_aarch64(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::macos_aarch64::BLOB as RT;

    // 1. Lower user code to AArch64 object
    let (user_text, sym_table, external_relocs) =
        lower_executable_krir_to_aarch64_object_inner(executable, target)?;
    let user_len = user_text.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&user_text);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset map
    let mut user_syms: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    for &(ref name, offset, _size) in &sym_table {
        user_syms.insert(name.as_str(), offset);
    }

    // 4. Resolve external relocations (BL imm26 patching)
    for &(patch_offset, ref target_sym) in &external_relocs {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(target_sym) {
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(target_sym.as_str()) {
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", target_sym));
        };

        let patch_pc = patch_offset as i64;
        let disp = target_offset - patch_pc;
        if disp % 4 != 0 {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' not 4-byte aligned", target_sym
            ));
        }
        let imm26 = (disp / 4) as i32;
        if !(-(1 << 25)..(1 << 25)).contains(&imm26) {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' exceeds imm26 range", target_sym
            ));
        }
        let idx = patch_offset as usize;
        let existing = u32::from_le_bytes(text[idx..idx + 4].try_into().unwrap());
        let word = (existing & 0xFC00_0000) | ((imm26 as u32) & 0x03FF_FFFF);
        text[idx..idx + 4].copy_from_slice(&word.to_le_bytes());
    }

    // 5. Patch runtime's BL main
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let bl_offset = user_len as i64 + RT.main_call_fixup as i64;
    let disp = main_offset as i64 - bl_offset;
    let imm26 = ((disp >> 2) as u32) & 0x03FF_FFFF;
    let bl_instr = 0x94000000u32 | imm26;
    let fixup_off = (user_len as u32 + RT.main_call_fixup) as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&bl_instr.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Produce Mach-O executable (arm64, writable text for runtime data area)
    Ok(krir::emit_macho_executable(&text, entry_in_text, true, true))
}

/// Native hostexe emitter for Windows x86_64.
///
/// Same x86_64 encoding as Linux, but uses the Windows runtime blob and emits
/// a PE executable with kernel32.dll imports.  The runtime blob accesses Win32
/// APIs through an IAT pointer stored in its data area.
#[cfg(target_os = "windows")]
fn emit_native_hostexe_windows_x86_64(
    executable: &krir::ExecutableKrirModule,
    _target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::windows_x86_64::BLOB as RT;

    // Windows uses the Win64 ABI (args in rcx/rdx/r8/r9)
    let win_target = krir::BackendTargetContract::x86_64_win64();

    // 1. Lower user code to x86_64 object
    let object = lower_executable_krir_to_x86_64_object(executable, &win_target)?;
    let user_len = object.text_bytes.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&object.text_bytes);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset map
    let mut user_syms: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for sym in &object.function_symbols {
        user_syms.insert(&sym.name, sym.offset);
    }

    // 4. Resolve relocations (x86_64 rel32 patching)
    for reloc in &object.relocations {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(&reloc.target_symbol) {
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(reloc.target_symbol.as_str()) {
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", reloc.target_symbol));
        };

        let value = target_offset - reloc.offset as i64 + reloc.addend;
        let off = reloc.offset as usize;
        if off + 4 > text.len() {
            return Err(format!("hostexe: relocation at {} out of bounds", off));
        }
        text[off..off + 4].copy_from_slice(&(value as i32).to_le_bytes());
    }

    // 5. Patch runtime's "call main"
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let fixup_abs = user_len as u32 + RT.main_call_fixup;
    let displacement = main_offset as i32 - (fixup_abs as i32 + 4);
    let fixup_off = fixup_abs as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&displacement.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Build PE imports for kernel32.dll
    //    IAT order must match the blob's expectations:
    //    [0] GetStdHandle, [1] WriteFile, [2] ExitProcess, [3] VirtualAlloc,
    //    [4] GetEnvironmentVariableA, [5] CreateProcessA,
    //    [6] WaitForSingleObject, [7] GetExitCodeProcess
    let imports = vec![krir::PeImport {
        dll_name: "kernel32.dll".to_string(),
        functions: vec![
            "GetStdHandle".to_string(),
            "WriteFile".to_string(),
            "ExitProcess".to_string(),
            "VirtualAlloc".to_string(),
            "GetEnvironmentVariableA".to_string(),
            "CreateProcessA".to_string(),
            "WaitForSingleObject".to_string(),
            "GetExitCodeProcess".to_string(),
        ],
    }];

    // 8. Patch iat_base data slot with IAT virtual address.
    //    The PE emitter places .text at RVA 0x1000 and .idata immediately after.
    //    Within .idata: IDT, then ILT, then IAT.
    //    IAT offset = IDT_size + ILT_size.
    //    IDT = (num_dlls + 1) * 20.  ILT = (num_funcs + 1) * 8 per DLL.
    let iat_base_off = RT.iat_base_data_offset
        .ok_or_else(|| "windows runtime blob missing iat_base_data_offset".to_string())?;
    let abs_iat_base_off = user_len as u32 + iat_base_off;

    // Compute IAT RVA within .idata.  Mirrors emit_pe_executable_x86_64 layout.
    let text_rva: u32 = 0x1000;
    let text_virtual_size = text.len() as u32;
    let idata_rva = text_rva + ((text_virtual_size.max(1) + 0xFFF) & !0xFFF);
    let num_dlls = imports.len() as u32;
    let idt_size = (num_dlls + 1) * 20;
    let mut ilt_size: u32 = 0;
    for imp in &imports {
        ilt_size += (imp.functions.len() as u32 + 1) * 8;
    }
    let iat_rva = idata_rva + idt_size + ilt_size;
    let image_base: u64 = 0x140000000;
    let iat_va = image_base + iat_rva as u64;

    let slot = abs_iat_base_off as usize;
    text[slot..slot + 8].copy_from_slice(&iat_va.to_le_bytes());

    // 9. Produce PE executable (writable text for runtime data area)
    Ok(krir::emit_pe_executable_x86_64(&text, entry_in_text, &imports, true))
}

/// Native hostexe emitter for Windows AArch64.
///
/// AArch64 encoding with BL imm26 patching, uses the Windows AArch64 runtime
/// blob, and emits a PE executable with kernel32.dll imports and a minimal
/// `.reloc` section (required for DYNAMIC_BASE on ARM64).
#[cfg(target_os = "windows")]
fn emit_native_hostexe_windows_aarch64(
    executable: &krir::ExecutableKrirModule,
    _target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    use crate::runtime::windows_aarch64::BLOB as RT;

    // Windows AArch64 uses the standard AAPCS64 calling convention (same
    // register allocation as other AArch64 targets).
    let win_target = krir::BackendTargetContract::aarch64_win();

    // 1. Lower user code to AArch64 object (internal BL/B already resolved)
    let (user_text, sym_table, external_relocs) =
        lower_executable_krir_to_aarch64_object_inner(executable, &win_target)?;
    let user_len = user_text.len();
    let rt_len = RT.code.len();

    // 2. Combine user code + runtime blob
    let mut text = Vec::with_capacity(user_len + rt_len);
    text.extend_from_slice(&user_text);
    text.extend_from_slice(RT.code);

    // 3. Build symbol offset map from user symbols
    let mut user_syms: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    for &(ref name, offset, _size) in &sym_table {
        user_syms.insert(name.as_str(), offset);
    }

    // 4. Resolve external relocations (user code -> runtime functions)
    for &(patch_offset, ref target_sym) in &external_relocs {
        let target_offset = if let Some(rt_off) = RT.symbol_offset(target_sym) {
            user_len as i64 + rt_off as i64
        } else if let Some(&user_off) = user_syms.get(target_sym.as_str()) {
            user_off as i64
        } else {
            return Err(format!("hostexe: unresolved symbol '{}'", target_sym));
        };

        // AArch64 BL/B fixup: imm26 field
        let patch_pc = patch_offset as i64;
        let disp = target_offset - patch_pc;
        if disp % 4 != 0 {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' not 4-byte aligned", target_sym
            ));
        }
        let imm26 = (disp / 4) as i32;
        if !(-(1 << 25)..(1 << 25)).contains(&imm26) {
            return Err(format!(
                "hostexe aarch64: displacement to '{}' exceeds imm26 range", target_sym
            ));
        }
        let idx = patch_offset as usize;
        let existing = u32::from_le_bytes(text[idx..idx + 4].try_into().unwrap());
        let word = (existing & 0xFC00_0000) | ((imm26 as u32) & 0x03FF_FFFF);
        text[idx..idx + 4].copy_from_slice(&word.to_le_bytes());
    }

    // 5. Patch runtime's BL main to reach user's main/entry
    let main_offset = user_syms.get("main")
        .or_else(|| user_syms.get("entry"))
        .copied()
        .ok_or_else(|| "hostexe: no 'main' or 'entry' symbol".to_string())?;

    let bl_offset = user_len as i64 + RT.main_call_fixup as i64;
    let disp = main_offset as i64 - bl_offset;
    let imm26 = ((disp >> 2) as u32) & 0x03FF_FFFF;
    let bl_instr = 0x94000000u32 | imm26;
    let fixup_off = (user_len as u32 + RT.main_call_fixup) as usize;
    text[fixup_off..fixup_off + 4].copy_from_slice(&bl_instr.to_le_bytes());

    // 6. Entry point = _start in runtime
    let start_offset = RT.symbol_offset("_start")
        .ok_or_else(|| "runtime blob missing _start".to_string())?;
    let entry_in_text = user_len as u32 + start_offset;

    // 7. Build PE imports for kernel32.dll
    //    IAT order must match the blob's expectations:
    //    [0] GetStdHandle, [1] WriteFile, [2] ExitProcess, [3] VirtualAlloc
    let imports = vec![krir::PeImport {
        dll_name: "kernel32.dll".to_string(),
        functions: vec![
            "GetStdHandle".to_string(),
            "WriteFile".to_string(),
            "ExitProcess".to_string(),
            "VirtualAlloc".to_string(),
        ],
    }];

    // 8. Patch iat_base data slot with IAT virtual address.
    //    The PE emitter places .text at RVA 0x1000 and .idata immediately after.
    //    Within .idata: IDT, then ILT, then IAT.
    //    IAT offset = IDT_size + ILT_size.
    //    IDT = (num_dlls + 1) * 20.  ILT = (num_funcs + 1) * 8 per DLL.
    let iat_base_off = RT.iat_base_data_offset
        .ok_or_else(|| "windows runtime blob missing iat_base_data_offset".to_string())?;
    let abs_iat_base_off = user_len as u32 + iat_base_off;

    // Compute IAT RVA within .idata.  Mirrors emit_pe_executable_aarch64 layout.
    let text_rva: u32 = 0x1000;
    let text_virtual_size = text.len() as u32;
    let idata_rva = text_rva + ((text_virtual_size.max(1) + 0xFFF) & !0xFFF);
    let num_dlls = imports.len() as u32;
    let idt_size = (num_dlls + 1) * 20;
    let mut ilt_size: u32 = 0;
    for imp in &imports {
        ilt_size += (imp.functions.len() as u32 + 1) * 8;
    }
    let iat_rva = idata_rva + idt_size + ilt_size;
    let image_base: u64 = 0x140000000;
    let iat_va = image_base + iat_rva as u64;

    let slot = abs_iat_base_off as usize;
    text[slot..slot + 8].copy_from_slice(&iat_va.to_le_bytes());

    // 9. Produce PE executable (writable text for runtime data area)
    Ok(krir::emit_pe_executable_aarch64(&text, entry_in_text, &imports, true))
}

fn emit_x86_64_static_library(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let object = lower_executable_krir_to_x86_64_object(executable, target)?;
    let object_bytes = emit_x86_64_object_bytes(&object);
    let symbols: Vec<&str> = object.function_symbols.iter().map(|s| s.name.as_str()).collect();
    Ok(krir::emit_native_ar_archive("input.o", &object_bytes, &symbols))
}

fn emit_aarch64_elf_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    krir::emit_aarch64_elf_executable_native(executable, target)
}

fn emit_aarch64_static_library(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    let object_bytes = emit_aarch64_elf_object_bytes(executable, target)?;
    let symbols: Vec<&str> = executable.functions.iter().map(|f| f.name.as_str()).collect();
    Ok(krir::emit_native_ar_archive("input.o", &object_bytes, &symbols))
}

/// On Linux/macOS/Windows, uses the native hostexe path for AArch64: lower to
/// an AArch64 object, concatenate the pre-assembled runtime blob, resolve BL
/// relocations, and emit ELF/Mach-O/PE directly.
///
/// On unsupported platforms, falls back to the cc-based path.
fn emit_aarch64_host_executable_bytes(
    executable: &krir::ExecutableKrirModule,
    target: &BackendTargetContract,
) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "linux")]
    {
        return emit_native_hostexe_linux_aarch64(executable, target);
    }

    #[cfg(target_os = "macos")]
    {
        return emit_native_hostexe_macos_aarch64(executable, target);
    }

    #[cfg(target_os = "windows")]
    {
        return emit_native_hostexe_windows_aarch64(executable, target);
    }

    // Exotic OSes: fall back to cc-based path.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let cc = find_host_tool(&["aarch64-linux-gnu-gcc", "cc", "gcc", "clang"]).ok_or_else(|| {
            "hostexe aarch64 emit requires a C compiler (aarch64-linux-gnu-gcc, cc, gcc, or clang)"
                .to_string()
        })?;

        let asm_module = lower_executable_krir_to_aarch64_asm(executable, target)?;
        let asm_text = emit_aarch64_asm_text(&asm_module);

        let temp_dir = unique_temp_dir("hostexe-aarch64");
        fs::create_dir_all(&temp_dir).map_err(|err| {
            format!(
                "failed to create temp dir '{}': {}",
                temp_dir.display(),
                err
            )
        })?;

        let cleanup = TempArtifactDir {
            path: temp_dir.clone(),
        };
        let asm_path = temp_dir.join("input.s");
        let object_path = temp_dir.join("input.o");
        let output_path = temp_dir.join("output");

        fs::write(&asm_path, asm_text.as_bytes())
            .map_err(|err| format!("failed to write temp ASM '{}': {}", asm_path.display(), err))?;

        let as_output = Command::new(&cc)
            .arg("-c")
            .arg(&asm_path)
            .arg("-o")
            .arg(&object_path)
            .output()
            .map_err(|err| format!("failed to assemble with '{}': {}", cc, err))?;

        if !as_output.status.success() {
            return Err(format!(
                "hostexe aarch64 assemble failed with '{}':\nstdout:\n{}\nstderr:\n{}",
                cc,
                String::from_utf8_lossy(&as_output.stdout),
                String::from_utf8_lossy(&as_output.stderr)
            ));
        }

        let cc_output = Command::new(&cc)
            .arg(&object_path)
            .arg("-o")
            .arg(&output_path)
            .output()
            .map_err(|err| format!("failed to link with '{}': {}", cc, err))?;

        if !cc_output.status.success() {
            return Err(format!(
                "hostexe aarch64 link failed with '{}':\nstdout:\n{}\nstderr:\n{}",
                cc,
                String::from_utf8_lossy(&cc_output.stdout),
                String::from_utf8_lossy(&cc_output.stderr)
            ));
        }

        let bytes = fs::read(&output_path).map_err(|err| {
            format!(
                "failed to read linked output '{}': {}",
                output_path.display(),
                err
            )
        })?;
        drop(cleanup);
        Ok(bytes)
    }
}

#[allow(dead_code)] // Used only on platforms without native hostexe
fn find_host_tool(candidates: &[&str]) -> Option<String> {
    // On Windows, also try the `.exe` suffix when searching PATH.
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };

    if let Some(path) = std::env::var_os("PATH") {
        let found = std::env::split_paths(&path).find_map(|dir| {
            candidates.iter().find_map(|candidate| {
                let full = dir.join(format!("{}{}", candidate, exe_suffix));
                if full.is_file() {
                    Some(full.to_string_lossy().into_owned())
                } else {
                    let plain = dir.join(candidate);
                    if plain.is_file() {
                        Some(plain.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                }
            })
        });
        if found.is_some() {
            return found;
        }
    }

    // On Windows, probe VS-bundled LLVM via vswhere.exe when not in PATH.
    #[cfg(windows)]
    {
        if let Some(p) = find_vs_bundled_clang() {
            return Some(p.to_string_lossy().into_owned());
        }
    }

    None
}

/// On Windows, use `vswhere.exe` to locate the VS installation and return the
/// path to the bundled `clang.exe` (ships with the "C++ Clang tools for Windows"
/// optional component, or the "LLVM tools" workload).
#[cfg(windows)]
fn find_vs_bundled_clang() -> Option<PathBuf> {
    let vswhere =
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.is_file() {
        return None;
    }
    let out = Command::new(&vswhere)
        .args(["-latest", "-property", "installationPath"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let vs_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if vs_path.is_empty() {
        return None;
    }
    // VS bundles LLVM in <VS>\VC\Tools\Llvm\x64\bin\clang.exe (x64 host)
    // and <VS>\VC\Tools\Llvm\bin\clang.exe (x86 host).
    for sub in &[
        r"VC\Tools\Llvm\x64\bin\clang.exe",
        r"VC\Tools\Llvm\bin\clang.exe",
    ] {
        let candidate = PathBuf::from(&vs_path).join(sub);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[allow(dead_code)] // Used only on platforms without native hostexe
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

#[allow(dead_code)] // Used only on platforms without native hostexe
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
    use super::{BackendArtifactKind, atomic_temp_path_for, write_atomic_file};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn emit_kind_krbofat_parses() {
        let kind = BackendArtifactKind::parse("krbofat");
        assert!(kind.is_ok());
        assert_eq!(kind.unwrap().as_str(), "krbofat");
    }

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
    /// Number of functions annotated with `@ctx(irq)`. Derived from `ctx_distribution`.
    #[serde(default)]
    pub irq_fn_count: usize,
    /// Deepest observed lock nesting depth. Populated from `passes::AnalysisReport`.
    #[serde(default)]
    pub max_lock_depth: u64,
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
            *ctx_distribution
                .entry(ctx.as_str().to_string())
                .or_insert(0) += 1;
        }
        for eff in &func.eff_used {
            *eff_distribution
                .entry(eff.as_str().to_string())
                .or_insert(0) += 1;
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
        irq_fn_count: module
            .functions
            .iter()
            .filter(|f| f.ctx_ok.contains(&krir::Ctx::Irq))
            .count(),
        max_lock_depth: 0, // filled by caller after passes::analyze_module
        module_caps: module.module_caps.clone(),
    }
}

fn telemetry_op_discriminant(op: &KrirOp) -> String {
    match op {
        KrirOp::Call { .. } => "call",
        KrirOp::CallWithArgs { .. } => "call_with_args",
        KrirOp::TailCall { .. } => "tail_call",
        KrirOp::CallCapture { .. } => "call_capture",
        KrirOp::CallCaptureWithArgs { .. } => "call_capture_with_args",
        KrirOp::BranchIfZero { .. } => "branch_if_zero",
        KrirOp::BranchIfZeroWithArgs { .. } => "branch_if_zero_with_args",
        KrirOp::BranchIfEq { .. } => "branch_if_eq",
        KrirOp::BranchIfMaskNonZero { .. } => "branch_if_mask_non_zero",
        KrirOp::CriticalEnter => "critical_enter",
        KrirOp::CriticalExit => "critical_exit",
        KrirOp::UnsafeEnter => "unsafe_enter",
        KrirOp::UnsafeExit => "unsafe_exit",
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
        KrirOp::RawPtrLoad { .. } => "raw_ptr_load",
        KrirOp::RawPtrStore { .. } => "raw_ptr_store",
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
        KrirOp::InlineAsm(_) => "inline_asm",
        KrirOp::LoadStaticCstrAddr { .. } => "load_static_cstr_addr",
        KrirOp::PrintStdout { .. } => "print_stdout",
        KrirOp::PortIn { .. } => "port_in",
        KrirOp::PortOut { .. } => "port_out",
        KrirOp::Syscall { .. } => "syscall",
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

    // ── Pattern: irq_raw_mmio ─────────────────────────────────────────────────
    let raw_mmio_count = report.op_counts.get("raw_mmio_read").copied().unwrap_or(0)
        + report.op_counts.get("raw_mmio_write").copied().unwrap_or(0);
    if report.irq_fn_count > 0 && raw_mmio_count > 0 {
        let fitness = ((30usize + report.irq_fn_count * 10) as u8).min(80);
        matches.push(PatternMatch {
            id: "irq_raw_mmio",
            title: "Raw MMIO in IRQ context",
            signal: format!(
                "module has {} irq-context function(s) performing raw MMIO — consider a spinlock or atomic abstraction",
                report.irq_fn_count
            ),
            suggestion: "Wrap raw MMIO access in a spinlock or use an atomic abstraction.",
            fitness,
            requires_experimental: false,
        });
    }

    // ── Pattern: high_lock_depth ──────────────────────────────────────────────
    if report.max_lock_depth >= 3 {
        let fitness = (20u64 + report.max_lock_depth.saturating_sub(2) * 15).min(75) as u8;
        matches.push(PatternMatch {
            id: "high_lock_depth",
            title: "Deep lock nesting",
            signal: format!(
                "max lock nesting depth is {} — consider flattening the acquisition order to reduce deadlock risk",
                report.max_lock_depth
            ),
            suggestion: "Restructure lock acquisition so no path holds more than 2 locks simultaneously.",
            fitness,
            requires_experimental: false,
        });
    }

    // ── Pattern: mmio_without_lock ────────────────────────────────────────────
    if report.mmio_register_count > 0 && report.lock_class_count == 0 {
        matches.push(PatternMatch {
            id: "mmio_without_lock",
            title: "MMIO without lock class",
            signal: format!(
                "module declares {} MMIO register(s) but no lock class — concurrent access from thread and IRQ contexts is unguarded",
                report.mmio_register_count
            ),
            suggestion: "Declare a lock class and use it to guard all MMIO access.",
            fitness: 40,
            requires_experimental: false,
        });
    }

    // Sort: fitness descending, then id ascending for deterministic output.
    matches.sort_by(|a, b| b.fitness.cmp(&a.fitness).then(a.id.cmp(b.id)));
    matches
}

#[cfg(test)]
mod lc_pattern_tests {
    use super::*;

    fn base_report() -> TelemetryReport {
        TelemetryReport {
            surface: "stable",
            function_count: 1,
            extern_function_count: 0,
            call_edge_count: 0,
            mmio_base_count: 0,
            mmio_register_count: 0,
            lock_class_count: 0,
            percpu_var_count: 0,
            total_ops: 0,
            op_counts: Default::default(),
            experimental_features: vec![],
            ctx_distribution: Default::default(),
            eff_distribution: Default::default(),
            module_caps: vec![],
            irq_fn_count: 0,
            max_lock_depth: 0,
        }
    }

    #[test]
    fn irq_raw_mmio_fires() {
        let mut r = base_report();
        r.irq_fn_count = 2;
        r.op_counts.insert("raw_mmio_write".to_string(), 3);
        let ms = detect_patterns(&r);
        let m = ms
            .iter()
            .find(|m| m.id == "irq_raw_mmio")
            .expect("pattern fired");
        assert_eq!(m.fitness, 50); // min(30 + 2*10, 80) = 50
    }

    #[test]
    fn irq_raw_mmio_no_fire_without_irq() {
        let mut r = base_report();
        r.op_counts.insert("raw_mmio_write".to_string(), 3);
        let ms = detect_patterns(&r);
        assert!(ms.iter().all(|m| m.id != "irq_raw_mmio"));
    }

    #[test]
    fn high_lock_depth_fires_at_3() {
        let mut r = base_report();
        r.max_lock_depth = 3;
        let ms = detect_patterns(&r);
        let m = ms
            .iter()
            .find(|m| m.id == "high_lock_depth")
            .expect("pattern fired");
        assert_eq!(m.fitness, 35); // 20 + (3-2)*15 = 35
    }

    #[test]
    fn high_lock_depth_no_fire_at_2() {
        let mut r = base_report();
        r.max_lock_depth = 2;
        let ms = detect_patterns(&r);
        assert!(ms.iter().all(|m| m.id != "high_lock_depth"));
    }

    #[test]
    fn mmio_without_lock_fires() {
        let mut r = base_report();
        r.mmio_register_count = 2;
        let ms = detect_patterns(&r);
        let m = ms
            .iter()
            .find(|m| m.id == "mmio_without_lock")
            .expect("pattern fired");
        assert_eq!(m.fitness, 40);
    }

    #[test]
    fn mmio_without_lock_no_fire_with_lock() {
        let mut r = base_report();
        r.mmio_register_count = 2;
        r.lock_class_count = 1;
        let ms = detect_patterns(&r);
        assert!(ms.iter().all(|m| m.id != "mmio_without_lock"));
    }
}
