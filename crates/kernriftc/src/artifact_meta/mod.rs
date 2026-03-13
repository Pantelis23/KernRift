mod elf;
mod emit;
mod krbo;
mod path;
mod report;

pub(crate) use elf::parse_elf_object_artifact_metadata;
pub(crate) use emit::write_backend_artifact_sidecar;
pub(crate) use krbo::parse_krbo_artifact_metadata;
pub(crate) use report::{
    BACKEND_ARTIFACT_META_SCHEMA_VERSION, ElfObjectArtifactMetadata, KrboArtifactMetadata,
};
