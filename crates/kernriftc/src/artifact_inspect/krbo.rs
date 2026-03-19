use std::collections::BTreeSet;

use super::parse_nul_terminated_utf8;
use super::read_u32_at;
use super::report::{
    ARTIFACT_INSPECTION_SCHEMA_VERSION, ArtifactInspectionFlags, ArtifactInspectionRelocation,
    ArtifactInspectionReport, ArtifactInspectionSymbol,
};

pub(crate) fn inspect_krbo_artifact(bytes: &[u8]) -> Result<ArtifactInspectionReport, String> {
    const HEADER_SIZE: usize = 48;
    const SYMBOL_SIZE: usize = 24;
    const FIXUP_SIZE: usize = 24;

    if bytes.len() < HEADER_SIZE {
        return Err(
            "inspect-artifact: failed to parse KRBO artifact: artifact too small".to_string(),
        );
    }
    if &bytes[0..4] != b"KRBO" {
        return Err("inspect-artifact: failed to parse KRBO artifact: invalid magic".to_string());
    }

    let code_offset = read_u32_at(bytes, 16, true, "KRBO header code_offset")? as usize;
    let code_len = read_u32_at(bytes, 20, true, "KRBO header code_len")? as usize;
    let symbols_offset = read_u32_at(bytes, 24, true, "KRBO header symbols_offset")? as usize;
    let symbol_count = read_u32_at(bytes, 28, true, "KRBO header symbol_count")? as usize;
    let fixups_offset = read_u32_at(bytes, 32, true, "KRBO header fixups_offset")? as usize;
    let fixup_count = read_u32_at(bytes, 36, true, "KRBO header fixup_count")? as usize;
    let strings_offset = read_u32_at(bytes, 40, true, "KRBO header strings_offset")? as usize;
    let strings_len = read_u32_at(bytes, 44, true, "KRBO header strings_len")? as usize;

    let code_end = code_offset.checked_add(code_len).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: code range overflow".to_string()
    })?;
    let symbols_len = symbol_count.checked_mul(SYMBOL_SIZE).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: symbol table overflow".to_string()
    })?;
    let symbols_end = symbols_offset.checked_add(symbols_len).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: symbol range overflow".to_string()
    })?;
    let fixups_len = fixup_count.checked_mul(FIXUP_SIZE).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: fixup table overflow".to_string()
    })?;
    let fixups_end = fixups_offset.checked_add(fixups_len).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: fixup range overflow".to_string()
    })?;
    let strings_end = strings_offset.checked_add(strings_len).ok_or_else(|| {
        "inspect-artifact: failed to parse KRBO artifact: string table range overflow".to_string()
    })?;
    if code_end > bytes.len()
        || symbols_end > bytes.len()
        || fixups_end > bytes.len()
        || strings_end > bytes.len()
    {
        return Err(
            "inspect-artifact: failed to parse KRBO artifact: section range exceeds artifact size"
                .to_string(),
        );
    }

    let strings = &bytes[strings_offset..strings_end];
    let mut symbols = Vec::<ArtifactInspectionSymbol>::new();
    let mut defined_symbols = BTreeSet::<String>::new();
    let mut undefined_symbols = BTreeSet::<String>::new();

    for idx in 0..symbol_count {
        let base = symbols_offset + idx * SYMBOL_SIZE;
        let name_offset = read_u32_at(bytes, base, true, "KRBO symbol name_offset")?;
        let kind_tag = bytes[base + 4];
        let definition_tag = bytes[base + 5];
        let name = parse_nul_terminated_utf8(strings, name_offset as usize, "KRBO symbol name")?;

        let category = match kind_tag {
            1 => "function",
            _ => "unknown",
        };
        let definition = match definition_tag {
            1 => "defined",
            2 => "undefined",
            _ => "unknown",
        };
        if definition_tag == 1 {
            defined_symbols.insert(name.clone());
        } else if definition_tag == 2 {
            undefined_symbols.insert(name.clone());
        }
        symbols.push(ArtifactInspectionSymbol {
            name,
            category,
            definition,
        });
    }
    symbols.sort_by(|a, b| a.name.cmp(&b.name).then(a.definition.cmp(b.definition)));

    let mut relocations = Vec::<ArtifactInspectionRelocation>::new();
    for idx in 0..fixup_count {
        let base = fixups_offset + idx * FIXUP_SIZE;
        let target_offset = read_u32_at(bytes, base + 4, true, "KRBO fixup target_offset")?;
        let kind_tag = bytes[base + 16];
        let width = bytes[base + 17];
        let target =
            parse_nul_terminated_utf8(strings, target_offset as usize, "KRBO fixup target")?;
        let reloc_type = match kind_tag {
            1 => format!("x86_64_call_rel32/w{}", width),
            _ => format!("unknown_fixup_kind_{}/w{}", kind_tag, width),
        };
        relocations.push(ArtifactInspectionRelocation {
            section: ".text".to_string(),
            reloc_type,
            target,
        });
    }
    relocations.sort_by(|a, b| {
        a.section
            .cmp(&b.section)
            .then(a.reloc_type.cmp(&b.reloc_type))
            .then(a.target.cmp(&b.target))
    });

    let defined_symbols = defined_symbols.into_iter().collect::<Vec<_>>();
    let undefined_symbols = undefined_symbols.into_iter().collect::<Vec<_>>();
    let target_id = bytes[9];
    let machine = match target_id {
        1 => Some("x86_64"),
        _ => Some("unknown"),
    };
    let endianness = match bytes[10] {
        1 => Some("little"),
        _ => Some("unknown"),
    };
    let flags = ArtifactInspectionFlags {
        has_entry_symbol: defined_symbols.iter().any(|name| name == "entry"),
        has_undefined_symbols: !undefined_symbols.is_empty(),
        has_text_relocations: !relocations.is_empty(),
    };

    Ok(ArtifactInspectionReport {
        schema_version: ARTIFACT_INSPECTION_SCHEMA_VERSION,
        file: String::new(),
        artifact_kind: "krbo",
        file_size: bytes.len(),
        machine,
        pointer_bits: Some(bytes[11] as u16),
        endianness,
        symbols,
        defined_symbols,
        undefined_symbols,
        relocations,
        asm: None,
        flags,
    })
}
