use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

mod eval;
mod parse;
mod profile;

pub(crate) use eval::{
    contracts_bundle_schema_version, evaluate_policy, format_contracts_inspect_summary,
    format_policy_violation, print_policy_violations,
};
pub(crate) use parse::{
    decode_contracts_bundle, load_policy_file, parse_policy_text, validate_json_against_schema_text,
};
pub(crate) use profile::materialize_kernel_profile_policy;

#[cfg(test)]
use eval::policy_violation;
use parse::normalize_policy;
use profile::policy_rule_is_enabled;
#[cfg(test)]
use profile::{
    apply_policy_materialization_action, enabled_policy_rules, kernel_profile_default_rules,
    materialize_kernel_profile_rule,
};

pub(crate) const CONTRACTS_SCHEMA_V1: &str =
    include_str!("../../../../docs/schemas/kernrift_contracts_v1.schema.json");
pub(crate) const CONTRACTS_SCHEMA_V2: &str =
    include_str!("../../../../docs/schemas/kernrift_contracts_v2.schema.json");
pub(crate) const CONTRACTS_SCHEMA_VERSION: &str = "kernrift_contracts_v1";
pub(crate) const CONTRACTS_SCHEMA_VERSION_V2: &str = "kernrift_contracts_v2";

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct PolicyFile {
    #[serde(default)]
    limits: PolicyLimits,
    #[serde(default)]
    locks: PolicyLocks,
    #[serde(default)]
    caps: PolicyCaps,
    #[serde(default)]
    kernel: PolicyKernel,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
struct PolicyLimits {
    #[serde(default)]
    max_lock_depth: Option<u64>,
    #[serde(default)]
    max_no_yield_span: Option<u64>,
    #[serde(default)]
    forbid_unbounded_no_yield: bool,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
struct PolicyLocks {
    #[serde(default)]
    forbid_edges: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
struct PolicyCaps {
    #[serde(default)]
    allow_module: Vec<String>,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
struct PolicyKernel {
    #[serde(default)]
    allow_raw_mmio: Option<bool>,
    #[serde(default)]
    max_raw_mmio_sites: Option<u64>,
    #[serde(default)]
    max_raw_mmio_sites_in_irq: Option<u64>,
    #[serde(default)]
    allow_raw_mmio_symbols: Vec<String>,
    #[serde(default)]
    forbid_raw_mmio_in_irq: bool,
    #[serde(default)]
    forbid_alloc_in_irq: bool,
    #[serde(default)]
    forbid_block_in_irq: bool,
    #[serde(default)]
    forbid_yield_in_irq: bool,
    #[serde(default)]
    forbid_yield_in_critical: bool,
    #[serde(default)]
    forbid_effects_in_critical: Vec<String>,
    #[serde(default)]
    forbid_caps_in_irq: Vec<String>,
    #[serde(default)]
    allow_caps_in_irq: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContractsBundle {
    schema_version: String,
    capabilities: ContractsCapabilities,
    facts: ContractsFacts,
    lockgraph: ContractsLockgraph,
    report: ContractsReport,
}

#[derive(Debug, Deserialize)]
struct ContractsCapabilities {
    module_caps: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ContractsLockgraph {
    edges: Vec<ContractsLockEdge>,
}

#[derive(Debug, Deserialize)]
struct ContractsLockEdge {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct ContractsReport {
    max_lock_depth: u64,
    no_yield_spans: BTreeMap<String, ContractsNoYieldSpan>,
    #[serde(default)]
    effects: ContractsReportEffects,
    #[serde(default)]
    critical: ContractsReportCritical,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsReportEffects {
    #[serde(default)]
    _yield_sites_count: u64,
    #[serde(default)]
    _alloc_sites_count: u64,
    #[serde(default)]
    _block_sites_count: u64,
    #[serde(default)]
    raw_mmio_sites_count: u64,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsFacts {
    symbols: Vec<ContractsFactSymbol>,
}

#[derive(Debug, Deserialize)]
struct ContractsFactSymbol {
    name: String,
    #[serde(default)]
    attrs: ContractsFactAttrs,
    #[serde(default)]
    ctx_reachable: Vec<String>,
    #[serde(default)]
    eff_transitive: Vec<String>,
    #[serde(default)]
    eff_provenance: Vec<ContractsEffectProvenance>,
    #[serde(default)]
    caps_transitive: Vec<String>,
    #[serde(default)]
    caps_provenance: Vec<ContractsCapabilityProvenance>,
    #[serde(default)]
    raw_mmio_used: bool,
    #[serde(default)]
    raw_mmio_sites_count: u64,
}

#[derive(Debug, Deserialize, Default)]
struct ContractsFactAttrs {
    #[serde(default)]
    critical: bool,
}

impl ContractsFactSymbol {
    fn has_ctx_reachable(&self, ctx: &str) -> bool {
        self.ctx_reachable.iter().any(|c| c == ctx)
    }

    fn has_eff_transitive(&self, eff: &str) -> bool {
        self.eff_transitive.iter().any(|e| e == eff)
    }

    fn has_cap_transitive(&self, cap: &str) -> bool {
        self.caps_transitive.iter().any(|c| c == cap)
    }

    fn eff_provenance(&self, eff: &str) -> Option<&ContractsProvenance> {
        self.eff_provenance
            .iter()
            .find(|entry| entry.effect == eff)
            .map(|entry| &entry.provenance)
    }

    fn cap_provenance(&self, cap: &str) -> Option<&ContractsProvenance> {
        self.caps_provenance
            .iter()
            .find(|entry| entry.capability == cap)
            .map(|entry| &entry.provenance)
    }

    fn has_raw_mmio_usage(&self) -> bool {
        self.raw_mmio_used || self.raw_mmio_sites_count > 0
    }
}

#[derive(Debug, Deserialize, Default)]
struct ContractsReportCritical {
    #[serde(default, rename = "depth_max")]
    _depth_max: u64,
    #[serde(default)]
    violations: Vec<ContractsCriticalViolation>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ContractsCriticalViolation {
    function: String,
    effect: String,
    provenance: ContractsProvenance,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct ContractsProvenance {
    #[serde(default)]
    direct: bool,
    #[serde(default)]
    via_callee: Vec<String>,
    #[serde(default)]
    via_extern: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ContractsEffectProvenance {
    effect: String,
    provenance: ContractsProvenance,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ContractsCapabilityProvenance {
    capability: String,
    provenance: ContractsProvenance,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
enum ContractsNoYieldSpan {
    Bounded(u64),
    Unbounded(String),
}

impl ContractsNoYieldSpan {
    fn is_unbounded(&self) -> bool {
        matches!(self, Self::Unbounded(v) if v == "unbounded")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyFamily {
    Context,
    Lock,
    Effect,
    Region,
    Capability,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyRule {
    CapModuleAllowlist,
    KernelCriticalRegionAlloc,
    KernelCriticalRegionBlock,
    KernelCriticalRegionYield,
    KernelIrqAlloc,
    KernelIrqBlock,
    KernelIrqYield,
    KernelIrqCapForbid,
    KernelRawMmioForbid,
    KernelRawMmioSiteLimit,
    KernelRawMmioSymbolAllowlist,
    KernelIrqRawMmioForbid,
    KernelIrqRawMmioSiteLimit,
    KernelPolicyRequiresV2,
    LimitMaxLockDepth,
    LockForbidEdge,
    NoYieldSpanLimit,
    NoYieldUnbounded,
}

const RULE_CAP_MODULE_ALLOWLIST: &str = "CAP_MODULE_ALLOWLIST";
const RULE_KERNEL_CRITICAL_REGION_ALLOC: &str = "KERNEL_CRITICAL_REGION_ALLOC";
const RULE_KERNEL_CRITICAL_REGION_BLOCK: &str = "KERNEL_CRITICAL_REGION_BLOCK";
const RULE_KERNEL_CRITICAL_REGION_YIELD: &str = "KERNEL_CRITICAL_REGION_YIELD";
const RULE_KERNEL_IRQ_ALLOC: &str = "KERNEL_IRQ_ALLOC";
const RULE_KERNEL_IRQ_BLOCK: &str = "KERNEL_IRQ_BLOCK";
const RULE_KERNEL_IRQ_YIELD: &str = "KERNEL_IRQ_YIELD";
const RULE_KERNEL_IRQ_CAP_FORBID: &str = "KERNEL_IRQ_CAP_FORBID";
const RULE_KERNEL_RAW_MMIO_FORBID: &str = "KERNEL_RAW_MMIO_FORBID";
const RULE_KERNEL_RAW_MMIO_SITE_LIMIT: &str = "KERNEL_RAW_MMIO_SITE_LIMIT";
const RULE_KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST: &str = "KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST";
const RULE_KERNEL_IRQ_RAW_MMIO_FORBID: &str = "KERNEL_IRQ_RAW_MMIO_FORBID";
const RULE_KERNEL_IRQ_RAW_MMIO_SITE_LIMIT: &str = "KERNEL_IRQ_RAW_MMIO_SITE_LIMIT";
const RULE_KERNEL_POLICY_REQUIRES_V2: &str = "KERNEL_POLICY_REQUIRES_V2";
const RULE_LIMIT_MAX_LOCK_DEPTH: &str = "LIMIT_MAX_LOCK_DEPTH";
const RULE_LOCK_FORBID_EDGE: &str = "LOCK_FORBID_EDGE";
const RULE_NO_YIELD_SPAN_LIMIT: &str = "NO_YIELD_SPAN_LIMIT";
const RULE_NO_YIELD_UNBOUNDED: &str = "NO_YIELD_UNBOUNDED";
const KERNEL_PROFILE_DEFAULT_MAX_LOCK_DEPTH: u64 = 1;
const KERNEL_PROFILE_DEFAULT_MAX_NO_YIELD_SPAN: u64 = 64;
const KERNEL_PROFILE_DEFAULT_FORBID_EDGES: [(&str, &str); 1] = [("ConsoleLock", "SchedLock")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyMaterializationAction {
    SetAllowRawMmio(bool),
    AppendCriticalEffect(&'static str),
    SetForbidAllocInIrq,
    SetForbidBlockInIrq,
    SetMaxLockDepth(u64),
    SetLockForbidEdges(&'static [(&'static str, &'static str)]),
    SetMaxNoYieldSpan(u64),
    SetForbidUnboundedNoYield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyEnablementProbe {
    KernelRawMmioForbidConfigured,
    KernelRawMmioSiteLimitConfigured,
    KernelRawMmioSymbolAllowlistConfigured,
    KernelForbidRawMmioInIrq,
    KernelIrqRawMmioSiteLimitConfigured,
    CapsAllowModuleNonEmpty,
    KernelCriticalEffectPresent(&'static str),
    KernelForbidYieldInCriticalFlag,
    KernelForbidAllocInIrq,
    KernelForbidBlockInIrq,
    KernelForbidYieldInIrq,
    KernelIrqCapsConfigured,
    LimitMaxLockDepthSet,
    LockForbidEdgesConfigured,
    LimitMaxNoYieldSpanSet,
    LimitForbidUnboundedNoYield,
    KernelV2RulesEnabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyTriggerKind {
    ModuleCapabilityDisallowed,
    CriticalRegionEffectForbidden { effect: &'static str },
    IrqEffectForbidden { effect: &'static str },
    IrqCapabilityForbidden,
    RawMmioForbidden,
    RawMmioSiteCountExceeded,
    RawMmioSymbolNotAllowed,
    IrqRawMmioForbidden,
    IrqRawMmioSiteCountExceeded,
    SchemaCompatibility,
    LockDepthExceeded,
    ForbiddenLockEdgePresent,
    NoYieldSpanExceeded,
    NoYieldUnbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyConditionDescriptor {
    ModuleCapabilityNotAllowed,
    CriticalRegionEffectObserved { effect: &'static str },
    IrqEffectObserved { effect: &'static str },
    IrqCapabilityObserved,
    RawMmioObserved,
    RawMmioSitesCountAboveConfiguredLimit,
    RawMmioSymbolNotAllowed,
    IrqRawMmioObserved,
    IrqRawMmioSitesCountAboveConfiguredLimit,
    SchemaVersionRequiresV2,
    LockDepthAboveConfiguredLimit,
    ForbiddenLockEdgeObserved,
    NoYieldSpanAboveConfiguredLimit,
    NoYieldSpanUnbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyViolationBinderKind {
    SchemaMismatch,
    LockDepth,
    ForbiddenLockEdge,
    NoYieldLimit,
    NoYieldUnbounded,
    IrqEffect,
    RawMmioForbidden,
    RawMmioSiteLimit,
    RawMmioSymbol,
    IrqRawMmioForbidden,
    IrqRawMmioSiteLimit,
    CriticalRegionViolation,
    ModuleCapability,
    IrqCapability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyArtifactDependency {
    PolicySchemaVersion,
    CapabilitiesModuleCaps,
    FactsSymbolsCtxReachable,
    FactsSymbolsEffTransitive,
    FactsSymbolsEffProvenance,
    FactsSymbolsRawMmio,
    FactsSymbolsCapsTransitive,
    FactsSymbolsCapsProvenance,
    ReportCriticalViolations,
    ReportEffectsRawMmioSitesCount,
    ReportMaxLockDepth,
    ReportNoYieldSpans,
    LockgraphEdges,
}

const MATERIALIZE_KERNEL_RAW_MMIO_FORBID: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetAllowRawMmio(false)];
const MATERIALIZE_NONE: &[PolicyMaterializationAction] = &[];
const MATERIALIZE_KERNEL_CRITICAL_REGION_ALLOC: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::AppendCriticalEffect("alloc")];
const MATERIALIZE_KERNEL_CRITICAL_REGION_BLOCK: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::AppendCriticalEffect("block")];
const MATERIALIZE_KERNEL_CRITICAL_REGION_YIELD: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::AppendCriticalEffect("yield")];
const MATERIALIZE_KERNEL_IRQ_ALLOC: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetForbidAllocInIrq];
const MATERIALIZE_KERNEL_IRQ_BLOCK: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetForbidBlockInIrq];
const MATERIALIZE_LIMIT_MAX_LOCK_DEPTH: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetMaxLockDepth(
        KERNEL_PROFILE_DEFAULT_MAX_LOCK_DEPTH,
    )];
const MATERIALIZE_LOCK_FORBID_EDGE: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetLockForbidEdges(
        &KERNEL_PROFILE_DEFAULT_FORBID_EDGES,
    )];
const MATERIALIZE_NO_YIELD_SPAN_LIMIT: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetMaxNoYieldSpan(
        KERNEL_PROFILE_DEFAULT_MAX_NO_YIELD_SPAN,
    )];
const MATERIALIZE_NO_YIELD_UNBOUNDED: &[PolicyMaterializationAction] =
    &[PolicyMaterializationAction::SetForbidUnboundedNoYield];
const ENABLE_IF_KERNEL_RAW_MMIO_FORBID_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelRawMmioForbidConfigured];
const ENABLE_IF_KERNEL_RAW_MMIO_SITE_LIMIT_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelRawMmioSiteLimitConfigured];
const ENABLE_IF_KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelRawMmioSymbolAllowlistConfigured];
const ENABLE_IF_KERNEL_FORBID_RAW_MMIO_IN_IRQ: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelForbidRawMmioInIrq];
const ENABLE_IF_KERNEL_IRQ_RAW_MMIO_SITE_LIMIT_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelIrqRawMmioSiteLimitConfigured];
const ENABLE_IF_CAPS_ALLOW_MODULE_NON_EMPTY: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::CapsAllowModuleNonEmpty];
const ENABLE_IF_KERNEL_CRITICAL_EFFECT_ALLOC: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelCriticalEffectPresent("alloc")];
const ENABLE_IF_KERNEL_CRITICAL_EFFECT_BLOCK: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelCriticalEffectPresent("block")];
const ENABLE_IF_KERNEL_CRITICAL_YIELD: &[PolicyEnablementProbe] = &[
    PolicyEnablementProbe::KernelForbidYieldInCriticalFlag,
    PolicyEnablementProbe::KernelCriticalEffectPresent("yield"),
];
const ENABLE_IF_KERNEL_FORBID_ALLOC_IN_IRQ: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelForbidAllocInIrq];
const ENABLE_IF_KERNEL_FORBID_BLOCK_IN_IRQ: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelForbidBlockInIrq];
const ENABLE_IF_KERNEL_FORBID_YIELD_IN_IRQ: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelForbidYieldInIrq];
const ENABLE_IF_KERNEL_IRQ_CAPS_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelIrqCapsConfigured];
const ENABLE_IF_KERNEL_V2_RULES_ENABLED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::KernelV2RulesEnabled];
const ENABLE_IF_LIMIT_MAX_LOCK_DEPTH_SET: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::LimitMaxLockDepthSet];
const ENABLE_IF_LOCK_FORBID_EDGES_CONFIGURED: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::LockForbidEdgesConfigured];
const ENABLE_IF_LIMIT_MAX_NO_YIELD_SPAN_SET: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::LimitMaxNoYieldSpanSet];
const ENABLE_IF_LIMIT_FORBID_UNBOUNDED_NO_YIELD: &[PolicyEnablementProbe] =
    &[PolicyEnablementProbe::LimitForbidUnboundedNoYield];
const DEPENDS_ON_FACTS_SYMBOLS_RAW_MMIO: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::FactsSymbolsRawMmio];
const DEPENDS_ON_IRQ_RAW_MMIO: &[PolicyArtifactDependency] = &[
    PolicyArtifactDependency::FactsSymbolsCtxReachable,
    PolicyArtifactDependency::FactsSymbolsRawMmio,
];
const DEPENDS_ON_REPORT_EFFECTS_RAW_MMIO_SITES: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::ReportEffectsRawMmioSitesCount];
const DEPENDS_ON_POLICY_SCHEMA_VERSION: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::PolicySchemaVersion];
const DEPENDS_ON_CAPABILITIES_MODULE_CAPS: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::CapabilitiesModuleCaps];
const DEPENDS_ON_IRQ_EFFECT_ALLOC: &[PolicyArtifactDependency] = &[
    PolicyArtifactDependency::FactsSymbolsCtxReachable,
    PolicyArtifactDependency::FactsSymbolsEffTransitive,
    PolicyArtifactDependency::FactsSymbolsEffProvenance,
];
const DEPENDS_ON_IRQ_EFFECT_BLOCK: &[PolicyArtifactDependency] = &[
    PolicyArtifactDependency::FactsSymbolsCtxReachable,
    PolicyArtifactDependency::FactsSymbolsEffTransitive,
    PolicyArtifactDependency::FactsSymbolsEffProvenance,
];
const DEPENDS_ON_IRQ_CAPABILITY: &[PolicyArtifactDependency] = &[
    PolicyArtifactDependency::FactsSymbolsCtxReachable,
    PolicyArtifactDependency::FactsSymbolsCapsTransitive,
    PolicyArtifactDependency::FactsSymbolsCapsProvenance,
];
const DEPENDS_ON_CRITICAL_REGION_EFFECT: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::ReportCriticalViolations];
const DEPENDS_ON_REPORT_MAX_LOCK_DEPTH: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::ReportMaxLockDepth];
const DEPENDS_ON_LOCKGRAPH_EDGES: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::LockgraphEdges];
const DEPENDS_ON_REPORT_NO_YIELD_SPANS: &[PolicyArtifactDependency] =
    &[PolicyArtifactDependency::ReportNoYieldSpans];
