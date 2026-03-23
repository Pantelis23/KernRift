use std::path::Path;
use std::process::ExitCode;

use kernriftc::{collect_telemetry, compile_file_with_surface, detect_patterns};
use passes::analyze_module;

use super::args::{LivingCompilerArgs, LivingCompilerFormat};
use super::diff::{compute_diff, git_show_head, DiffStatus};
use super::fix::{apply_fixes, find_fix_sites, unified_diff};

pub(crate) fn run_living_compiler(args: &LivingCompilerArgs) -> ExitCode {
    if args.fix {
        return run_fix_mode(args);
    }
    if args.diff {
        return run_diff_mode(args);
    }
    // Normal mode
    let suggestions = match compile_and_detect(Path::new(&args.input_path), args.surface) {
        Ok(s) => s,
        Err(errs) => {
            crate::print_errors(&errs);
            return ExitCode::from(2);
        }
    };
    let display = filter_by_min_fitness(&suggestions, args);
    match args.format {
        LivingCompilerFormat::Text => print_text(&display),
        LivingCompilerFormat::Json => print_json(&display),
    }
    ci_exit_code(&suggestions, args)
}

fn compile_and_detect(
    path: &Path,
    surface: kernriftc::SurfaceProfile,
) -> Result<Vec<kernriftc::PatternMatch>, Vec<String>> {
    let module = compile_file_with_surface(path, surface)?;
    let (analysis, _) = analyze_module(&module);
    let mut report = collect_telemetry(&module, surface);
    report.max_lock_depth = analysis.max_lock_depth;
    Ok(detect_patterns(&report))
}

fn compile_and_detect_str(
    src: &str,
    surface: kernriftc::SurfaceProfile,
) -> Result<Vec<kernriftc::PatternMatch>, Vec<String>> {
    let module = kernriftc::compile_source_with_surface(src, surface)?;
    let (analysis, _) = analyze_module(&module);
    let mut report = collect_telemetry(&module, surface);
    report.max_lock_depth = analysis.max_lock_depth;
    Ok(detect_patterns(&report))
}

fn filter_by_min_fitness(
    suggestions: &[kernriftc::PatternMatch],
    args: &LivingCompilerArgs,
) -> Vec<kernriftc::PatternMatch> {
    if args.ci || args.min_fitness_explicit {
        suggestions.iter().filter(|m| m.fitness >= args.ci_min_fitness).cloned().collect()
    } else {
        suggestions.to_vec()
    }
}

fn ci_exit_code(suggestions: &[kernriftc::PatternMatch], args: &LivingCompilerArgs) -> ExitCode {
    if args.ci && suggestions.iter().any(|m| m.fitness >= args.ci_min_fitness) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_diff_mode(args: &LivingCompilerArgs) -> ExitCode {
    let (before_src, after_path) = if let Some(after) = &args.diff_after {
        // Two-file mode: read before from input_path
        match std::fs::read_to_string(&args.input_path) {
            Ok(s) => (s, after.as_str()),
            Err(e) => {
                eprintln!("lc diff: cannot read {}: {}", args.input_path, e);
                return ExitCode::from(2);
            }
        }
    } else {
        // Git-aware mode: before from HEAD
        match git_show_head(&args.input_path) {
            Ok(s) => (s, args.input_path.as_str()),
            Err(e) => {
                eprintln!("{}", e);
                return ExitCode::from(2);
            }
        }
    };

    let before_suggestions = compile_and_detect_str(&before_src, args.surface)
        .unwrap_or_default();
    let after_suggestions = match compile_and_detect(Path::new(after_path), args.surface) {
        Ok(s) => s,
        Err(errs) => {
            crate::print_errors(&errs);
            return ExitCode::from(2);
        }
    };

    let entries = compute_diff(&before_suggestions, &after_suggestions);

    println!("lc diff: {} new/worsened suggestion(s)", entries.len());
    for entry in &entries {
        let status_str = match &entry.status {
            DiffStatus::New => "new".to_string(),
            DiffStatus::Worsened { fitness_before } => format!("worsened (was {})", fitness_before),
        };
        println!();
        println!("[{}] {}  fitness: {}", status_str, entry.suggestion.id, entry.suggestion.fitness);
        println!("    {}", entry.suggestion.signal);
    }

    if args.ci && entries.iter().any(|e| e.suggestion.fitness >= args.ci_min_fitness) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_fix_mode(args: &LivingCompilerArgs) -> ExitCode {
    let source = match std::fs::read_to_string(&args.input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lc fix: cannot read {}: {}", args.input_path, e);
            return ExitCode::from(2);
        }
    };

    let suggestions = match compile_and_detect(Path::new(&args.input_path), args.surface) {
        Ok(s) => s,
        Err(errs) => {
            crate::print_errors(&errs);
            return ExitCode::from(2);
        }
    };

    if !suggestions.iter().any(|m| m.id == "try_tail_call") {
        println!("lc fix: nothing to fix");
        return ExitCode::SUCCESS;
    }

    let sites = find_fix_sites(&source);
    if sites.is_empty() {
        println!("lc fix: nothing to fix");
        return ExitCode::SUCCESS;
    }

    if args.dry_run {
        let patched = apply_fixes(&source, &sites);
        print!("{}", unified_diff(&args.input_path, &source, &patched));
        // Spec §5: --fix --dry-run + --ci evaluates pre-fix state
        return ci_exit_code(&suggestions, args);
    }

    // --write: apply atomically
    let patched = apply_fixes(&source, &sites);
    let tmp_path = format!("{}.lc-fix.tmp", args.input_path);
    if let Err(e) = std::fs::write(&tmp_path, &patched) {
        eprintln!("lc fix: failed to write temp file: {}", e);
        return ExitCode::from(2);
    }
    if let Err(e) = std::fs::rename(&tmp_path, &args.input_path) {
        eprintln!("lc fix: failed to rename: {}", e);
        let _ = std::fs::remove_file(&tmp_path);
        return ExitCode::from(2);
    }

    println!("fixed: {} ({} site(s))", args.input_path, sites.len());

    if args.ci {
        let fixed_suggestions = match compile_and_detect(Path::new(&args.input_path), args.surface) {
            Ok(s) => s,
            Err(_) => return ExitCode::from(2),
        };
        return ci_exit_code(&fixed_suggestions, args);
    }

    ExitCode::SUCCESS
}

fn print_text(suggestions: &[kernriftc::PatternMatch]) {
    println!("living-compiler: {} suggestion(s)", suggestions.len());
    for (i, m) in suggestions.iter().enumerate() {
        println!();
        println!("[{}] {}  fitness: {}", i + 1, m.id, m.fitness);
        println!("    title: {}", m.title);
        println!("    signal: {}", m.signal);
        println!("    suggestion: {}", m.suggestion);
        if m.requires_experimental {
            println!("    requires: --surface experimental");
        }
    }
}

fn print_json(suggestions: &[kernriftc::PatternMatch]) {
    let output = serde_json::json!({
        "suggestion_count": suggestions.len(),
        "suggestions": suggestions,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("serialize")
    );
}
