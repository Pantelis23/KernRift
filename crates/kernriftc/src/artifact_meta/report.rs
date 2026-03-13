use serde::Serialize;

pub(crate) const BACKEND_ARTIFACT_META_SCHEMA_VERSION: &str = "kernrift_artifact_meta_v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BackendArtifactMetadata {
    pub(crate) schema_version: &'static str,
    pub(crate) emit_kind: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) byte_len: usize,
    pub(crate) sha256: String,
    pub(crate) input_path: String,
    pub(crate) input_path_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) krbo: Option<KrboArtifactMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) elfobj: Option<ElfObjectArtifactMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KrboArtifactMetadata {
    pub(crate) magic: String,
    pub(crate) version_major: u8,
    pub(crate) version_minor: u8,
    pub(crate) format_revision: u16,
    pub(crate) target_tag: u8,
    pub(crate) target_name: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ElfObjectArtifactMetadata {
    pub(crate) magic: String,
    pub(crate) class: &'static str,
    pub(crate) endianness: &'static str,
    pub(crate) elf_type: &'static str,
    pub(crate) machine: &'static str,
}