const CONDITIONS_MODULE_CAPABILITY_NOT_ALLOWED: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::ModuleCapabilityNotAllowed];
const CONDITIONS_CRITICAL_REGION_ALLOC: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::CriticalRegionEffectObserved { effect: "alloc" }];
const CONDITIONS_CRITICAL_REGION_BLOCK: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::CriticalRegionEffectObserved { effect: "block" }];
const CONDITIONS_CRITICAL_REGION_YIELD: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::CriticalRegionEffectObserved { effect: "yield" }];
const CONDITIONS_IRQ_EFFECT_ALLOC: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqEffectObserved { effect: "alloc" }];
const CONDITIONS_IRQ_EFFECT_BLOCK: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqEffectObserved { effect: "block" }];
const CONDITIONS_IRQ_EFFECT_YIELD: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqEffectObserved { effect: "yield" }];
const CONDITIONS_IRQ_CAPABILITY: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqCapabilityObserved];
const CONDITIONS_RAW_MMIO_OBSERVED: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::RawMmioObserved];
const CONDITIONS_RAW_MMIO_SITE_LIMIT: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit];
const CONDITIONS_RAW_MMIO_SYMBOL_ALLOWLIST: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::RawMmioSymbolNotAllowed];
const CONDITIONS_IRQ_RAW_MMIO_FORBID: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqRawMmioObserved];
const CONDITIONS_IRQ_RAW_MMIO_SITE_LIMIT: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::IrqRawMmioSitesCountAboveConfiguredLimit];
const CONDITIONS_SCHEMA_VERSION_REQUIRES_V2: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::SchemaVersionRequiresV2];
const CONDITIONS_LOCK_DEPTH_ABOVE_LIMIT: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::LockDepthAboveConfiguredLimit];
const CONDITIONS_FORBIDDEN_LOCK_EDGE: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::ForbiddenLockEdgeObserved];
const CONDITIONS_NO_YIELD_SPAN_LIMIT: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::NoYieldSpanAboveConfiguredLimit];
const CONDITIONS_NO_YIELD_SPAN_UNBOUNDED: &[PolicyConditionDescriptor] =
    &[PolicyConditionDescriptor::NoYieldSpanUnbounded];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PolicyRuleSpec {
    rule: PolicyRule,
    code: &'static str,
    family: PolicyFamily,
    sort_rank: u32,
    requires_v2: bool,
    default_enabled_in_profile_kernel: bool,
    diagnostic_template_id: &'static str,
    materialization_actions: &'static [PolicyMaterializationAction],
    enablement_probes: &'static [PolicyEnablementProbe],
    trigger_kind: PolicyTriggerKind,
    artifact_dependencies: &'static [PolicyArtifactDependency],
    condition_descriptors: &'static [PolicyConditionDescriptor],
    binder_kind: PolicyViolationBinderKind,
}

