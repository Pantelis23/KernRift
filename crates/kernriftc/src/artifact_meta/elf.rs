use super::report::ElfObjectArtifactMetadata;

pub(crate) fn parse_elf_object_artifact_metadata(
    bytes: &[u8],
) -> Result<ElfObjectArtifactMetadata, String> {
    if bytes.len() < 20 {
        return Err("failed to derive elfobj metadata: artifact too small".to_string());
    }
    if &bytes[0..4] != b"\x7fELF" {
        return Err("failed to derive elfobj metadata: invalid ELF magic".to_string());
    }

    let class = match bytes[4] {
        2 => "elf64",
        _ => "unknown",
    };
    let endianness = match bytes[5] {
        1 => "little",
        _ => "unknown",
    };
    let elf_type = match u16::from_le_bytes([bytes[16], bytes[17]]) {
        1 => "relocatable",
        _ => "unknown",
    };
    let machine = match u16::from_le_bytes([bytes[18], bytes[19]]) {
        62 => "x86_64",
        _ => "unknown",
    };

    Ok(ElfObjectArtifactMetadata {
        magic: "7f454c46".to_string(),
        class,
        endianness,
        elf_type,
        machine,
    })
}
