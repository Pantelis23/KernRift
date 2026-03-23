use std::path::Path;
use std::process::ExitCode;

use kernriftc::{collect_telemetry, compile_file_with_surface, detect_patterns};
use passes::analyze_module;

use super::args::{LivingCompilerArgs, LivingCompilerFormat};

pub(crate) fn run_living_compiler(args: &LivingCompilerArgs) -> ExitCode {
    let module = match compile_file_with_surface(Path::new(&args.input_path), args.surface) {
        Ok(m) => m,
        Err(errs) => {
            crate::print_errors(&errs);
            return ExitCode::from(2);
        }
    };

    let (analysis, _) = analyze_module(&module);

    let mut report = collect_telemetry(&module, args.surface);
    report.max_lock_depth = analysis.max_lock_depth;
    let suggestions = detect_patterns(&report);

    match args.format {
        LivingCompilerFormat::Text => print_text(&suggestions),
        LivingCompilerFormat::Json => print_json(&suggestions),
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