const POLICY_RULE_CATALOG: [PolicyRuleSpec; 18] = [
    PolicyRuleSpec {
        rule: PolicyRule::CapModuleAllowlist,
        code: RULE_CAP_MODULE_ALLOWLIST,
        family: PolicyFamily::Capability,
        sort_rank: 100,
        requires_v2: false,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "cap.module_allowlist",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_CAPS_ALLOW_MODULE_NON_EMPTY,
        trigger_kind: PolicyTriggerKind::ModuleCapabilityDisallowed,
        artifact_dependencies: DEPENDS_ON_CAPABILITIES_MODULE_CAPS,
        condition_descriptors: CONDITIONS_MODULE_CAPABILITY_NOT_ALLOWED,
        binder_kind: PolicyViolationBinderKind::ModuleCapability,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelCriticalRegionAlloc,
        code: RULE_KERNEL_CRITICAL_REGION_ALLOC,
        family: PolicyFamily::Region,
        sort_rank: 101,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.critical_region.alloc",
        materialization_actions: MATERIALIZE_KERNEL_CRITICAL_REGION_ALLOC,
        enablement_probes: ENABLE_IF_KERNEL_CRITICAL_EFFECT_ALLOC,
        trigger_kind: PolicyTriggerKind::CriticalRegionEffectForbidden { effect: "alloc" },
        artifact_dependencies: DEPENDS_ON_CRITICAL_REGION_EFFECT,
        condition_descriptors: CONDITIONS_CRITICAL_REGION_ALLOC,
        binder_kind: PolicyViolationBinderKind::CriticalRegionViolation,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelCriticalRegionBlock,
        code: RULE_KERNEL_CRITICAL_REGION_BLOCK,
        family: PolicyFamily::Region,
        sort_rank: 102,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.critical_region.block",
        materialization_actions: MATERIALIZE_KERNEL_CRITICAL_REGION_BLOCK,
        enablement_probes: ENABLE_IF_KERNEL_CRITICAL_EFFECT_BLOCK,
        trigger_kind: PolicyTriggerKind::CriticalRegionEffectForbidden { effect: "block" },
        artifact_dependencies: DEPENDS_ON_CRITICAL_REGION_EFFECT,
        condition_descriptors: CONDITIONS_CRITICAL_REGION_BLOCK,
        binder_kind: PolicyViolationBinderKind::CriticalRegionViolation,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelCriticalRegionYield,
        code: RULE_KERNEL_CRITICAL_REGION_YIELD,
        family: PolicyFamily::Region,
        sort_rank: 103,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.critical_region.yield",
        materialization_actions: MATERIALIZE_KERNEL_CRITICAL_REGION_YIELD,
        enablement_probes: ENABLE_IF_KERNEL_CRITICAL_YIELD,
        trigger_kind: PolicyTriggerKind::CriticalRegionEffectForbidden { effect: "yield" },
        artifact_dependencies: DEPENDS_ON_CRITICAL_REGION_EFFECT,
        condition_descriptors: CONDITIONS_CRITICAL_REGION_YIELD,
        binder_kind: PolicyViolationBinderKind::CriticalRegionViolation,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqAlloc,
        code: RULE_KERNEL_IRQ_ALLOC,
        family: PolicyFamily::Effect,
        sort_rank: 104,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.irq.alloc",
        materialization_actions: MATERIALIZE_KERNEL_IRQ_ALLOC,
        enablement_probes: ENABLE_IF_KERNEL_FORBID_ALLOC_IN_IRQ,
        trigger_kind: PolicyTriggerKind::IrqEffectForbidden { effect: "alloc" },
        artifact_dependencies: DEPENDS_ON_IRQ_EFFECT_ALLOC,
        condition_descriptors: CONDITIONS_IRQ_EFFECT_ALLOC,
        binder_kind: PolicyViolationBinderKind::IrqEffect,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqBlock,
        code: RULE_KERNEL_IRQ_BLOCK,
        family: PolicyFamily::Effect,
        sort_rank: 105,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.irq.block",
        materialization_actions: MATERIALIZE_KERNEL_IRQ_BLOCK,
        enablement_probes: ENABLE_IF_KERNEL_FORBID_BLOCK_IN_IRQ,
        trigger_kind: PolicyTriggerKind::IrqEffectForbidden { effect: "block" },
        artifact_dependencies: DEPENDS_ON_IRQ_EFFECT_BLOCK,
        condition_descriptors: CONDITIONS_IRQ_EFFECT_BLOCK,
        binder_kind: PolicyViolationBinderKind::IrqEffect,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqYield,
        code: RULE_KERNEL_IRQ_YIELD,
        family: PolicyFamily::Effect,
        sort_rank: 106,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.irq.yield",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_FORBID_YIELD_IN_IRQ,
        trigger_kind: PolicyTriggerKind::IrqEffectForbidden { effect: "yield" },
        artifact_dependencies: DEPENDS_ON_IRQ_EFFECT_ALLOC,
        condition_descriptors: CONDITIONS_IRQ_EFFECT_YIELD,
        binder_kind: PolicyViolationBinderKind::IrqEffect,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqCapForbid,
        code: RULE_KERNEL_IRQ_CAP_FORBID,
        family: PolicyFamily::Capability,
        sort_rank: 107,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.irq.cap_forbid",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_IRQ_CAPS_CONFIGURED,
        trigger_kind: PolicyTriggerKind::IrqCapabilityForbidden,
        artifact_dependencies: DEPENDS_ON_IRQ_CAPABILITY,
        condition_descriptors: CONDITIONS_IRQ_CAPABILITY,
        binder_kind: PolicyViolationBinderKind::IrqCapability,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelRawMmioForbid,
        code: RULE_KERNEL_RAW_MMIO_FORBID,
        family: PolicyFamily::Effect,
        sort_rank: 108,
        requires_v2: true,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.raw_mmio.forbid",
        materialization_actions: MATERIALIZE_KERNEL_RAW_MMIO_FORBID,
        enablement_probes: ENABLE_IF_KERNEL_RAW_MMIO_FORBID_CONFIGURED,
        trigger_kind: PolicyTriggerKind::RawMmioForbidden,
        artifact_dependencies: DEPENDS_ON_FACTS_SYMBOLS_RAW_MMIO,
        condition_descriptors: CONDITIONS_RAW_MMIO_OBSERVED,
        binder_kind: PolicyViolationBinderKind::RawMmioForbidden,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelRawMmioSiteLimit,
        code: RULE_KERNEL_RAW_MMIO_SITE_LIMIT,
        family: PolicyFamily::Effect,
        sort_rank: 109,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.raw_mmio.site_limit",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_RAW_MMIO_SITE_LIMIT_CONFIGURED,
        trigger_kind: PolicyTriggerKind::RawMmioSiteCountExceeded,
        artifact_dependencies: DEPENDS_ON_REPORT_EFFECTS_RAW_MMIO_SITES,
        condition_descriptors: CONDITIONS_RAW_MMIO_SITE_LIMIT,
        binder_kind: PolicyViolationBinderKind::RawMmioSiteLimit,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelRawMmioSymbolAllowlist,
        code: RULE_KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST,
        family: PolicyFamily::Effect,
        sort_rank: 110,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.raw_mmio.symbol_allowlist",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST_CONFIGURED,
        trigger_kind: PolicyTriggerKind::RawMmioSymbolNotAllowed,
        artifact_dependencies: DEPENDS_ON_FACTS_SYMBOLS_RAW_MMIO,
        condition_descriptors: CONDITIONS_RAW_MMIO_SYMBOL_ALLOWLIST,
        binder_kind: PolicyViolationBinderKind::RawMmioSymbol,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqRawMmioForbid,
        code: RULE_KERNEL_IRQ_RAW_MMIO_FORBID,
        family: PolicyFamily::Effect,
        sort_rank: 116,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.irq.raw_mmio.forbid",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_FORBID_RAW_MMIO_IN_IRQ,
        trigger_kind: PolicyTriggerKind::IrqRawMmioForbidden,
        artifact_dependencies: DEPENDS_ON_IRQ_RAW_MMIO,
        condition_descriptors: CONDITIONS_IRQ_RAW_MMIO_FORBID,
        binder_kind: PolicyViolationBinderKind::IrqRawMmioForbidden,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelIrqRawMmioSiteLimit,
        code: RULE_KERNEL_IRQ_RAW_MMIO_SITE_LIMIT,
        family: PolicyFamily::Effect,
        sort_rank: 117,
        requires_v2: true,
        default_enabled_in_profile_kernel: false,
        diagnostic_template_id: "kernel.irq.raw_mmio.site_limit",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_IRQ_RAW_MMIO_SITE_LIMIT_CONFIGURED,
        trigger_kind: PolicyTriggerKind::IrqRawMmioSiteCountExceeded,
        artifact_dependencies: DEPENDS_ON_IRQ_RAW_MMIO,
        condition_descriptors: CONDITIONS_IRQ_RAW_MMIO_SITE_LIMIT,
        binder_kind: PolicyViolationBinderKind::IrqRawMmioSiteLimit,
    },
    PolicyRuleSpec {
        rule: PolicyRule::KernelPolicyRequiresV2,
        code: RULE_KERNEL_POLICY_REQUIRES_V2,
        family: PolicyFamily::Context,
        sort_rank: 111,
        requires_v2: false,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "kernel.requires_v2",
        materialization_actions: MATERIALIZE_NONE,
        enablement_probes: ENABLE_IF_KERNEL_V2_RULES_ENABLED,
        trigger_kind: PolicyTriggerKind::SchemaCompatibility,
        artifact_dependencies: DEPENDS_ON_POLICY_SCHEMA_VERSION,
        condition_descriptors: CONDITIONS_SCHEMA_VERSION_REQUIRES_V2,
        binder_kind: PolicyViolationBinderKind::SchemaMismatch,
    },
    PolicyRuleSpec {
        rule: PolicyRule::LimitMaxLockDepth,
        code: RULE_LIMIT_MAX_LOCK_DEPTH,
        family: PolicyFamily::Limit,
        sort_rank: 112,
        requires_v2: false,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "limit.max_lock_depth",
        materialization_actions: MATERIALIZE_LIMIT_MAX_LOCK_DEPTH,
        enablement_probes: ENABLE_IF_LIMIT_MAX_LOCK_DEPTH_SET,
        trigger_kind: PolicyTriggerKind::LockDepthExceeded,
        artifact_dependencies: DEPENDS_ON_REPORT_MAX_LOCK_DEPTH,
        condition_descriptors: CONDITIONS_LOCK_DEPTH_ABOVE_LIMIT,
        binder_kind: PolicyViolationBinderKind::LockDepth,
    },
    PolicyRuleSpec {
        rule: PolicyRule::LockForbidEdge,
        code: RULE_LOCK_FORBID_EDGE,
        family: PolicyFamily::Lock,
        sort_rank: 113,
        requires_v2: false,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "lock.forbid_edge",
        materialization_actions: MATERIALIZE_LOCK_FORBID_EDGE,
        enablement_probes: ENABLE_IF_LOCK_FORBID_EDGES_CONFIGURED,
        trigger_kind: PolicyTriggerKind::ForbiddenLockEdgePresent,
        artifact_dependencies: DEPENDS_ON_LOCKGRAPH_EDGES,
        condition_descriptors: CONDITIONS_FORBIDDEN_LOCK_EDGE,
        binder_kind: PolicyViolationBinderKind::ForbiddenLockEdge,
    },
    PolicyRuleSpec {
        rule: PolicyRule::NoYieldSpanLimit,
        code: RULE_NO_YIELD_SPAN_LIMIT,
        family: PolicyFamily::Effect,
        sort_rank: 114,
        requires_v2: false,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "no_yield.span_limit",
        materialization_actions: MATERIALIZE_NO_YIELD_SPAN_LIMIT,
        enablement_probes: ENABLE_IF_LIMIT_MAX_NO_YIELD_SPAN_SET,
        trigger_kind: PolicyTriggerKind::NoYieldSpanExceeded,
        artifact_dependencies: DEPENDS_ON_REPORT_NO_YIELD_SPANS,
        condition_descriptors: CONDITIONS_NO_YIELD_SPAN_LIMIT,
        binder_kind: PolicyViolationBinderKind::NoYieldLimit,
    },
    PolicyRuleSpec {
        rule: PolicyRule::NoYieldUnbounded,
        code: RULE_NO_YIELD_UNBOUNDED,
        family: PolicyFamily::Effect,
        sort_rank: 115,
        requires_v2: false,
        default_enabled_in_profile_kernel: true,
        diagnostic_template_id: "no_yield.unbounded",
        materialization_actions: MATERIALIZE_NO_YIELD_UNBOUNDED,
        enablement_probes: ENABLE_IF_LIMIT_FORBID_UNBOUNDED_NO_YIELD,
        trigger_kind: PolicyTriggerKind::NoYieldUnbounded,
        artifact_dependencies: DEPENDS_ON_REPORT_NO_YIELD_SPANS,
        condition_descriptors: CONDITIONS_NO_YIELD_SPAN_UNBOUNDED,
        binder_kind: PolicyViolationBinderKind::NoYieldUnbounded,
    },
];

