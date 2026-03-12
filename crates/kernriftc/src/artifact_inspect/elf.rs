use std::collections::BTreeMap;

use super::parse_nul_terminated_utf8;
use super::read_u16_at;
use super::read_u32_at;
use super::read_u64_at;
use super::report::{
    ArtifactInspectionFlags, ArtifactInspectionRelocation, ArtifactInspectionReport,
    ArtifactInspectionSymbol,
};

pub(crate) fn inspect_elf_artifact(bytes: &[u8]) -> Result<ArtifactInspectionReport, String> {
    if bytes.len() < 64 {
        return Err(
            "inspect-artifact: failed to parse ELF artifact: artifact too small".to_string(),
        );
    }
    if &bytes[0..4] != b"\x7fELF" {
        return Err("inspect-artifact: failed to parse ELF artifact: invalid magic".to_string());
    }

    let class = bytes[4];
    if class != 2 {
        return Err(format!(
            "inspect-artifact: failed to parse ELF artifact: unsupported class '{}'",
            class
        ));
    }
    let data = bytes[5];
    let little = match data {
        1 => true,
        2 => false,
        _ => {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: unsupported endianness '{}'",
                data
            ));
        }
    };

    let e_type = read_u16_at(bytes, 16, little, "ELF e_type")?;
    let e_machine = read_u16_at(bytes, 18, little, "ELF e_machine")?;
    let e_shoff = read_u64_at(bytes, 40, little, "ELF e_shoff")? as usize;
    let e_shentsize = read_u16_at(bytes, 58, little, "ELF e_shentsize")? as usize;
    let e_shnum = read_u16_at(bytes, 60, little, "ELF e_shnum")? as usize;
    let e_shstrndx = read_u16_at(bytes, 62, little, "ELF e_shstrndx")? as usize;

    #[derive(Debug, Clone)]
    struct RawSection {
        name_offset: u32,
        section_type: u32,
        offset: u64,
        size: u64,
        link: u32,
        entsize: u64,
    }
    #[derive(Debug, Clone)]
    struct Section {
        name: String,
        section_type: u32,
        offset: usize,
        size: usize,
        link: usize,
        entsize: usize,
    }

    let mut raw_sections = Vec::<RawSection>::new();
    if e_shnum > 0 {
        if e_shentsize < 64 {
            return Err(
                "inspect-artifact: failed to parse ELF artifact: invalid section header size"
                    .to_string(),
            );
        }
        let sh_table_len = e_shnum.checked_mul(e_shentsize).ok_or_else(|| {
            "inspect-artifact: failed to parse ELF artifact: section table overflow".to_string()
        })?;
        let sh_end = e_shoff.checked_add(sh_table_len).ok_or_else(|| {
            "inspect-artifact: failed to parse ELF artifact: section table range overflow"
                .to_string()
        })?;
        if sh_end > bytes.len() {
            return Err(
                "inspect-artifact: failed to parse ELF artifact: section table exceeds artifact size"
                    .to_string(),
            );
        }
        for idx in 0..e_shnum {
            let base = e_shoff + idx * e_shentsize;
            raw_sections.push(RawSection {
                name_offset: read_u32_at(bytes, base, little, "ELF sh_name")?,
                section_type: read_u32_at(bytes, base + 4, little, "ELF sh_type")?,
                offset: read_u64_at(bytes, base + 24, little, "ELF sh_offset")?,
                size: read_u64_at(bytes, base + 32, little, "ELF sh_size")?,
                link: read_u32_at(bytes, base + 40, little, "ELF sh_link")?,
                entsize: read_u64_at(bytes, base + 56, little, "ELF sh_entsize")?,
            });
        }
    }

    let mut sections = Vec::<Section>::new();
    if !raw_sections.is_empty() {
        if e_shstrndx >= raw_sections.len() {
            return Err(
                "inspect-artifact: failed to parse ELF artifact: invalid shstrndx".to_string(),
            );
        }
        let shstr = &raw_sections[e_shstrndx];
        let shstr_offset = usize::try_from(shstr.offset).map_err(|_| {
            "inspect-artifact: failed to parse ELF artifact: shstrtab offset out of range"
                .to_string()
        })?;
        let shstr_size = usize::try_from(shstr.size).map_err(|_| {
            "inspect-artifact: failed to parse ELF artifact: shstrtab size out of range".to_string()
        })?;
        let shstr_end = shstr_offset.checked_add(shstr_size).ok_or_else(|| {
            "inspect-artifact: failed to parse ELF artifact: shstrtab range overflow".to_string()
        })?;
        if shstr_end > bytes.len() {
            return Err(
                "inspect-artifact: failed to parse ELF artifact: shstrtab exceeds artifact size"
                    .to_string(),
            );
        }
        let shstr_bytes = &bytes[shstr_offset..shstr_end];

        for raw in &raw_sections {
            let offset = usize::try_from(raw.offset).map_err(|_| {
                "inspect-artifact: failed to parse ELF artifact: section offset out of range"
                    .to_string()
            })?;
            let size = usize::try_from(raw.size).map_err(|_| {
                "inspect-artifact: failed to parse ELF artifact: section size out of range"
                    .to_string()
            })?;
            let end = offset.checked_add(size).ok_or_else(|| {
                "inspect-artifact: failed to parse ELF artifact: section range overflow".to_string()
            })?;
            if end > bytes.len() {
                return Err(
                    "inspect-artifact: failed to parse ELF artifact: section exceeds artifact size"
                        .to_string(),
                );
            }
            let name = parse_nul_terminated_utf8(
                shstr_bytes,
                raw.name_offset as usize,
                "ELF section name",
            )?;
            sections.push(Section {
                name,
                section_type: raw.section_type,
                offset,
                size,
                link: raw.link as usize,
                entsize: usize::try_from(raw.entsize).unwrap_or(0),
            });
        }
    }

    let mut symbol_map = BTreeMap::<String, (&'static str, &'static str)>::new();
    let mut symbol_names_by_section = BTreeMap::<usize, Vec<String>>::new();
    for (section_index, section) in sections.iter().enumerate() {
        if section.section_type != 2 && section.section_type != 11 {
            continue;
        }
        if section.link >= sections.len() {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: symbol section '{}' has invalid sh_link",
                section.name
            ));
        }
        let strtab = &sections[section.link];
        let strtab_bytes = &bytes[strtab.offset..strtab.offset + strtab.size];
        let ent_size = if section.entsize == 0 {
            24
        } else {
            section.entsize
        };
        if ent_size < 24 {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: symbol section '{}' has invalid entsize",
                section.name
            ));
        }
        if section.size % ent_size != 0 {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: symbol section '{}' has non-integral entry count",
                section.name
            ));
        }

        let mut names = Vec::<String>::new();
        let count = section.size / ent_size;
        for idx in 0..count {
            let base = section.offset + idx * ent_size;
            let st_name = read_u32_at(bytes, base, little, "ELF st_name")? as usize;
            let st_info = bytes.get(base + 4).copied().ok_or_else(|| {
                "inspect-artifact: failed to parse ELF artifact: symbol info out of bounds"
                    .to_string()
            })?;
            let st_shndx = read_u16_at(bytes, base + 6, little, "ELF st_shndx")?;
            let name = parse_nul_terminated_utf8(strtab_bytes, st_name, "ELF symbol name")?;
            names.push(name.clone());
            if name.is_empty() {
                continue;
            }
            let category = match st_info & 0x0f {
                2 => "function",
                1 => "object",
                3 => "section",
                _ => "other",
            };
            let definition = if st_shndx == 0 {
                "undefined"
            } else {
                "defined"
            };
            match symbol_map.get_mut(&name) {
                Some((existing_category, existing_definition)) => {
                    if *existing_definition == "undefined" && definition == "defined" {
                        *existing_definition = "defined";
                    }
                    if *existing_category == "other" && category != "other" {
                        *existing_category = category;
                    }
                }
                None => {
                    symbol_map.insert(name, (category, definition));
                }
            }
        }
        symbol_names_by_section.insert(section_index, names);
    }

    let machine_name = elf_machine_name(e_machine);
    let mut relocations = Vec::<ArtifactInspectionRelocation>::new();
    for section in &sections {
        if section.section_type != 4 && section.section_type != 9 {
            continue;
        }
        if section.link >= sections.len() {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: relocation section '{}' has invalid sh_link",
                section.name
            ));
        }
        let symbols = symbol_names_by_section.get(&section.link).ok_or_else(|| {
            format!(
                "inspect-artifact: failed to parse ELF artifact: relocation section '{}' references missing symbol table",
                section.name
            )
        })?;

        let ent_size = if section.entsize == 0 {
            if section.section_type == 4 { 24 } else { 16 }
        } else {
            section.entsize
        };
        if ent_size < 16 {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: relocation section '{}' has invalid entsize",
                section.name
            ));
        }
        if section.size % ent_size != 0 {
            return Err(format!(
                "inspect-artifact: failed to parse ELF artifact: relocation section '{}' has non-integral entry count",
                section.name
            ));
        }

        let count = section.size / ent_size;
        for idx in 0..count {
            let base = section.offset + idx * ent_size;
            let r_info = read_u64_at(bytes, base + 8, little, "ELF r_info")?;
            let symbol_index = (r_info >> 32) as usize;
            let reloc_type = (r_info & 0xFFFF_FFFF) as u32;
            if symbol_index >= symbols.len() {
                return Err(format!(
                    "inspect-artifact: failed to parse ELF artifact: relocation section '{}' entry {} references out-of-range symbol index {}",
                    section.name, idx, symbol_index
                ));
            }
            let target = symbols[symbol_index].clone();
            relocations.push(ArtifactInspectionRelocation {
                section: section.name.clone(),
                reloc_type: elf_relocation_name(machine_name, reloc_type).to_string(),
                target,
            });
        }
    }
    relocations.sort_by(|a, b| {
        a.section
            .cmp(&b.section)
            .then(a.reloc_type.cmp(&b.reloc_type))
            .then(a.target.cmp(&b.target))
    });

    let mut symbols = symbol_map
        .into_iter()
        .map(|(name, (category, definition))| ArtifactInspectionSymbol {
            name,
            category,
            definition,
        })
        .collect::<Vec<_>>();
    symbols.sort_by(|a, b| a.name.cmp(&b.name).then(a.definition.cmp(b.definition)));

    let defined_symbols = symbols
        .iter()
        .filter(|symbol| symbol.definition == "defined")
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();
    let undefined_symbols = symbols
        .iter()
        .filter(|symbol| symbol.definition == "undefined")
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();

    let has_text_relocations = relocations
        .iter()
        .any(|reloc| reloc.section == ".rela.text" || reloc.section == ".rel.text");
    let artifact_kind = match e_type {
        1 => "elf_relocatable",
        2 => "elf_executable",
        3 => "elf_shared",
        _ => "elf_other",
    };

    Ok(ArtifactInspectionReport {
        artifact_kind,
        file_size: bytes.len(),
        machine: Some(machine_name),
        pointer_bits: Some(64),
        endianness: Some(if little { "little" } else { "big" }),
        symbols,
        defined_symbols: defined_symbols.clone(),
        undefined_symbols: undefined_symbols.clone(),
        relocations,
        asm: None,
        flags: ArtifactInspectionFlags {
            has_entry_symbol: defined_symbols.iter().any(|name| name == "entry"),
            has_undefined_symbols: !undefined_symbols.is_empty(),
            has_text_relocations,
        },
    })
}

fn elf_machine_name(code: u16) -> &'static str {
    match code {
        62 => "x86_64",
        _ => "unknown",
    }
}

fn elf_relocation_name(machine: &str, relocation: u32) -> &'static str {
    if machine != "x86_64" {
        return "unknown";
    }
    match relocation {
        1 => "R_X86_64_64",
        2 => "R_X86_64_PC32",
        4 => "R_X86_64_PLT32",
        10 => "R_X86_64_32",
        11 => "R_X86_64_32S",
        _ => "R_X86_64_UNKNOWN",
    }
}
