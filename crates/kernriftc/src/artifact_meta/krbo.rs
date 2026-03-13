use super::report::KrboArtifactMetadata;

pub(crate) fn parse_krbo_artifact_metadata(bytes: &[u8]) -> Result<KrboArtifactMetadata, String> {
    if bytes.len() < 12 {
        return Err("failed to derive krbo metadata: artifact too small".to_string());
    }
    let magic = std::str::from_utf8(&bytes[0..4])
        .map_err(|_| "failed to derive krbo metadata: invalid magic bytes".to_string())?
        .to_string();
    let target_tag = bytes[9];
    let target_name = match target_tag {
        1 => "x86_64-sysv",
        _ => "unknown",
    };

    Ok(KrboArtifactMetadata {
        magic,
        version_major: bytes[4],
        version_minor: bytes[5],
        format_revision: u16::from_le_bytes([bytes[6], bytes[7]]),
        target_tag,
        target_name,
    })
}
