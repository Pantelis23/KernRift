use std::collections::BTreeSet;

use super::is_asm_symbol_name;
use super::report::{
    ArtifactInspectionAsmSummary, ArtifactInspectionFlags, ArtifactInspectionReport,
    ArtifactInspectionSymbol,
};

pub(crate) fn inspect_asm_artifact(bytes: &[u8]) -> Result<ArtifactInspectionReport, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "inspect-artifact: unsupported artifact bytes".to_string())?;

    let mut globals = BTreeSet::<String>::new();
    let mut labels = BTreeSet::<String>::new();
    let mut call_targets = BTreeSet::<String>::new();
    let mut has_text = false;
    let mut has_instruction = false;
    let mut appears_x86_64_subset = true;

    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line == ".text" {
            has_text = true;
            continue;
        }
        if let Some(raw) = line
            .strip_prefix(".globl ")
            .or_else(|| line.strip_prefix(".global "))
        {
            let symbol = raw.split_whitespace().next().unwrap_or_default().trim();
            if symbol.is_empty() || !is_asm_symbol_name(symbol) {
                appears_x86_64_subset = false;
            } else {
                globals.insert(symbol.to_string());
            }
            continue;
        }
        if let Some(label) = line.strip_suffix(':') {
            if is_asm_symbol_name(label) {
                labels.insert(label.to_string());
            } else {
                appears_x86_64_subset = false;
            }
            continue;
        }
        if let Some(raw_target) = line.strip_prefix("call ") {
            let target = raw_target
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches(',')
                .trim_start_matches('*');
            if target.is_empty() || !is_asm_symbol_name(target) {
                appears_x86_64_subset = false;
            } else {
                call_targets.insert(target.to_string());
            }
            has_instruction = true;
            continue;
        }
        if line == "ret" || line == "retq" {
            has_instruction = true;
            continue;
        }
        appears_x86_64_subset = false;
    }

    if !has_text || labels.is_empty() || !has_instruction {
        return Err("inspect-artifact: unsupported artifact bytes".to_string());
    }

    let undefined_symbols = call_targets
        .iter()
        .filter(|target| !labels.contains(*target))
        .cloned()
        .collect::<Vec<_>>();
    let mut symbols = labels
        .iter()
        .cloned()
        .map(|name| ArtifactInspectionSymbol {
            name,
            category: "function",
            definition: "defined",
        })
        .collect::<Vec<_>>();
    for target in &undefined_symbols {
        symbols.push(ArtifactInspectionSymbol {
            name: target.clone(),
            category: "function",
            definition: "undefined",
        });
    }
    symbols.sort_by(|a, b| a.name.cmp(&b.name).then(a.definition.cmp(b.definition)));

    let defined_symbols = labels.into_iter().collect::<Vec<_>>();
    let globals = globals.into_iter().collect::<Vec<_>>();
    let call_targets = call_targets.into_iter().collect::<Vec<_>>();

    Ok(ArtifactInspectionReport {
        artifact_kind: "asm_text",
        file_size: bytes.len(),
        machine: Some("x86_64"),
        pointer_bits: None,
        endianness: None,
        symbols,
        defined_symbols: defined_symbols.clone(),
        undefined_symbols: undefined_symbols.clone(),
        relocations: Vec::new(),
        asm: Some(ArtifactInspectionAsmSummary {
            globals,
            labels: defined_symbols.clone(),
            direct_call_targets: call_targets,
            appears_x86_64_text_subset: appears_x86_64_subset,
        }),
        flags: ArtifactInspectionFlags {
            has_entry_symbol: defined_symbols.iter().any(|name| name == "entry"),
            has_undefined_symbols: !undefined_symbols.is_empty(),
            has_text_relocations: false,
        },
    })
}