#[cfg(test)]
const EMITTED_POLICY_RULES: [PolicyRule; 18] = [
    PolicyRule::CapModuleAllowlist,
    PolicyRule::KernelCriticalRegionAlloc,
    PolicyRule::KernelCriticalRegionBlock,
    PolicyRule::KernelCriticalRegionYield,
    PolicyRule::KernelIrqAlloc,
    PolicyRule::KernelIrqBlock,
    PolicyRule::KernelIrqYield,
    PolicyRule::KernelIrqCapForbid,
    PolicyRule::KernelRawMmioForbid,
    PolicyRule::KernelRawMmioSiteLimit,
    PolicyRule::KernelRawMmioSymbolAllowlist,
    PolicyRule::KernelIrqRawMmioForbid,
    PolicyRule::KernelIrqRawMmioSiteLimit,
    PolicyRule::KernelPolicyRequiresV2,
    PolicyRule::LimitMaxLockDepth,
    PolicyRule::LockForbidEdge,
    PolicyRule::NoYieldSpanLimit,
    PolicyRule::NoYieldUnbounded,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyViolation {
    rule: PolicyRule,
    family: PolicyFamily,
    sort_rank: u32,
    requires_v2: bool,
    default_enabled_in_profile_kernel: bool,
    diagnostic_template_id: &'static str,
    code: &'static str,
    msg: String,
    evidence: Vec<String>,
}

impl Ord for PolicyViolation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.sort_rank, self.code, &self.msg)
            .cmp(&(other.sort_rank, other.code, &other.msg))
            .then_with(|| self.family.cmp(&other.family))
            .then_with(|| self.requires_v2.cmp(&other.requires_v2))
            .then_with(|| {
                self.default_enabled_in_profile_kernel
                    .cmp(&other.default_enabled_in_profile_kernel)
            })
            .then_with(|| {
                self.diagnostic_template_id
                    .cmp(other.diagnostic_template_id)
            })
            .then_with(|| self.evidence.cmp(&other.evidence))
            .then_with(|| self.rule.cmp(&other.rule))
    }
}

impl PartialOrd for PolicyViolation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn policy_rule_spec(rule: PolicyRule) -> PolicyRuleSpec {
    POLICY_RULE_CATALOG
        .iter()
        .copied()
        .find(|spec| spec.rule == rule)
        .unwrap_or_else(|| panic!("unknown policy rule '{:?}'", rule))
}

fn policy_family_specs(family: PolicyFamily) -> impl Iterator<Item = PolicyRuleSpec> {
    POLICY_RULE_CATALOG
        .iter()
        .copied()
        .filter(move |spec| spec.family == family)
}

fn policy_rule_conditions(rule: PolicyRule) -> &'static [PolicyConditionDescriptor] {
    policy_rule_spec(rule).condition_descriptors
}

fn policy_rule_binder_kind(rule: PolicyRule) -> PolicyViolationBinderKind {
    policy_rule_spec(rule).binder_kind
}

fn policy_condition_effect(descriptor: PolicyConditionDescriptor) -> Option<&'static str> {
    match descriptor {
        PolicyConditionDescriptor::CriticalRegionEffectObserved { effect }
        | PolicyConditionDescriptor::IrqEffectObserved { effect } => Some(effect),
        _ => None,
    }
}

fn policy_rule_effect_condition(rule: PolicyRule) -> Option<&'static str> {
    policy_rule_conditions(rule)
        .iter()
        .find_map(|descriptor| policy_condition_effect(*descriptor))
}

fn policy_rule_max_lock_depth(policy: &PolicyFile, rule: PolicyRule) -> Option<u64> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::LockDepthAboveConfiguredLimit
            )
        })
        .then_some(policy.limits.max_lock_depth)
        .flatten()
}

fn policy_rule_forbidden_lock_edges(
    policy: &PolicyFile,
    rule: PolicyRule,
) -> Option<&[[String; 2]]> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::ForbiddenLockEdgeObserved
            )
        })
        .then_some(policy.locks.forbid_edges.as_slice())
}

fn policy_rule_max_no_yield_span(policy: &PolicyFile, rule: PolicyRule) -> Option<u64> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::NoYieldSpanAboveConfiguredLimit
            )
        })
        .then_some(policy.limits.max_no_yield_span)
        .flatten()
}

fn policy_rule_module_cap_allowlist(policy: &PolicyFile, rule: PolicyRule) -> Option<&[String]> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::ModuleCapabilityNotAllowed
            )
        })
        .then_some(policy.caps.allow_module.as_slice())
}

fn policy_rule_raw_mmio_allow_global(policy: &PolicyFile, rule: PolicyRule) -> bool {
    policy_rule_conditions(rule).iter().any(|descriptor| {
        matches!(
            descriptor,
            PolicyConditionDescriptor::RawMmioObserved
                | PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit
                | PolicyConditionDescriptor::RawMmioSymbolNotAllowed
        )
    }) && policy.kernel.allow_raw_mmio == Some(true)
}

fn policy_rule_raw_mmio_site_limit(policy: &PolicyFile, rule: PolicyRule) -> Option<u64> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit
            )
        })
        .then_some(policy.kernel.max_raw_mmio_sites)
        .flatten()
}

fn policy_rule_raw_mmio_symbol_allowlist(
    policy: &PolicyFile,
    rule: PolicyRule,
) -> Option<&[String]> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::RawMmioSymbolNotAllowed
            )
        })
        .then_some(policy.kernel.allow_raw_mmio_symbols.as_slice())
}

fn policy_rule_forbid_raw_mmio_in_irq(policy: &PolicyFile, rule: PolicyRule) -> bool {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| matches!(descriptor, PolicyConditionDescriptor::IrqRawMmioObserved))
        && policy.kernel.forbid_raw_mmio_in_irq
}

fn policy_rule_irq_raw_mmio_site_limit(policy: &PolicyFile, rule: PolicyRule) -> Option<u64> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::IrqRawMmioSitesCountAboveConfiguredLimit
            )
        })
        .then_some(policy.kernel.max_raw_mmio_sites_in_irq)
        .flatten()
}

