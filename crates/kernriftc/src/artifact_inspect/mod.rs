mod asm;
mod elf;
mod format;
mod krbo;
mod report;

pub use format::format_artifact_inspection_report_text;
pub use report::ArtifactInspectionReport;

pub fn inspect_artifact_from_bytes(bytes: &[u8]) -> Result<ArtifactInspectionReport, String> {
    if bytes.len() >= 4 && &bytes[0..4] == b"KRBO" {
        return krbo::inspect_krbo_artifact(bytes);
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"\x7fELF" {
        return elf::inspect_elf_artifact(bytes);
    }
    if let Ok(report) = asm::inspect_asm_artifact(bytes) {
        return Ok(report);
    }
    Err("inspect-artifact: unsupported artifact bytes".to_string())
}

pub(crate) fn parse_nul_terminated_utf8(
    bytes: &[u8],
    offset: usize,
    context: &str,
) -> Result<String, String> {
    if offset >= bytes.len() {
        return Err(format!(
            "inspect-artifact: failed to parse {}: string offset out of bounds",
            context
        ));
    }
    let tail = &bytes[offset..];
    let terminator = tail.iter().position(|byte| *byte == 0).ok_or_else(|| {
        format!(
            "inspect-artifact: failed to parse {}: missing NUL terminator",
            context
        )
    })?;
    std::str::from_utf8(&tail[..terminator])
        .map(|text| text.to_string())
        .map_err(|_| {
            format!(
                "inspect-artifact: failed to parse {}: invalid UTF-8",
                context
            )
        })
}

pub(crate) fn read_u16_at(
    bytes: &[u8],
    offset: usize,
    little: bool,
    context: &str,
) -> Result<u16, String> {
    let slice = bytes.get(offset..offset + 2).ok_or_else(|| {
        format!(
            "inspect-artifact: failed to parse {}: out-of-bounds read",
            context
        )
    })?;
    let array = [slice[0], slice[1]];
    Ok(if little {
        u16::from_le_bytes(array)
    } else {
        u16::from_be_bytes(array)
    })
}

pub(crate) fn read_u32_at(
    bytes: &[u8],
    offset: usize,
    little: bool,
    context: &str,
) -> Result<u32, String> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(|| {
        format!(
            "inspect-artifact: failed to parse {}: out-of-bounds read",
            context
        )
    })?;
    let array = [slice[0], slice[1], slice[2], slice[3]];
    Ok(if little {
        u32::from_le_bytes(array)
    } else {
        u32::from_be_bytes(array)
    })
}

pub(crate) fn read_u64_at(
    bytes: &[u8],
    offset: usize,
    little: bool,
    context: &str,
) -> Result<u64, String> {
    let slice = bytes.get(offset..offset + 8).ok_or_else(|| {
        format!(
            "inspect-artifact: failed to parse {}: out-of-bounds read",
            context
        )
    })?;
    let array = [
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ];
    Ok(if little {
        u64::from_le_bytes(array)
    } else {
        u64::from_be_bytes(array)
    })
}

pub(crate) fn is_asm_symbol_name(symbol: &str) -> bool {
    let mut chars = symbol.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || matches!(first, '_' | '.' | '$')) {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '$' | '@'))
}

#[cfg(test)]
mod tests {
    use super::{elf, inspect_artifact_from_bytes, krbo};
    use krir::{
        BackendTargetId, CompilerOwnedCodeSection, CompilerOwnedFixupKind, CompilerOwnedObject,
        CompilerOwnedObjectFixup, CompilerOwnedObjectHeader, CompilerOwnedObjectKind,
        CompilerOwnedObjectSymbol, CompilerOwnedObjectSymbolDefinition,
        CompilerOwnedObjectSymbolKind, TargetEndian, X86_64ElfFunctionSymbol,
        X86_64ElfRelocatableObject, X86_64ElfRelocation, X86_64ElfRelocationKind,
        emit_compiler_owned_object_bytes, emit_x86_64_object_bytes,
    };

