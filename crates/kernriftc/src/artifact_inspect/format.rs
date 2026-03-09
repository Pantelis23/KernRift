use super::report::ArtifactInspectionReport;

pub fn format_artifact_inspection_report_text(report: &ArtifactInspectionReport) -> String {
    let mut lines = Vec::<String>::new();
    lines.push(format!("Artifact: {}", report.artifact_kind));
    lines.push(format!("File size: {} bytes", report.file_size));
    if let Some(machine) = report.machine {
        lines.push(format!("Machine: {}", machine));
    }
    if let Some(pointer_bits) = report.pointer_bits {
        lines.push(format!("Pointer width: {}-bit", pointer_bits));
    }
    if let Some(endianness) = report.endianness {
        lines.push(format!("Endianness: {}", endianness));
    }

    lines.push("Defined symbols:".to_string());
    if report.defined_symbols.is_empty() {
        lines.push("- <none>".to_string());
    } else {
        for symbol in &report.defined_symbols {
            lines.push(format!("- {}", symbol));
        }
    }

    lines.push("Undefined symbols:".to_string());
    if report.undefined_symbols.is_empty() {
        lines.push("- <none>".to_string());
    } else {
        for symbol in &report.undefined_symbols {
            lines.push(format!("- {}", symbol));
        }
    }

    lines.push("Relocations:".to_string());
    if report.relocations.is_empty() {
        lines.push("- <none>".to_string());
    } else {
        for relocation in &report.relocations {
            lines.push(format!(
                "- {} {} -> {}",
                relocation.section, relocation.reloc_type, relocation.target
            ));
        }
    }

    if let Some(asm) = &report.asm {
        lines.push("ASM globals:".to_string());
        if asm.globals.is_empty() {
            lines.push("- <none>".to_string());
        } else {
            for global in &asm.globals {
                lines.push(format!("- {}", global));
            }
        }

        lines.push("ASM labels:".to_string());
        if asm.labels.is_empty() {
            lines.push("- <none>".to_string());
        } else {
            for label in &asm.labels {
                lines.push(format!("- {}", label));
            }
        }

        lines.push("ASM direct call targets:".to_string());
        if asm.direct_call_targets.is_empty() {
            lines.push("- <none>".to_string());
        } else {
            for target in &asm.direct_call_targets {
                lines.push(format!("- {}", target));
            }
        }

        lines.push(format!(
            "ASM appears_x86_64_text_subset: {}",
            yes_no(asm.appears_x86_64_text_subset)
        ));
    }

    lines.push("Flags:".to_string());
    lines.push(format!(
        "- has_entry_symbol: {}",
        yes_no(report.flags.has_entry_symbol)
    ));
    lines.push(format!(
        "- has_undefined_symbols: {}",
        yes_no(report.flags.has_undefined_symbols)
    ));
    lines.push(format!(
        "- has_text_relocations: {}",
        yes_no(report.flags.has_text_relocations)
    ));
    lines.join("\n")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