fn policy_rule_irq_capability_lists(
    policy: &PolicyFile,
    rule: PolicyRule,
) -> Option<(BTreeSet<String>, Vec<String>)> {
    policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| matches!(descriptor, PolicyConditionDescriptor::IrqCapabilityObserved))
        .then(|| {
            let allowed = policy
                .kernel
                .allow_caps_in_irq
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut forbidden = policy.kernel.forbid_caps_in_irq.clone();
            forbidden.sort();
            forbidden.dedup();
            (allowed, forbidden)
        })
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyConditionHelperPath {
    SchemaCompatibility,
    LockDepth,
    LockEdges,
    NoYieldLimit,
    NoYieldUnbounded,
    IrqEffect,
    RawMmioForbidden,
    RawMmioSiteLimit,
    RawMmioSymbolAllowlist,
    IrqRawMmioForbidden,
    IrqRawMmioSiteLimit,
    CriticalRegionEffectMap,
    ModuleCapabilityAllowlist,
    IrqCapabilityPrecedence,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PolicyConditionObservationKind {
    SchemaMismatch,
    LockDepth,
    ForbiddenLockEdge,
    NoYieldLimit,
    NoYieldUnbounded,
    IrqEffect,
    RawMmioForbidden,
    RawMmioSiteLimit,
    RawMmioSymbol,
    IrqRawMmioForbidden,
    IrqRawMmioSiteLimit,
    CriticalRegionRule,
    ModuleCapability,
    IrqCapability,
}

#[cfg(test)]
fn policy_condition_helper_path(
    descriptor: PolicyConditionDescriptor,
) -> PolicyConditionHelperPath {
    match descriptor {
        PolicyConditionDescriptor::ModuleCapabilityNotAllowed => {
            PolicyConditionHelperPath::ModuleCapabilityAllowlist
        }
        PolicyConditionDescriptor::CriticalRegionEffectObserved { .. } => {
            PolicyConditionHelperPath::CriticalRegionEffectMap
        }
        PolicyConditionDescriptor::IrqEffectObserved { .. } => PolicyConditionHelperPath::IrqEffect,
        PolicyConditionDescriptor::IrqCapabilityObserved => {
            PolicyConditionHelperPath::IrqCapabilityPrecedence
        }
        PolicyConditionDescriptor::RawMmioObserved => PolicyConditionHelperPath::RawMmioForbidden,
        PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit => {
            PolicyConditionHelperPath::RawMmioSiteLimit
        }
        PolicyConditionDescriptor::RawMmioSymbolNotAllowed => {
            PolicyConditionHelperPath::RawMmioSymbolAllowlist
        }
        PolicyConditionDescriptor::IrqRawMmioObserved => {
            PolicyConditionHelperPath::IrqRawMmioForbidden
        }
        PolicyConditionDescriptor::IrqRawMmioSitesCountAboveConfiguredLimit => {
            PolicyConditionHelperPath::IrqRawMmioSiteLimit
        }
        PolicyConditionDescriptor::SchemaVersionRequiresV2 => {
            PolicyConditionHelperPath::SchemaCompatibility
        }
        PolicyConditionDescriptor::LockDepthAboveConfiguredLimit => {
            PolicyConditionHelperPath::LockDepth
        }
        PolicyConditionDescriptor::ForbiddenLockEdgeObserved => {
            PolicyConditionHelperPath::LockEdges
        }
        PolicyConditionDescriptor::NoYieldSpanAboveConfiguredLimit => {
            PolicyConditionHelperPath::NoYieldLimit
        }
        PolicyConditionDescriptor::NoYieldSpanUnbounded => {
            PolicyConditionHelperPath::NoYieldUnbounded
        }
    }
}

#[cfg(test)]
fn policy_condition_observation_kind(
    descriptor: PolicyConditionDescriptor,
) -> PolicyConditionObservationKind {
    match policy_condition_helper_path(descriptor) {
        PolicyConditionHelperPath::SchemaCompatibility => {
            PolicyConditionObservationKind::SchemaMismatch
        }
        PolicyConditionHelperPath::LockDepth => PolicyConditionObservationKind::LockDepth,
        PolicyConditionHelperPath::LockEdges => PolicyConditionObservationKind::ForbiddenLockEdge,
        PolicyConditionHelperPath::NoYieldLimit => PolicyConditionObservationKind::NoYieldLimit,
        PolicyConditionHelperPath::NoYieldUnbounded => {
            PolicyConditionObservationKind::NoYieldUnbounded
        }
        PolicyConditionHelperPath::IrqEffect => PolicyConditionObservationKind::IrqEffect,
        PolicyConditionHelperPath::RawMmioForbidden => {
            PolicyConditionObservationKind::RawMmioForbidden
        }
        PolicyConditionHelperPath::RawMmioSiteLimit => {
            PolicyConditionObservationKind::RawMmioSiteLimit
        }
        PolicyConditionHelperPath::RawMmioSymbolAllowlist => {
            PolicyConditionObservationKind::RawMmioSymbol
        }
        PolicyConditionHelperPath::IrqRawMmioForbidden => {
            PolicyConditionObservationKind::IrqRawMmioForbidden
        }
        PolicyConditionHelperPath::IrqRawMmioSiteLimit => {
            PolicyConditionObservationKind::IrqRawMmioSiteLimit
        }
        PolicyConditionHelperPath::CriticalRegionEffectMap => {
            PolicyConditionObservationKind::CriticalRegionRule
        }
        PolicyConditionHelperPath::ModuleCapabilityAllowlist => {
            PolicyConditionObservationKind::ModuleCapability
        }
        PolicyConditionHelperPath::IrqCapabilityPrecedence => {
            PolicyConditionObservationKind::IrqCapability
        }
    }
}

#[cfg(test)]
fn policy_observation_binder_kind(
    kind: PolicyConditionObservationKind,
) -> PolicyViolationBinderKind {
    match kind {
        PolicyConditionObservationKind::SchemaMismatch => PolicyViolationBinderKind::SchemaMismatch,
        PolicyConditionObservationKind::LockDepth => PolicyViolationBinderKind::LockDepth,
        PolicyConditionObservationKind::ForbiddenLockEdge => {
            PolicyViolationBinderKind::ForbiddenLockEdge
        }
        PolicyConditionObservationKind::NoYieldLimit => PolicyViolationBinderKind::NoYieldLimit,
        PolicyConditionObservationKind::NoYieldUnbounded => {
            PolicyViolationBinderKind::NoYieldUnbounded
        }
        PolicyConditionObservationKind::IrqEffect => PolicyViolationBinderKind::IrqEffect,
        PolicyConditionObservationKind::RawMmioForbidden => {
            PolicyViolationBinderKind::RawMmioForbidden
        }
        PolicyConditionObservationKind::RawMmioSiteLimit => {
            PolicyViolationBinderKind::RawMmioSiteLimit
        }
        PolicyConditionObservationKind::RawMmioSymbol => PolicyViolationBinderKind::RawMmioSymbol,
        PolicyConditionObservationKind::IrqRawMmioForbidden => {
            PolicyViolationBinderKind::IrqRawMmioForbidden
        }
        PolicyConditionObservationKind::IrqRawMmioSiteLimit => {
            PolicyViolationBinderKind::IrqRawMmioSiteLimit
        }
        PolicyConditionObservationKind::CriticalRegionRule => {
            PolicyViolationBinderKind::CriticalRegionViolation
        }
        PolicyConditionObservationKind::ModuleCapability => {
            PolicyViolationBinderKind::ModuleCapability
        }
        PolicyConditionObservationKind::IrqCapability => PolicyViolationBinderKind::IrqCapability,
    }
}

#[cfg(test)]
fn policy_family_allowed_binder_kinds(
    family: PolicyFamily,
) -> &'static [PolicyViolationBinderKind] {
    match family {
        PolicyFamily::Context => &[PolicyViolationBinderKind::SchemaMismatch],
        PolicyFamily::Limit | PolicyFamily::Lock => &[
            PolicyViolationBinderKind::LockDepth,
            PolicyViolationBinderKind::ForbiddenLockEdge,
        ],
        PolicyFamily::Effect => &[
            PolicyViolationBinderKind::NoYieldLimit,
            PolicyViolationBinderKind::NoYieldUnbounded,
            PolicyViolationBinderKind::IrqEffect,
            PolicyViolationBinderKind::RawMmioForbidden,
            PolicyViolationBinderKind::RawMmioSiteLimit,
            PolicyViolationBinderKind::RawMmioSymbol,
            PolicyViolationBinderKind::IrqRawMmioForbidden,
            PolicyViolationBinderKind::IrqRawMmioSiteLimit,
        ],
        PolicyFamily::Region => &[PolicyViolationBinderKind::CriticalRegionViolation],
        PolicyFamily::Capability => &[
            PolicyViolationBinderKind::ModuleCapability,
            PolicyViolationBinderKind::IrqCapability,
        ],
    }
}

