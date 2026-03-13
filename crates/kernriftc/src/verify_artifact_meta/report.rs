use serde::Serialize;

pub(crate) const VERIFY_ARTIFACT_META_SCHEMA_VERSION: &str = "kernrift_verify_artifact_meta_v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VerifyArtifactMetaReport {
    pub(crate) schema_version: &'static str,
    pub(crate) result: &'static str,
    pub(crate) exit_code: u8,
    pub(crate) message: String,
}