    fn write_u16_le(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64_le(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn minimal_krbo_bytes() -> Vec<u8> {
        emit_compiler_owned_object_bytes(&CompilerOwnedObject {
            header: CompilerOwnedObjectHeader {
                magic: *b"KRBO",
                version_major: 0,
                version_minor: 1,
                object_kind: CompilerOwnedObjectKind::LinearRelocatable,
                target_id: BackendTargetId::X86_64Sysv,
                endian: TargetEndian::Little,
                pointer_bits: 64,
                format_revision: 2,
            },
            code: CompilerOwnedCodeSection {
                name: ".text",
                bytes: vec![0xC3],
            },
            symbols: vec![CompilerOwnedObjectSymbol {
                name: "entry".to_string(),
                kind: CompilerOwnedObjectSymbolKind::Function,
                definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
                offset: 0,
                size: 1,
            }],
            fixups: Vec::new(),
        })
    }

    fn reloc_krbo_bytes() -> Vec<u8> {
        emit_compiler_owned_object_bytes(&CompilerOwnedObject {
            header: CompilerOwnedObjectHeader {
                magic: *b"KRBO",
                version_major: 0,
                version_minor: 1,
                object_kind: CompilerOwnedObjectKind::LinearRelocatable,
                target_id: BackendTargetId::X86_64Sysv,
                endian: TargetEndian::Little,
                pointer_bits: 64,
                format_revision: 2,
            },
            code: CompilerOwnedCodeSection {
                name: ".text",
                bytes: vec![0xE8, 0, 0, 0, 0, 0xC3],
            },
            symbols: vec![
                CompilerOwnedObjectSymbol {
                    name: "entry".to_string(),
                    kind: CompilerOwnedObjectSymbolKind::Function,
                    definition: CompilerOwnedObjectSymbolDefinition::DefinedText,
                    offset: 0,
                    size: 6,
                },
                CompilerOwnedObjectSymbol {
                    name: "ext".to_string(),
                    kind: CompilerOwnedObjectSymbolKind::Function,
                    definition: CompilerOwnedObjectSymbolDefinition::UndefinedExternal,
                    offset: 0,
                    size: 0,
                },
            ],
            fixups: vec![CompilerOwnedObjectFixup {
                source_symbol: "entry".to_string(),
                patch_offset: 1,
                kind: CompilerOwnedFixupKind::X86_64CallRel32,
                target_symbol: "ext".to_string(),
                width_bytes: 4,
            }],
        })
    }

    fn reloc_elf_bytes() -> Vec<u8> {
        emit_x86_64_object_bytes(&X86_64ElfRelocatableObject {
            format: "elf64-relocatable",
            text_section: ".text",
            text_bytes: vec![0xE8, 0, 0, 0, 0, 0xC3],
            function_symbols: vec![X86_64ElfFunctionSymbol {
                name: "entry".to_string(),
                offset: 0,
                size: 6,
            }],
            undefined_function_symbols: vec!["ext".to_string()],
            relocations: vec![X86_64ElfRelocation {
                offset: 1,
                kind: X86_64ElfRelocationKind::X86_64Plt32,
                target_symbol: "ext".to_string(),
                addend: -4,
            }],
        })
    }

    #[test]
    fn krbo_rejects_malformed_header_too_small() {
        let err = krbo::inspect_krbo_artifact(b"KRBO").expect_err("tiny KRBO must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse KRBO artifact: artifact too small"
        );
    }

    #[test]
    fn krbo_rejects_out_of_bounds_symbol_name_offset() {
        let mut bytes = minimal_krbo_bytes();
        let symbols_offset = u32::from_le_bytes(bytes[24..28].try_into().expect("u32")) as usize;
        write_u32_le(&mut bytes, symbols_offset, 0xFFFF_FFFF);
        let err = krbo::inspect_krbo_artifact(&bytes)
            .expect_err("invalid symbol string offset must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse KRBO symbol name: string offset out of bounds"
        );
    }

    #[test]
    fn krbo_rejects_out_of_bounds_fixup_target_offset() {
        let mut bytes = reloc_krbo_bytes();
        let fixups_offset = u32::from_le_bytes(bytes[32..36].try_into().expect("u32")) as usize;
        write_u32_le(&mut bytes, fixups_offset + 4, 0xFFFF_FFFF);
        let err =
            krbo::inspect_krbo_artifact(&bytes).expect_err("invalid fixup target offset must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse KRBO fixup target: string offset out of bounds"
        );
    }

    #[test]
    fn elf_rejects_malformed_magic() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELX");
        bytes[4] = 2;
        bytes[5] = 1;
        let err = elf::inspect_elf_artifact(&bytes).expect_err("invalid ELF magic must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse ELF artifact: invalid magic"
        );
    }

    #[test]
    fn elf_rejects_unsupported_class() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 1;
        bytes[5] = 1;
        let err = elf::inspect_elf_artifact(&bytes).expect_err("ELF32 class must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse ELF artifact: unsupported class '1'"
        );
    }

    #[test]
    fn elf_rejects_section_table_bounds_overflow() {
        let mut bytes = vec![0u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        write_u64_le(&mut bytes, 40, 1024);
        write_u16_le(&mut bytes, 58, 64);
        write_u16_le(&mut bytes, 60, 1);
        write_u16_le(&mut bytes, 62, 0);
        let err =
            elf::inspect_elf_artifact(&bytes).expect_err("out-of-range section table must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse ELF artifact: section table exceeds artifact size"
        );
    }

    #[test]
    fn elf_rejects_relocation_section_with_bad_symbol_table_link() {
        let mut bytes = reloc_elf_bytes();
        let shoff = u64::from_le_bytes(bytes[40..48].try_into().expect("u64")) as usize;
        let shentsize = u16::from_le_bytes(bytes[58..60].try_into().expect("u16")) as usize;
        let rela_text_section_index = 2usize;
        let sh_link_offset = shoff + rela_text_section_index * shentsize + 40;
        write_u32_le(&mut bytes, sh_link_offset, 99);
        let err =
            elf::inspect_elf_artifact(&bytes).expect_err("invalid relocation sh_link must fail");
        assert_eq!(
            err,
            "inspect-artifact: failed to parse ELF artifact: relocation section '.rela.text' has invalid sh_link"
        );
    }

    #[test]
    fn asm_close_shape_with_unknown_instruction_is_rejected() {
        let err = inspect_artifact_from_bytes(b".text\n.globl entry\nentry:\n    nop\n")
            .expect_err("unsupported asm instruction must fail");
        assert_eq!(err, "inspect-artifact: unsupported artifact bytes");
    }
}