#[cfg(test)]
fn policy_artifact_dependency_is_v2_only(dependency: PolicyArtifactDependency) -> bool {
    matches!(
        dependency,
        PolicyArtifactDependency::FactsSymbolsCtxReachable
            | PolicyArtifactDependency::FactsSymbolsEffTransitive
            | PolicyArtifactDependency::FactsSymbolsEffProvenance
            | PolicyArtifactDependency::FactsSymbolsRawMmio
            | PolicyArtifactDependency::FactsSymbolsCapsTransitive
            | PolicyArtifactDependency::FactsSymbolsCapsProvenance
            | PolicyArtifactDependency::ReportCriticalViolations
            | PolicyArtifactDependency::ReportEffectsRawMmioSitesCount
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn policy_rule_catalog_has_unique_codes_and_ranks() {
        let codes = POLICY_RULE_CATALOG
            .iter()
            .map(|spec| spec.code)
            .collect::<Vec<_>>();
        let ranks = POLICY_RULE_CATALOG
            .iter()
            .map(|spec| spec.sort_rank)
            .collect::<Vec<_>>();
        assert_eq!(codes.len(), codes.iter().collect::<BTreeSet<_>>().len());
        assert_eq!(ranks.len(), ranks.iter().collect::<BTreeSet<_>>().len());
    }

    #[test]
    fn policy_rule_catalog_is_complete_for_emitted_codes() {
        let catalog_rules = POLICY_RULE_CATALOG
            .iter()
            .map(|spec| spec.rule)
            .collect::<BTreeSet<_>>();
        let emitted_rules = EMITTED_POLICY_RULES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            catalog_rules, emitted_rules,
            "policy catalog must contain every emitted rule"
        );
    }

    #[test]
    fn every_emitted_rule_resolves_to_exactly_one_definition() {
        for rule in EMITTED_POLICY_RULES {
            let count = POLICY_RULE_CATALOG
                .iter()
                .filter(|spec| spec.rule == rule)
                .count();
            assert_eq!(count, 1, "rule {:?} must map to one definition", rule);
        }
    }

    #[test]
    fn policy_rule_catalog_entries_match_lookup_function() {
        for spec in POLICY_RULE_CATALOG {
            let looked_up = policy_rule_spec(spec.rule);
            assert_eq!(looked_up.code, spec.code);
            assert_eq!(looked_up.family, spec.family);
            assert_eq!(looked_up.sort_rank, spec.sort_rank);
            assert_eq!(looked_up.requires_v2, spec.requires_v2);
            assert_eq!(
                looked_up.default_enabled_in_profile_kernel,
                spec.default_enabled_in_profile_kernel
            );
            assert_eq!(
                looked_up.diagnostic_template_id,
                spec.diagnostic_template_id
            );
            assert_eq!(
                looked_up.materialization_actions,
                spec.materialization_actions
            );
            assert_eq!(looked_up.enablement_probes, spec.enablement_probes);
            assert_eq!(looked_up.trigger_kind, spec.trigger_kind);
            assert_eq!(looked_up.artifact_dependencies, spec.artifact_dependencies);
            assert_eq!(looked_up.condition_descriptors, spec.condition_descriptors);
            assert_eq!(looked_up.binder_kind, spec.binder_kind);
        }
    }

    #[test]
    fn policy_rule_catalog_marks_v2_required_rules() {
        let v2_required_rules = POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.requires_v2)
            .map(|spec| spec.rule)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            v2_required_rules,
            BTreeSet::from([
                PolicyRule::KernelCriticalRegionAlloc,
                PolicyRule::KernelCriticalRegionBlock,
                PolicyRule::KernelCriticalRegionYield,
                PolicyRule::KernelIrqAlloc,
                PolicyRule::KernelIrqBlock,
                PolicyRule::KernelIrqYield,
                PolicyRule::KernelIrqCapForbid,
                PolicyRule::KernelRawMmioForbid,
                PolicyRule::KernelRawMmioSiteLimit,
                PolicyRule::KernelRawMmioSymbolAllowlist,
                PolicyRule::KernelIrqRawMmioForbid,
                PolicyRule::KernelIrqRawMmioSiteLimit,
            ])
        );
    }

    #[test]
    fn policy_rule_catalog_kernel_profile_defaults_are_expected() {
        let defaults = POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
            .map(|spec| spec.rule)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            defaults,
            BTreeSet::from([
                PolicyRule::KernelCriticalRegionAlloc,
                PolicyRule::KernelCriticalRegionBlock,
                PolicyRule::KernelCriticalRegionYield,
                PolicyRule::KernelIrqAlloc,
                PolicyRule::KernelIrqBlock,
                PolicyRule::KernelRawMmioForbid,
                PolicyRule::KernelPolicyRequiresV2,
                PolicyRule::LimitMaxLockDepth,
                PolicyRule::LockForbidEdge,
                PolicyRule::NoYieldSpanLimit,
                PolicyRule::NoYieldUnbounded,
            ])
        );
    }

    #[test]
    fn policy_rule_catalog_has_unique_diagnostic_template_ids() {
        let mut seen = BTreeMap::<&'static str, &'static str>::new();
        for spec in POLICY_RULE_CATALOG {
            if let Some(existing_code) = seen.insert(spec.diagnostic_template_id, spec.code) {
                panic!(
                    "duplicate diagnostic_template_id '{}' for '{}' and '{}'",
                    spec.diagnostic_template_id, existing_code, spec.code
                );
            }
        }
    }

    #[test]
    fn kernel_profile_default_rules_are_definition_driven_and_unique() {
        let defaults_from_catalog = POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
            .map(|spec| spec.rule)
            .collect::<Vec<_>>();
        let materialized_default_rules = kernel_profile_default_rules();
        assert_eq!(materialized_default_rules, defaults_from_catalog);
        assert_eq!(
            materialized_default_rules.len(),
            materialized_default_rules
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
        );
    }

    #[test]
    fn materialized_kernel_profile_enables_only_default_enabled_rules() {
        let materialized = materialize_kernel_profile_policy()
            .expect("materialized kernel profile must build and normalize");
        let enabled = enabled_policy_rules(&materialized);
        let defaults = POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
            .map(|spec| spec.rule)
            .collect::<BTreeSet<_>>();
        assert_eq!(enabled, defaults);
    }

    #[test]
    fn materialized_kernel_profile_enables_expected_rules_once() {
        let materialized = materialize_kernel_profile_policy()
            .expect("materialized kernel profile must build and normalize");
        let enabled = enabled_policy_rules(&materialized);
        let expected = BTreeSet::from([
            PolicyRule::KernelCriticalRegionAlloc,
            PolicyRule::KernelCriticalRegionBlock,
            PolicyRule::KernelCriticalRegionYield,
            PolicyRule::KernelIrqAlloc,
            PolicyRule::KernelIrqBlock,
            PolicyRule::KernelRawMmioForbid,
            PolicyRule::KernelPolicyRequiresV2,
            PolicyRule::LimitMaxLockDepth,
            PolicyRule::LockForbidEdge,
            PolicyRule::NoYieldSpanLimit,
            PolicyRule::NoYieldUnbounded,
        ]);
        assert_eq!(enabled, expected);
    }

    #[test]
    fn materialized_kernel_profile_no_duplicate_side_effects() {
        let mut duplicated = PolicyFile::default();
        for rule in kernel_profile_default_rules() {
            materialize_kernel_profile_rule(&mut duplicated, rule);
            materialize_kernel_profile_rule(&mut duplicated, rule);
        }
        let duplicated = normalize_policy(duplicated, "<duplicated-materialization>")
            .expect("duplicated materialized policy must normalize");
        let single =
            materialize_kernel_profile_policy().expect("single materialized policy must normalize");
        assert_eq!(duplicated, single);
    }

    #[test]
    fn each_enabled_default_rule_has_single_materialization_effect() {
        let effect_counts = POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
            .map(|spec| (spec.rule, materialization_effect_count(spec.rule)))
            .collect::<BTreeMap<_, _>>();
        for (rule, count) in effect_counts {
            if rule == PolicyRule::KernelPolicyRequiresV2 {
                assert_eq!(count, 0, "KERNEL_POLICY_REQUIRES_V2 is metadata-derived");
            } else {
                assert_eq!(count, 1, "rule {:?} must map to one policy effect", rule);
            }
        }
    }

    #[test]
    fn default_enabled_rules_declare_actions_or_metadata_only_exception() {
        let metadata_only = BTreeSet::from([PolicyRule::KernelPolicyRequiresV2]);
        for spec in POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
        {
            if metadata_only.contains(&spec.rule) {
                assert!(
                    spec.materialization_actions.is_empty(),
                    "metadata-only rule {:?} must declare no actions",
                    spec.rule
                );
            } else {
                assert!(
                    !spec.materialization_actions.is_empty(),
                    "default-enabled rule {:?} must declare actions",
                    spec.rule
                );
            }
        }
    }

    #[test]
    fn materialized_profile_from_declared_actions_matches_default_materialization() {
        let mut from_actions = PolicyFile::default();
        for spec in POLICY_RULE_CATALOG
            .iter()
            .filter(|spec| spec.default_enabled_in_profile_kernel)
        {
            for action in spec.materialization_actions {
                apply_policy_materialization_action(&mut from_actions, *action);
            }
        }
        let from_actions = normalize_policy(from_actions, "<from-actions>")
            .expect("actions policy should normalize");
        let from_rules = materialize_kernel_profile_policy()
            .expect("materialized kernel profile must build and normalize");
        assert_eq!(from_actions, from_rules);
    }

    #[test]
    fn every_rule_declares_at_least_one_enablement_probe() {
        for spec in POLICY_RULE_CATALOG {
            assert!(
                !spec.enablement_probes.is_empty(),
                "rule {:?} must declare enablement probes",
                spec.rule
            );
        }
    }

    #[test]
    fn every_rule_declares_trigger_kind_and_artifact_dependencies() {
        for spec in POLICY_RULE_CATALOG {
            assert!(
                !spec.artifact_dependencies.is_empty(),
                "rule {:?} must declare artifact dependencies",
                spec.rule
            );

            match spec.trigger_kind {
                PolicyTriggerKind::ModuleCapabilityDisallowed => {
                    assert_eq!(
                        spec.artifact_dependencies,
                        DEPENDS_ON_CAPABILITIES_MODULE_CAPS
                    );
                }
                PolicyTriggerKind::CriticalRegionEffectForbidden { .. } => {
                    assert_eq!(
                        spec.artifact_dependencies,
                        DEPENDS_ON_CRITICAL_REGION_EFFECT
                    );
                }
                PolicyTriggerKind::IrqEffectForbidden { .. } => {
                    assert!(
                        spec.artifact_dependencies
                            .contains(&PolicyArtifactDependency::FactsSymbolsCtxReachable)
                    );
                    assert!(
                        spec.artifact_dependencies
                            .contains(&PolicyArtifactDependency::FactsSymbolsEffTransitive)
                    );
                    assert!(
                        spec.artifact_dependencies
                            .contains(&PolicyArtifactDependency::FactsSymbolsEffProvenance)
                    );
                }
                PolicyTriggerKind::IrqCapabilityForbidden => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_IRQ_CAPABILITY);
                }
                PolicyTriggerKind::RawMmioForbidden => {
                    assert_eq!(
                        spec.artifact_dependencies,
                        DEPENDS_ON_FACTS_SYMBOLS_RAW_MMIO
                    );
                }
                PolicyTriggerKind::RawMmioSiteCountExceeded => {
                    assert_eq!(
                        spec.artifact_dependencies,
                        DEPENDS_ON_REPORT_EFFECTS_RAW_MMIO_SITES
                    );
                }
                PolicyTriggerKind::RawMmioSymbolNotAllowed => {
                    assert_eq!(
                        spec.artifact_dependencies,
                        DEPENDS_ON_FACTS_SYMBOLS_RAW_MMIO
                    );
                }
                PolicyTriggerKind::IrqRawMmioForbidden => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_IRQ_RAW_MMIO);
                }
                PolicyTriggerKind::IrqRawMmioSiteCountExceeded => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_IRQ_RAW_MMIO);
                }
                PolicyTriggerKind::SchemaCompatibility => {
                    assert_eq!(
                        spec.rule,
                        PolicyRule::KernelPolicyRequiresV2,
                        "schema compatibility trigger is reserved for metadata-only v2 gating"
                    );
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_POLICY_SCHEMA_VERSION);
                }
                PolicyTriggerKind::LockDepthExceeded => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_REPORT_MAX_LOCK_DEPTH);
                }
                PolicyTriggerKind::ForbiddenLockEdgePresent => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_LOCKGRAPH_EDGES);
                }
                PolicyTriggerKind::NoYieldSpanExceeded | PolicyTriggerKind::NoYieldUnbounded => {
                    assert_eq!(spec.artifact_dependencies, DEPENDS_ON_REPORT_NO_YIELD_SPANS);
                }
            }
        }
    }

    #[test]
    fn every_rule_declares_at_least_one_condition_descriptor() {
        for spec in POLICY_RULE_CATALOG {
            assert!(
                !spec.condition_descriptors.is_empty(),
                "rule {:?} must declare condition descriptors",
                spec.rule
            );
        }
    }

    #[test]
    fn effect_condition_descriptors_match_trigger_metadata() {
        for spec in POLICY_RULE_CATALOG {
            let trigger_effect = match spec.trigger_kind {
                PolicyTriggerKind::CriticalRegionEffectForbidden { effect }
                | PolicyTriggerKind::IrqEffectForbidden { effect } => Some(effect),
                _ => None,
            };
            let condition_effects = spec
                .condition_descriptors
                .iter()
                .filter_map(|descriptor| policy_condition_effect(*descriptor))
                .collect::<Vec<_>>();
            if let Some(effect) = trigger_effect {
                assert_eq!(
                    condition_effects,
                    vec![effect],
                    "effect-bearing rule {:?} must declare exactly one matching effect condition",
                    spec.rule
                );
            } else {
                assert!(
                    condition_effects.is_empty(),
                    "non-effect rule {:?} must not declare effect conditions",
                    spec.rule
                );
            }
        }
    }

    #[test]
    fn no_impossible_descriptor_family_pairings_exist() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                match descriptor {
                    PolicyConditionDescriptor::ModuleCapabilityNotAllowed => {
                        assert_eq!(spec.family, PolicyFamily::Capability);
                    }
                    PolicyConditionDescriptor::CriticalRegionEffectObserved { .. } => {
                        assert_eq!(spec.family, PolicyFamily::Region);
                    }
                    PolicyConditionDescriptor::IrqEffectObserved { .. } => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                    }
                    PolicyConditionDescriptor::IrqCapabilityObserved => {
                        assert_eq!(spec.family, PolicyFamily::Capability);
                    }
                    PolicyConditionDescriptor::RawMmioObserved => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                        assert_eq!(spec.rule, PolicyRule::KernelRawMmioForbid);
                    }
                    PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                        assert_eq!(spec.rule, PolicyRule::KernelRawMmioSiteLimit);
                    }
                    PolicyConditionDescriptor::RawMmioSymbolNotAllowed => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                        assert_eq!(spec.rule, PolicyRule::KernelRawMmioSymbolAllowlist);
                    }
                    PolicyConditionDescriptor::IrqRawMmioObserved => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                        assert_eq!(spec.rule, PolicyRule::KernelIrqRawMmioForbid);
                    }
                    PolicyConditionDescriptor::IrqRawMmioSitesCountAboveConfiguredLimit => {
                        assert_eq!(spec.family, PolicyFamily::Effect);
                        assert_eq!(spec.rule, PolicyRule::KernelIrqRawMmioSiteLimit);
                    }
                    PolicyConditionDescriptor::SchemaVersionRequiresV2 => {
                        assert_eq!(spec.rule, PolicyRule::KernelPolicyRequiresV2);
                        assert_eq!(spec.family, PolicyFamily::Context);
                    }
                    PolicyConditionDescriptor::LockDepthAboveConfiguredLimit => {
                        assert_eq!(spec.rule, PolicyRule::LimitMaxLockDepth);
                    }
                    PolicyConditionDescriptor::ForbiddenLockEdgeObserved => {
                        assert_eq!(spec.rule, PolicyRule::LockForbidEdge);
                    }
                    PolicyConditionDescriptor::NoYieldSpanAboveConfiguredLimit => {
                        assert_eq!(spec.rule, PolicyRule::NoYieldSpanLimit);
                    }
                    PolicyConditionDescriptor::NoYieldSpanUnbounded => {
                        assert_eq!(spec.rule, PolicyRule::NoYieldUnbounded);
                    }
                }
            }
        }
    }

    #[test]
    fn every_condition_descriptor_maps_to_evaluator_helper_path() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                let path = policy_condition_helper_path(*descriptor);
                match path {
                    PolicyConditionHelperPath::SchemaCompatibility
                    | PolicyConditionHelperPath::LockDepth
                    | PolicyConditionHelperPath::LockEdges
                    | PolicyConditionHelperPath::NoYieldLimit
                    | PolicyConditionHelperPath::NoYieldUnbounded
                    | PolicyConditionHelperPath::IrqEffect
                    | PolicyConditionHelperPath::RawMmioForbidden
                    | PolicyConditionHelperPath::RawMmioSiteLimit
                    | PolicyConditionHelperPath::RawMmioSymbolAllowlist
                    | PolicyConditionHelperPath::IrqRawMmioForbidden
                    | PolicyConditionHelperPath::IrqRawMmioSiteLimit
                    | PolicyConditionHelperPath::CriticalRegionEffectMap
                    | PolicyConditionHelperPath::ModuleCapabilityAllowlist
                    | PolicyConditionHelperPath::IrqCapabilityPrecedence => {}
                }
            }
        }
    }

    #[test]
    fn every_condition_descriptor_maps_to_typed_observation_kind() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                let kind = policy_condition_observation_kind(*descriptor);
                match kind {
                    PolicyConditionObservationKind::SchemaMismatch
                    | PolicyConditionObservationKind::LockDepth
                    | PolicyConditionObservationKind::ForbiddenLockEdge
                    | PolicyConditionObservationKind::NoYieldLimit
                    | PolicyConditionObservationKind::NoYieldUnbounded
                    | PolicyConditionObservationKind::IrqEffect
                    | PolicyConditionObservationKind::RawMmioForbidden
                    | PolicyConditionObservationKind::RawMmioSiteLimit
                    | PolicyConditionObservationKind::RawMmioSymbol
                    | PolicyConditionObservationKind::IrqRawMmioForbidden
                    | PolicyConditionObservationKind::IrqRawMmioSiteLimit
                    | PolicyConditionObservationKind::CriticalRegionRule
                    | PolicyConditionObservationKind::ModuleCapability
                    | PolicyConditionObservationKind::IrqCapability => {}
                }
            }
        }
    }

    #[test]
    fn every_rule_declares_one_binder_kind() {
        for spec in POLICY_RULE_CATALOG {
            match spec.binder_kind {
                PolicyViolationBinderKind::SchemaMismatch
                | PolicyViolationBinderKind::LockDepth
                | PolicyViolationBinderKind::ForbiddenLockEdge
                | PolicyViolationBinderKind::NoYieldLimit
                | PolicyViolationBinderKind::NoYieldUnbounded
                | PolicyViolationBinderKind::IrqEffect
                | PolicyViolationBinderKind::RawMmioForbidden
                | PolicyViolationBinderKind::RawMmioSiteLimit
                | PolicyViolationBinderKind::RawMmioSymbol
                | PolicyViolationBinderKind::IrqRawMmioForbidden
                | PolicyViolationBinderKind::IrqRawMmioSiteLimit
                | PolicyViolationBinderKind::CriticalRegionViolation
                | PolicyViolationBinderKind::ModuleCapability
                | PolicyViolationBinderKind::IrqCapability => {}
            }
        }
    }

    #[test]
    fn binder_kind_matches_condition_observation_kind() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                let kind = policy_condition_observation_kind(*descriptor);
                let binder_kind = policy_observation_binder_kind(kind);
                assert_eq!(binder_kind, spec.binder_kind);
            }
        }
    }

    #[test]
    fn effect_condition_descriptors_use_single_evaluator_path() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                if let Some(effect) = policy_condition_effect(*descriptor) {
                    let helper_path = policy_condition_helper_path(*descriptor);
                    match helper_path {
                        PolicyConditionHelperPath::IrqEffect => {
                            assert!(
                                matches!(
                                    spec.trigger_kind,
                                    PolicyTriggerKind::IrqEffectForbidden {
                                        effect: trigger_effect
                                    } if trigger_effect == effect
                                ),
                                "irq effect descriptor for {:?} must match trigger effect",
                                spec.rule
                            );
                        }
                        PolicyConditionHelperPath::CriticalRegionEffectMap => {
                            assert!(
                                matches!(
                                    spec.trigger_kind,
                                    PolicyTriggerKind::CriticalRegionEffectForbidden {
                                        effect: trigger_effect
                                    } if trigger_effect == effect
                                ),
                                "critical effect descriptor for {:?} must match trigger effect",
                                spec.rule
                            );
                        }
                        other => {
                            panic!(
                                "effect descriptor {:?} for {:?} mapped to non-effect helper {:?}",
                                descriptor, spec.rule, other
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn effect_observation_kinds_use_effect_aware_binder_paths() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                if policy_condition_effect(*descriptor).is_some() {
                    let path = policy_observation_binder_kind(policy_condition_observation_kind(
                        *descriptor,
                    ));
                    assert!(
                        matches!(
                            path,
                            PolicyViolationBinderKind::IrqEffect
                                | PolicyViolationBinderKind::CriticalRegionViolation
                        ),
                        "effect descriptor {:?} for {:?} must use an effect-aware binder path",
                        descriptor,
                        spec.rule
                    );
                }
            }
        }
    }

    #[test]
    fn effect_condition_descriptors_use_effect_bearing_observation_kinds() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                if policy_condition_effect(*descriptor).is_some() {
                    let kind = policy_condition_observation_kind(*descriptor);
                    assert!(
                        matches!(
                            kind,
                            PolicyConditionObservationKind::IrqEffect
                                | PolicyConditionObservationKind::CriticalRegionRule
                        ),
                        "effect descriptor {:?} for {:?} must use an effect-bearing observation kind",
                        descriptor,
                        spec.rule
                    );
                }
            }
        }
    }

    #[test]
    fn kernel_policy_requires_v2_uses_schema_helper_path() {
        let spec = policy_rule_spec(PolicyRule::KernelPolicyRequiresV2);
        assert_eq!(
            spec.condition_descriptors,
            CONDITIONS_SCHEMA_VERSION_REQUIRES_V2
        );
        assert_eq!(
            policy_condition_helper_path(PolicyConditionDescriptor::SchemaVersionRequiresV2),
            PolicyConditionHelperPath::SchemaCompatibility
        );
        assert_eq!(
            policy_condition_observation_kind(PolicyConditionDescriptor::SchemaVersionRequiresV2),
            PolicyConditionObservationKind::SchemaMismatch
        );
        assert_eq!(
            policy_observation_binder_kind(PolicyConditionObservationKind::SchemaMismatch),
            PolicyViolationBinderKind::SchemaMismatch
        );
        assert_eq!(spec.binder_kind, PolicyViolationBinderKind::SchemaMismatch);
    }

    #[test]
    fn no_impossible_rule_binder_pairings_exist() {
        for spec in POLICY_RULE_CATALOG {
            match spec.rule {
                PolicyRule::CapModuleAllowlist => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::ModuleCapability
                    )
                }
                PolicyRule::KernelCriticalRegionAlloc
                | PolicyRule::KernelCriticalRegionBlock
                | PolicyRule::KernelCriticalRegionYield => assert_eq!(
                    spec.binder_kind,
                    PolicyViolationBinderKind::CriticalRegionViolation
                ),
                PolicyRule::KernelIrqAlloc
                | PolicyRule::KernelIrqBlock
                | PolicyRule::KernelIrqYield => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::IrqEffect)
                }
                PolicyRule::KernelRawMmioForbid => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::RawMmioForbidden
                    )
                }
                PolicyRule::KernelRawMmioSiteLimit => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::RawMmioSiteLimit
                    )
                }
                PolicyRule::KernelRawMmioSymbolAllowlist => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::RawMmioSymbol)
                }
                PolicyRule::KernelIrqRawMmioForbid => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::IrqRawMmioForbidden
                    )
                }
                PolicyRule::KernelIrqRawMmioSiteLimit => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::IrqRawMmioSiteLimit
                    )
                }
                PolicyRule::KernelIrqCapForbid => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::IrqCapability)
                }
                PolicyRule::KernelPolicyRequiresV2 => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::SchemaMismatch)
                }
                PolicyRule::LimitMaxLockDepth => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::LockDepth)
                }
                PolicyRule::LockForbidEdge => assert_eq!(
                    spec.binder_kind,
                    PolicyViolationBinderKind::ForbiddenLockEdge
                ),
                PolicyRule::NoYieldSpanLimit => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::NoYieldLimit)
                }
                PolicyRule::NoYieldUnbounded => {
                    assert_eq!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::NoYieldUnbounded
                    )
                }
            }
        }
    }

    #[test]
    fn no_impossible_observation_kind_binder_pairings_exist() {
        for spec in POLICY_RULE_CATALOG {
            for descriptor in spec.condition_descriptors {
                let kind = policy_condition_observation_kind(*descriptor);
                let binder_kind = policy_observation_binder_kind(kind);
                match (kind, binder_kind) {
                    (
                        PolicyConditionObservationKind::SchemaMismatch,
                        PolicyViolationBinderKind::SchemaMismatch,
                    )
                    | (
                        PolicyConditionObservationKind::LockDepth,
                        PolicyViolationBinderKind::LockDepth,
                    )
                    | (
                        PolicyConditionObservationKind::ForbiddenLockEdge,
                        PolicyViolationBinderKind::ForbiddenLockEdge,
                    )
                    | (
                        PolicyConditionObservationKind::NoYieldLimit,
                        PolicyViolationBinderKind::NoYieldLimit,
                    )
                    | (
                        PolicyConditionObservationKind::NoYieldUnbounded,
                        PolicyViolationBinderKind::NoYieldUnbounded,
                    )
                    | (
                        PolicyConditionObservationKind::IrqEffect,
                        PolicyViolationBinderKind::IrqEffect,
                    )
                    | (
                        PolicyConditionObservationKind::RawMmioForbidden,
                        PolicyViolationBinderKind::RawMmioForbidden,
                    )
                    | (
                        PolicyConditionObservationKind::RawMmioSiteLimit,
                        PolicyViolationBinderKind::RawMmioSiteLimit,
                    )
                    | (
                        PolicyConditionObservationKind::RawMmioSymbol,
                        PolicyViolationBinderKind::RawMmioSymbol,
                    )
                    | (
                        PolicyConditionObservationKind::IrqRawMmioForbidden,
                        PolicyViolationBinderKind::IrqRawMmioForbidden,
                    )
                    | (
                        PolicyConditionObservationKind::IrqRawMmioSiteLimit,
                        PolicyViolationBinderKind::IrqRawMmioSiteLimit,
                    )
                    | (
                        PolicyConditionObservationKind::CriticalRegionRule,
                        PolicyViolationBinderKind::CriticalRegionViolation,
                    )
                    | (
                        PolicyConditionObservationKind::ModuleCapability,
                        PolicyViolationBinderKind::ModuleCapability,
                    )
                    | (
                        PolicyConditionObservationKind::IrqCapability,
                        PolicyViolationBinderKind::IrqCapability,
                    ) => {}
                    other => panic!(
                        "impossible observation/binder pairing {:?} for {:?}",
                        other, spec.rule
                    ),
                }
            }
        }
    }

    #[test]
    fn family_local_binder_contracts_only_allow_family_binder_kinds() {
        for spec in POLICY_RULE_CATALOG {
            let allowed = policy_family_allowed_binder_kinds(spec.family);
            assert!(
                allowed.contains(&spec.binder_kind),
                "{:?} in family {:?} uses binder {:?} outside its family-local contract",
                spec.rule,
                spec.family,
                spec.binder_kind
            );
        }
    }

    #[test]
    fn no_impossible_family_binder_kind_contract_pairings_exist() {
        for spec in POLICY_RULE_CATALOG {
            match spec.family {
                PolicyFamily::Context => {
                    assert_eq!(spec.binder_kind, PolicyViolationBinderKind::SchemaMismatch)
                }
                PolicyFamily::Limit | PolicyFamily::Lock => assert!(
                    matches!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::LockDepth
                            | PolicyViolationBinderKind::ForbiddenLockEdge
                    ),
                    "{:?} should use a lock/limit binder kind",
                    spec.rule
                ),
                PolicyFamily::Effect => assert!(
                    matches!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::NoYieldLimit
                            | PolicyViolationBinderKind::NoYieldUnbounded
                            | PolicyViolationBinderKind::IrqEffect
                            | PolicyViolationBinderKind::RawMmioForbidden
                            | PolicyViolationBinderKind::RawMmioSiteLimit
                            | PolicyViolationBinderKind::RawMmioSymbol
                            | PolicyViolationBinderKind::IrqRawMmioForbidden
                            | PolicyViolationBinderKind::IrqRawMmioSiteLimit
                    ),
                    "{:?} should use an effect binder kind",
                    spec.rule
                ),
                PolicyFamily::Region => assert_eq!(
                    spec.binder_kind,
                    PolicyViolationBinderKind::CriticalRegionViolation
                ),
                PolicyFamily::Capability => assert!(
                    matches!(
                        spec.binder_kind,
                        PolicyViolationBinderKind::ModuleCapability
                            | PolicyViolationBinderKind::IrqCapability
                    ),
                    "{:?} should use a capability binder kind",
                    spec.rule
                ),
            }
        }
    }

    #[test]
    fn requires_v2_rules_depend_on_v2_only_artifacts() {
        for spec in POLICY_RULE_CATALOG.iter().filter(|spec| spec.requires_v2) {
            assert!(
                spec.artifact_dependencies
                    .iter()
                    .any(|dep| policy_artifact_dependency_is_v2_only(*dep)),
                "v2 rule {:?} must declare at least one v2-only artifact dependency",
                spec.rule
            );
        }
    }

    #[test]
    fn policy_enablement_probes_activate_rule_enablement() {
        for spec in POLICY_RULE_CATALOG {
            let mut policy = PolicyFile::default();
            for probe in spec.enablement_probes {
                satisfy_enablement_probe_for_test(&mut policy, *probe);
            }
            let policy = normalize_policy(policy, "<enablement-probe>")
                .expect("probe policy should normalize");
            assert!(
                policy_rule_is_enabled(&policy, spec.rule),
                "rule {:?} must enable with its declared probes",
                spec.rule
            );
        }
    }

    #[test]
    fn kernel_policy_requires_v2_remains_metadata_derived_enablement() {
        let empty = normalize_policy(PolicyFile::default(), "<empty>")
            .expect("default policy should normalize");
        assert!(
            !policy_rule_is_enabled(&empty, PolicyRule::KernelPolicyRequiresV2),
            "default policy should not enable KERNEL_POLICY_REQUIRES_V2"
        );

        let mut via_v2_rule = PolicyFile::default();
        via_v2_rule.kernel.forbid_alloc_in_irq = true;
        let via_v2_rule = normalize_policy(via_v2_rule, "<via-v2-rule>")
            .expect("kernel rule policy should normalize");
        assert!(
            policy_rule_is_enabled(&via_v2_rule, PolicyRule::KernelPolicyRequiresV2),
            "v2 kernel rule enablement should enable KERNEL_POLICY_REQUIRES_V2"
        );

        let mut via_non_v2_rule = PolicyFile::default();
        via_non_v2_rule.limits.max_lock_depth = Some(1);
        let via_non_v2_rule = normalize_policy(via_non_v2_rule, "<via-non-v2-rule>")
            .expect("non-v2 rule policy should normalize");
        assert!(
            !policy_rule_is_enabled(&via_non_v2_rule, PolicyRule::KernelPolicyRequiresV2),
            "non-v2-only rules should not enable KERNEL_POLICY_REQUIRES_V2"
        );
    }

    #[test]
    fn kernel_policy_requires_v2_remains_metadata_derived_condition() {
        assert_eq!(
            policy_rule_conditions(PolicyRule::KernelPolicyRequiresV2),
            CONDITIONS_SCHEMA_VERSION_REQUIRES_V2,
            "KERNEL_POLICY_REQUIRES_V2 must remain schema-compatibility metadata"
        );
    }

    fn materialization_effect_count(rule: PolicyRule) -> usize {
        let before = normalize_policy(PolicyFile::default(), "<before>")
            .expect("default policy should normalize");
        let mut after = PolicyFile::default();
        materialize_kernel_profile_rule(&mut after, rule);
        let after =
            normalize_policy(after, "<after>").expect("materialized policy should normalize");
        usize::from(before != after)
    }

    fn satisfy_enablement_probe_for_test(policy: &mut PolicyFile, probe: PolicyEnablementProbe) {
        match probe {
            PolicyEnablementProbe::KernelRawMmioForbidConfigured => {
                policy.kernel.allow_raw_mmio = Some(false);
            }
            PolicyEnablementProbe::KernelRawMmioSiteLimitConfigured => {
                policy.kernel.max_raw_mmio_sites = Some(1);
            }
            PolicyEnablementProbe::KernelRawMmioSymbolAllowlistConfigured => {
                policy
                    .kernel
                    .allow_raw_mmio_symbols
                    .push("entry".to_string());
            }
            PolicyEnablementProbe::KernelForbidRawMmioInIrq => {
                policy.kernel.forbid_raw_mmio_in_irq = true;
            }
            PolicyEnablementProbe::KernelIrqRawMmioSiteLimitConfigured => {
                policy.kernel.max_raw_mmio_sites_in_irq = Some(1);
            }
            PolicyEnablementProbe::CapsAllowModuleNonEmpty => {
                policy.caps.allow_module.push("PhysMap".to_string());
            }
            PolicyEnablementProbe::KernelCriticalEffectPresent(effect) => policy
                .kernel
                .forbid_effects_in_critical
                .push(effect.to_string()),
            PolicyEnablementProbe::KernelForbidYieldInCriticalFlag => {
                policy.kernel.forbid_yield_in_critical = true;
            }
            PolicyEnablementProbe::KernelForbidAllocInIrq => {
                policy.kernel.forbid_alloc_in_irq = true;
            }
            PolicyEnablementProbe::KernelForbidBlockInIrq => {
                policy.kernel.forbid_block_in_irq = true;
            }
            PolicyEnablementProbe::KernelForbidYieldInIrq => {
                policy.kernel.forbid_yield_in_irq = true;
            }
            PolicyEnablementProbe::KernelIrqCapsConfigured => {
                policy.kernel.forbid_caps_in_irq.push("PhysMap".to_string());
            }
            PolicyEnablementProbe::LimitMaxLockDepthSet => {
                policy.limits.max_lock_depth = Some(1);
            }
            PolicyEnablementProbe::LockForbidEdgesConfigured => {
                policy
                    .locks
                    .forbid_edges
                    .push(["ConsoleLock".to_string(), "SchedLock".to_string()]);
            }
            PolicyEnablementProbe::LimitMaxNoYieldSpanSet => {
                policy.limits.max_no_yield_span = Some(1);
            }
            PolicyEnablementProbe::LimitForbidUnboundedNoYield => {
                policy.limits.forbid_unbounded_no_yield = true;
            }
            PolicyEnablementProbe::KernelV2RulesEnabled => {
                policy.kernel.forbid_alloc_in_irq = true;
            }
        }
    }

    #[test]
    fn policy_violations_sort_by_rank_then_code_then_message() {
        let mut violations = [
            policy_violation(PolicyRule::LockForbidEdge, "z".to_string()),
            policy_violation(PolicyRule::KernelIrqAlloc, "b".to_string()),
            policy_violation(PolicyRule::KernelIrqAlloc, "a".to_string()),
            policy_violation(PolicyRule::CapModuleAllowlist, "m".to_string()),
            policy_violation(PolicyRule::LimitMaxLockDepth, "x".to_string()),
        ];
        violations.sort();

        let ordered = violations
            .iter()
            .map(|v| (v.sort_rank, v.code, v.msg.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                (100, RULE_CAP_MODULE_ALLOWLIST, "m"),
                (104, RULE_KERNEL_IRQ_ALLOC, "a"),
                (104, RULE_KERNEL_IRQ_ALLOC, "b"),
                (112, RULE_LIMIT_MAX_LOCK_DEPTH, "x"),
                (113, RULE_LOCK_FORBID_EDGE, "z"),
            ]
        );
    }
}
