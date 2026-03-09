use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInspectionReport {
    pub artifact_kind: &'static str,
    pub file_size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer_bits: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endianness: Option<&'static str>,
    pub symbols: Vec<ArtifactInspectionSymbol>,
    pub defined_symbols: Vec<String>,
    pub undefined_symbols: Vec<String>,
    pub relocations: Vec<ArtifactInspectionRelocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asm: Option<ArtifactInspectionAsmSummary>,
    pub flags: ArtifactInspectionFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInspectionSymbol {
    pub name: String,
    pub category: &'static str,
    pub definition: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInspectionRelocation {
    pub section: String,
    #[serde(rename = "type")]
    pub reloc_type: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInspectionAsmSummary {
    pub globals: Vec<String>,
    pub labels: Vec<String>,
    pub direct_call_targets: Vec<String>,
    pub appears_x86_64_text_subset: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInspectionFlags {
    pub has_entry_symbol: bool,
    pub has_undefined_symbols: bool,
    pub has_text_relocations: bool,
}
