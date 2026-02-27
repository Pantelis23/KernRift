use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use emit::{
    ContractsSchema as EmitContractsSchema, emit_caps_manifest_json, emit_contracts_json,
    emit_contracts_json_canonical, emit_contracts_json_canonical_with_schema,
    emit_contracts_json_with_schema, emit_krir_json, emit_lockgraph_json, emit_report_json,
};
use jsonschema::JSONSchema;
use kernriftc::{analyze, check_file, check_module, compile_file};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const CONTRACTS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v1.schema.json");
const CONTRACTS_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v2.schema.json");
const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");
const CONTRACTS_SCHEMA_VERSION: &str = "kernrift_contracts_v1";
const CONTRACTS_SCHEMA_VERSION_V2: &str = "kernrift_contracts_v2";
const VERIFY_REPORT_SCHEMA_VERSION: &str = "kernrift_verify_report_v1";
const EXIT_POLICY_VIOLATION: u8 = 1;
const EXIT_INVALID_INPUT: u8 = 2;

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
struct PolicyFile {
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
struct ContractsBundle {
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
    critical: ContractsReportCritical,
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
    FactsSymbolsCapsTransitive,
    FactsSymbolsCapsProvenance,
    ReportCriticalViolations,
    ReportMaxLockDepth,
    ReportNoYieldSpans,
    LockgraphEdges,
}

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

const POLICY_RULE_CATALOG: [PolicyRuleSpec; 13] = [
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
        rule: PolicyRule::KernelPolicyRequiresV2,
        code: RULE_KERNEL_POLICY_REQUIRES_V2,
        family: PolicyFamily::Context,
        sort_rank: 108,
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
        sort_rank: 109,
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
        sort_rank: 110,
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
        sort_rank: 111,
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
        sort_rank: 112,
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
const EMITTED_POLICY_RULES: [PolicyRule; 13] = [
    PolicyRule::CapModuleAllowlist,
    PolicyRule::KernelCriticalRegionAlloc,
    PolicyRule::KernelCriticalRegionBlock,
    PolicyRule::KernelCriticalRegionYield,
    PolicyRule::KernelIrqAlloc,
    PolicyRule::KernelIrqBlock,
    PolicyRule::KernelIrqYield,
    PolicyRule::KernelIrqCapForbid,
    PolicyRule::KernelPolicyRequiresV2,
    PolicyRule::LimitMaxLockDepth,
    PolicyRule::LockForbidEdge,
    PolicyRule::NoYieldSpanLimit,
    PolicyRule::NoYieldUnbounded,
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct PolicyViolation {
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

struct PolicyArgs {
    policy_path: String,
    contracts_path: String,
    evidence: bool,
}

struct InspectArgs {
    contracts_path: String,
}

struct InspectReportArgs {
    report_path: String,
}

impl PartialOrd for PolicyViolation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct CheckArgs {
    path: String,
    profile: Option<CheckProfile>,
    contracts_schema: Option<ContractsSchemaArg>,
    contracts_out: Option<String>,
    policy_path: Option<String>,
    hash_out: Option<String>,
    sign_key_path: Option<String>,
    sig_out: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckProfile {
    Kernel,
}

impl CheckProfile {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "kernel" => Ok(Self::Kernel),
            other => Err(format!(
                "invalid check mode: unknown profile '{}', expected 'kernel'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractsSchemaArg {
    V1,
    V2,
}

impl ContractsSchemaArg {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "v1" => Ok(Self::V1),
            "v2" => Ok(Self::V2),
            other => Err(format!(
                "invalid check mode: unknown contracts schema '{}', expected 'v1' or 'v2'",
                other
            )),
        }
    }

    fn to_emit_schema(self) -> EmitContractsSchema {
        match self {
            Self::V1 => EmitContractsSchema::V1,
            Self::V2 => EmitContractsSchema::V2,
        }
    }
}

#[derive(Debug)]
struct VerifyArgs {
    contracts_path: String,
    hash_path: String,
    sig_path: Option<String>,
    pubkey_path: Option<String>,
    report_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerifyStatus {
    Pass,
    Deny,
    InvalidInput,
}

impl VerifyStatus {
    fn as_exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Deny => EXIT_POLICY_VIOLATION,
            Self::InvalidInput => EXIT_INVALID_INPUT,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Deny => "deny",
            Self::InvalidInput => "invalid_input",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReport {
    schema_version: &'static str,
    result: &'static str,
    inputs: VerifyReportInputs,
    hash: VerifyReportHash,
    contracts: VerifyReportContracts,
    signature: VerifyReportSignature,
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReport {
    schema_version: String,
    result: String,
    inputs: DecodedVerifyReportInputs,
    hash: DecodedVerifyReportHash,
    contracts: DecodedVerifyReportContracts,
    signature: DecodedVerifyReportSignature,
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportInputs {
    contracts: String,
    hash: String,
    sig: Option<String>,
    pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportHash {
    expected_sha256: Option<String>,
    computed_sha256: Option<String>,
    matched: bool,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportContracts {
    utf8_valid: bool,
    schema_valid: bool,
    schema_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DecodedVerifyReportSignature {
    checked: bool,
    valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportInputs {
    contracts: String,
    hash: String,
    sig: Option<String>,
    pubkey: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportHash {
    expected_sha256: Option<String>,
    computed_sha256: Option<String>,
    matched: bool,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportContracts {
    utf8_valid: bool,
    schema_valid: bool,
    schema_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyReportSignature {
    checked: bool,
    valid: Option<bool>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("kernriftc {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(2);
    }

    match args[1].as_str() {
        "check" => match parse_check_args(&args[2..]) {
            Ok(parsed) => run_check(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "verify" => match parse_verify_args(&args[2..]) {
            Ok(parsed) => run_verify(&parsed),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "policy" => match parse_policy_args(&args[2..]) {
            Ok(parsed) => run_policy(&parsed.policy_path, &parsed.contracts_path, parsed.evidence),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "inspect" => match parse_inspect_args(&args[2..]) {
            Ok(parsed) => run_inspect(&parsed.contracts_path),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "inspect-report" => match parse_inspect_report_args(&args[2..]) {
            Ok(parsed) => run_inspect_report(&parsed.report_path),
            Err(err) => {
                eprintln!("{}", err);
                print_usage();
                ExitCode::from(EXIT_INVALID_INPUT)
            }
        },
        "--selftest" => {
            if args.len() != 2 {
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            run_selftest()
        }
        "--emit" => {
            if args.len() >= 3 && args[2] == "report" {
                if args.len() != 6 || args[3] != "--metrics" {
                    eprintln!("invalid emit report mode");
                    print_usage();
                    return ExitCode::from(EXIT_INVALID_INPUT);
                }
                run_report(&args[4], &args[5])
            } else {
                if args.len() != 4 {
                    eprintln!("invalid emit mode");
                    print_usage();
                    return ExitCode::from(EXIT_INVALID_INPUT);
                }
                run_emit(&args[2], &args[3])
            }
        }
        "--report" => {
            if args.len() != 4 {
                eprintln!("invalid report mode");
                print_usage();
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
            run_report(&args[2], &args[3])
        }
        _ => {
            print_usage();
            ExitCode::from(EXIT_INVALID_INPUT)
        }
    }
}

fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut profile = None::<CheckProfile>;
    let mut contracts_schema = None::<ContractsSchemaArg>;
    let mut contracts_out = None::<String>;
    let mut policy_path = None::<String>;
    let mut hash_out = None::<String>;
    let mut sign_key_path = None::<String>;
    let mut sig_out = None::<String>;
    let mut positionals = Vec::<String>::new();

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--profile" => {
                if profile.is_some() {
                    return Err("invalid check mode: duplicate --profile".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --profile requires a value".to_string());
                };
                profile = Some(CheckProfile::parse(value)?);
                idx += 2;
            }
            "--contracts-schema" => {
                if contracts_schema.is_some() {
                    return Err("invalid check mode: duplicate --contracts-schema".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-schema requires a value".to_string()
                    );
                };
                contracts_schema = Some(ContractsSchemaArg::parse(value)?);
                idx += 2;
            }
            "--contracts-out" => {
                if contracts_out.is_some() {
                    return Err("invalid check mode: duplicate --contracts-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --contracts-out requires a file path".to_string()
                    );
                };
                contracts_out = Some(value.clone());
                idx += 2;
            }
            "--policy" => {
                if policy_path.is_some() {
                    return Err("invalid check mode: duplicate --policy".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --policy requires a file path".to_string());
                };
                policy_path = Some(value.clone());
                idx += 2;
            }
            "--hash-out" => {
                if hash_out.is_some() {
                    return Err("invalid check mode: duplicate --hash-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --hash-out requires a file path".to_string());
                };
                hash_out = Some(value.clone());
                idx += 2;
            }
            "--sign-ed25519" => {
                if sign_key_path.is_some() {
                    return Err("invalid check mode: duplicate --sign-ed25519".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "invalid check mode: --sign-ed25519 requires a key file path".to_string(),
                    );
                };
                sign_key_path = Some(value.clone());
                idx += 2;
            }
            "--sig-out" => {
                if sig_out.is_some() {
                    return Err("invalid check mode: duplicate --sig-out".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid check mode: --sig-out requires a file path".to_string());
                };
                sig_out = Some(value.clone());
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("invalid check mode: unknown flag '{}'", other));
            }
            _ => {
                positionals.push(args[idx].clone());
                idx += 1;
            }
        }
    }

    if positionals.len() != 1 {
        return Err("invalid check mode: expected exactly one <file.kr> input".to_string());
    }

    if sign_key_path.is_some() ^ sig_out.is_some() {
        return Err(
            "invalid check mode: --sign-ed25519 and --sig-out must be provided together"
                .to_string(),
        );
    }

    Ok(CheckArgs {
        path: positionals.remove(0),
        profile,
        contracts_schema,
        contracts_out,
        policy_path,
        hash_out,
        sign_key_path,
        sig_out,
    })
}

fn parse_policy_args(args: &[String]) -> Result<PolicyArgs, String> {
    let mut policy_path = None::<String>;
    let mut contracts_path = None::<String>;
    let mut evidence = false;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--policy" => {
                if policy_path.is_some() {
                    return Err("invalid policy mode: duplicate --policy".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid policy mode: --policy requires a file path".to_string());
                }
                policy_path = Some(args[idx].clone());
            }
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid policy mode: duplicate --contracts".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err("invalid policy mode: --contracts requires a file path".to_string());
                }
                contracts_path = Some(args[idx].clone());
            }
            "--evidence" => {
                if evidence {
                    return Err("invalid policy mode: duplicate --evidence".to_string());
                }
                evidence = true;
            }
            _ => {
                return Err(format!(
                    "invalid policy mode: unexpected argument '{}'",
                    args[idx]
                ));
            }
        }
        idx += 1;
    }

    let Some(policy_path) = policy_path else {
        return Err("invalid policy mode: missing --policy".to_string());
    };
    let Some(contracts_path) = contracts_path else {
        return Err("invalid policy mode: missing --contracts".to_string());
    };

    Ok(PolicyArgs {
        policy_path,
        contracts_path,
        evidence,
    })
}

fn parse_inspect_args(args: &[String]) -> Result<InspectArgs, String> {
    let mut contracts_path = None::<String>;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid inspect mode: duplicate --contracts".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect mode: --contracts requires a file path".to_string()
                    );
                }
                contracts_path = Some(args[idx].clone());
            }
            other => {
                return Err(format!(
                    "invalid inspect mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    let Some(contracts_path) = contracts_path else {
        return Err("invalid inspect mode: missing --contracts".to_string());
    };

    Ok(InspectArgs { contracts_path })
}

fn parse_inspect_report_args(args: &[String]) -> Result<InspectReportArgs, String> {
    let mut report_path = None::<String>;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--report" => {
                if report_path.is_some() {
                    return Err("invalid inspect-report mode: duplicate --report".to_string());
                }
                idx += 1;
                if idx >= args.len() {
                    return Err(
                        "invalid inspect-report mode: --report requires a file path".to_string()
                    );
                }
                report_path = Some(args[idx].clone());
            }
            other => {
                return Err(format!(
                    "invalid inspect-report mode: unexpected argument '{}'",
                    other
                ));
            }
        }
        idx += 1;
    }

    let Some(report_path) = report_path else {
        return Err("invalid inspect-report mode: missing --report".to_string());
    };

    Ok(InspectReportArgs { report_path })
}

fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, String> {
    let mut contracts_path = None::<String>;
    let mut hash_path = None::<String>;
    let mut sig_path = None::<String>;
    let mut pubkey_path = None::<String>;
    let mut report_path = None::<String>;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--contracts" => {
                if contracts_path.is_some() {
                    return Err("invalid verify mode: duplicate --contracts".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --contracts requires a file path".to_string());
                };
                contracts_path = Some(value.clone());
                idx += 2;
            }
            "--hash" => {
                if hash_path.is_some() {
                    return Err("invalid verify mode: duplicate --hash".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --hash requires a file path".to_string());
                };
                hash_path = Some(value.clone());
                idx += 2;
            }
            "--sig" => {
                if sig_path.is_some() {
                    return Err("invalid verify mode: duplicate --sig".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --sig requires a file path".to_string());
                };
                sig_path = Some(value.clone());
                idx += 2;
            }
            "--pubkey" => {
                if pubkey_path.is_some() {
                    return Err("invalid verify mode: duplicate --pubkey".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --pubkey requires a file path".to_string());
                };
                pubkey_path = Some(value.clone());
                idx += 2;
            }
            "--report" => {
                if report_path.is_some() {
                    return Err("invalid verify mode: duplicate --report".to_string());
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("invalid verify mode: --report requires a file path".to_string());
                };
                report_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(format!("invalid verify mode: unknown token '{}'", other));
            }
        }
    }

    let Some(contracts_path) = contracts_path else {
        return Err("invalid verify mode: missing --contracts <contracts.json>".to_string());
    };
    let Some(hash_path) = hash_path else {
        return Err("invalid verify mode: missing --hash <contracts.sha256>".to_string());
    };
    if sig_path.is_some() ^ pubkey_path.is_some() {
        return Err(
            "invalid verify mode: --sig and --pubkey must be provided together".to_string(),
        );
    }

    Ok(VerifyArgs {
        contracts_path,
        hash_path,
        sig_path,
        pubkey_path,
        report_path,
    })
}

fn run_check(args: &CheckArgs) -> ExitCode {
    if args.profile.is_none()
        && args.contracts_schema.is_none()
        && args.contracts_out.is_none()
        && args.policy_path.is_none()
        && args.hash_out.is_none()
        && args.sign_key_path.is_none()
        && args.sig_out.is_none()
    {
        return match check_file(Path::new(&args.path)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(errs) => {
                print_errors(&errs);
                ExitCode::from(EXIT_POLICY_VIOLATION)
            }
        };
    }

    let module = match compile_file(Path::new(&args.path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(EXIT_POLICY_VIOLATION);
        }
    };
    let check_errs = match check_module(&module) {
        Ok(()) => Vec::new(),
        Err(errs) => errs,
    };
    let (report, analysis_errs) = analyze(&module);
    let mut semantic_errs = check_errs.clone();
    semantic_errs.extend(analysis_errs.clone());
    semantic_errs.sort();
    semantic_errs.dedup();

    let contracts_schema = match resolve_contracts_schema(args.profile, args.contracts_schema) {
        Ok(schema) => schema,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let contracts =
        match emit_contracts_json_canonical_with_schema(&module, &report, contracts_schema) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
    let contracts_bundle = match decode_contracts_bundle(&contracts, "<generated>") {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let mut policy_violations = Vec::<PolicyViolation>::new();

    if let Some(profile) = args.profile {
        let profile_policy = match load_profile_policy(profile) {
            Ok(policy) => policy,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        policy_violations.extend(evaluate_policy(&profile_policy, &contracts_bundle));
    }

    if let Some(policy_path) = args.policy_path.as_deref() {
        let file_policy = match load_policy_file(policy_path) {
            Ok(policy) => policy,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        policy_violations.extend(evaluate_policy(&file_policy, &contracts_bundle));
    }

    if !semantic_errs.is_empty() {
        print_errors(&semantic_errs);
    }
    if !policy_violations.is_empty() {
        policy_violations.sort();
        policy_violations.dedup();
        print_policy_violations(&policy_violations, false);
    }
    if !semantic_errs.is_empty() || !policy_violations.is_empty() {
        return ExitCode::from(EXIT_POLICY_VIOLATION);
    }

    let hash_hex = sha256_hex(contracts.as_bytes());
    let mut signature_b64 = None::<String>;
    if let Some(key_path) = args.sign_key_path.as_deref() {
        let signing_key = match load_signing_key_hex(key_path) {
            Ok(key) => key,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        let sig = signing_key.sign(contracts.as_bytes());
        signature_b64 = Some(BASE64_STANDARD.encode(sig.to_bytes()));
    }

    let mut outputs = Vec::<(String, String)>::new();
    if let Some(path) = args.contracts_out.as_ref() {
        outputs.push((path.clone(), contracts));
    }
    if let Some(path) = args.hash_out.as_ref() {
        outputs.push((path.clone(), format!("{hash_hex}\n")));
    }
    if let Some(path) = args.sig_out.as_ref() {
        let Some(sig_b64) = signature_b64.as_ref() else {
            eprintln!("internal error: missing signature text for --sig-out");
            return ExitCode::from(EXIT_INVALID_INPUT);
        };
        outputs.push((path.clone(), format!("{sig_b64}\n")));
    }

    if let Err(err) = write_output_files(&outputs) {
        eprintln!("{}", err);
        return ExitCode::from(EXIT_INVALID_INPUT);
    }

    ExitCode::SUCCESS
}

fn run_verify(args: &VerifyArgs) -> ExitCode {
    let mut status = VerifyStatus::Pass;
    let mut report = new_verify_report(args);

    'verify: {
        let contracts_bytes = match fs::read(Path::new(&args.contracts_path)) {
            Ok(bytes) => bytes,
            Err(err) => {
                report.diagnostics.push(format!(
                    "failed to read contracts '{}': {}",
                    args.contracts_path, err
                ));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        let computed_hash = sha256_hex(&contracts_bytes);
        report.hash.computed_sha256 = Some(computed_hash.clone());

        let hash_text = match fs::read_to_string(Path::new(&args.hash_path)) {
            Ok(text) => text,
            Err(err) => {
                report
                    .diagnostics
                    .push(format!("failed to read hash '{}': {}", args.hash_path, err));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        let expected_hash = match normalize_hex(&hash_text, 64, &args.hash_path) {
            Ok(hex) => hex,
            Err(err) => {
                report.diagnostics.push(err);
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.hash.expected_sha256 = Some(expected_hash.clone());
        if computed_hash != expected_hash {
            report.diagnostics.push(format!(
                "verify: HASH_MISMATCH: expected {}, got {}",
                expected_hash, computed_hash
            ));
            status = VerifyStatus::Deny;
            break 'verify;
        }
        report.hash.matched = true;

        let contracts_text = match std::str::from_utf8(&contracts_bytes) {
            Ok(text) => text,
            Err(err) => {
                report.diagnostics.push(format!(
                    "failed to decode contracts '{}' as UTF-8: {}",
                    args.contracts_path, err
                ));
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.contracts.utf8_valid = true;

        let contracts_bundle = match decode_contracts_bundle(contracts_text, &args.contracts_path) {
            Ok(bundle) => bundle,
            Err(err) => {
                report.diagnostics.push(err);
                status = VerifyStatus::InvalidInput;
                break 'verify;
            }
        };
        report.contracts.schema_valid = true;
        report.contracts.schema_version = Some(contracts_bundle.schema_version.clone());

        if let (Some(sig_path), Some(pubkey_path)) =
            (args.sig_path.as_ref(), args.pubkey_path.as_ref())
        {
            report.signature.checked = true;
            let sig_text = match fs::read_to_string(Path::new(sig_path)) {
                Ok(text) => text,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("failed to read signature '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let sig_bytes = match BASE64_STANDARD.decode(sig_text.trim()) {
                Ok(bytes) => bytes,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("invalid base64 signature '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let sig = match Signature::from_slice(&sig_bytes) {
                Ok(sig) => sig,
                Err(err) => {
                    report
                        .diagnostics
                        .push(format!("invalid signature bytes '{}': {}", sig_path, err));
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            let verifying_key = match load_verifying_key_hex(pubkey_path) {
                Ok(key) => key,
                Err(err) => {
                    report.diagnostics.push(err);
                    status = VerifyStatus::InvalidInput;
                    break 'verify;
                }
            };
            if let Err(err) = verifying_key.verify(&contracts_bytes, &sig) {
                report
                    .diagnostics
                    .push(format!("verify: SIG_MISMATCH: {}", err));
                report.signature.valid = Some(false);
                status = VerifyStatus::Deny;
                break 'verify;
            }
            report.signature.valid = Some(true);
        }
    }

    report.result = status.as_str();

    let mut stderr_diagnostics = report.diagnostics.clone();
    stderr_diagnostics.sort();
    stderr_diagnostics.dedup();

    report.diagnostics = stderr_diagnostics
        .iter()
        .map(|diag| normalize_verify_diagnostic_for_report(diag, args))
        .collect();
    report.diagnostics.sort();
    report.diagnostics.dedup();

    if let Some(report_path) = args.report_path.as_ref() {
        let report_json = match emit_verify_report_json(&report) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("{}", err);
                return ExitCode::from(EXIT_INVALID_INPUT);
            }
        };
        if let Err(err) = write_output_files(&[(report_path.clone(), report_json)]) {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    }

    print_errors(&stderr_diagnostics);

    ExitCode::from(status.as_exit_code())
}

fn resolve_contracts_schema(
    profile: Option<CheckProfile>,
    requested: Option<ContractsSchemaArg>,
) -> Result<EmitContractsSchema, String> {
    if profile == Some(CheckProfile::Kernel) {
        if requested == Some(ContractsSchemaArg::V1) {
            return Err(
                "invalid check mode: --profile kernel requires contracts schema v2 (omit --contracts-schema or use --contracts-schema v2)"
                    .to_string(),
            );
        }
        return Ok(EmitContractsSchema::V2);
    }

    Ok(requested.unwrap_or(ContractsSchemaArg::V1).to_emit_schema())
}

fn new_verify_report(args: &VerifyArgs) -> VerifyReport {
    VerifyReport {
        schema_version: VERIFY_REPORT_SCHEMA_VERSION,
        result: VerifyStatus::InvalidInput.as_str(),
        inputs: VerifyReportInputs {
            contracts: stable_display_path(&args.contracts_path),
            hash: stable_display_path(&args.hash_path),
            sig: args.sig_path.as_deref().map(stable_display_path),
            pubkey: args.pubkey_path.as_deref().map(stable_display_path),
        },
        hash: VerifyReportHash {
            expected_sha256: None,
            computed_sha256: None,
            matched: false,
        },
        contracts: VerifyReportContracts {
            utf8_valid: false,
            schema_valid: false,
            schema_version: None,
        },
        signature: VerifyReportSignature {
            checked: false,
            valid: None,
        },
        diagnostics: Vec::new(),
    }
}

fn emit_verify_report_json(report: &VerifyReport) -> Result<String, String> {
    let value = serde_json::to_value(report)
        .map_err(|e| format!("failed to serialize verify report JSON: {}", e))?;
    let canonical = canonicalize_json_value(&value);
    validate_json_against_schema_text(
        &canonical,
        VERIFY_REPORT_SCHEMA_V1,
        "embedded verify report schema",
        "verify report",
    )?;
    serde_json::to_string_pretty(&canonical)
        .map_err(|e| format!("failed to format verify report JSON: {}", e))
}

fn decode_verify_report(
    report_text: &str,
    source_name: &str,
) -> Result<DecodedVerifyReport, String> {
    let report_json: Value = serde_json::from_str(report_text).map_err(|e| {
        format!(
            "failed to parse verify report JSON '{}': {}",
            source_name, e
        )
    })?;
    let schema_version = report_json
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "failed to decode verify report '{}': missing string field 'schema_version'",
                source_name
            )
        })?;
    if schema_version != VERIFY_REPORT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported verify report schema_version '{}', expected '{}'",
            schema_version, VERIFY_REPORT_SCHEMA_VERSION
        ));
    }
    validate_json_against_schema_text(
        &report_json,
        VERIFY_REPORT_SCHEMA_V1,
        "embedded verify report schema",
        "verify report",
    )?;

    serde_json::from_value(report_json).map_err(|e| {
        format!(
            "failed to decode verify report '{}' into inspect model: {}",
            source_name, e
        )
    })
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize_json_value(v)))
                .collect::<BTreeMap<_, _>>();
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json_value).collect()),
        _ => value.clone(),
    }
}

fn stable_display_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let p = Path::new(path);
    if p.is_absolute() {
        p.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| normalized)
    } else {
        normalized
    }
}

fn normalize_verify_diagnostic_for_report(diag: &str, args: &VerifyArgs) -> String {
    let mut out = diag.replace('\\', "/");
    let mut replacements = vec![
        (
            args.contracts_path.as_str(),
            stable_display_path(&args.contracts_path),
        ),
        (
            args.hash_path.as_str(),
            stable_display_path(&args.hash_path),
        ),
    ];
    if let Some(sig) = args.sig_path.as_deref() {
        replacements.push((sig, stable_display_path(sig)));
    }
    if let Some(pubkey) = args.pubkey_path.as_deref() {
        replacements.push((pubkey, stable_display_path(pubkey)));
    }

    for (raw, stable) in replacements {
        let normalized = raw.replace('\\', "/");
        out = out.replace(&normalized, &stable);
    }

    out
}

fn run_policy(policy_path: &str, contracts_path: &str, evidence: bool) -> ExitCode {
    let policy = match load_policy_file(policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let contracts_text = match fs::read_to_string(Path::new(contracts_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read contracts '{}': {}", contracts_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let contracts = match decode_contracts_bundle(&contracts_text, contracts_path) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    let violations = evaluate_policy(&policy, &contracts);
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        print_policy_violations(&violations, evidence);
        ExitCode::from(EXIT_POLICY_VIOLATION)
    }
}

fn run_inspect(contracts_path: &str) -> ExitCode {
    let contracts_text = match fs::read_to_string(Path::new(contracts_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read contracts '{}': {}", contracts_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let contracts = match decode_contracts_bundle(&contracts_text, contracts_path) {
        Ok(bundle) => bundle,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    println!("{}", format_contracts_inspect_summary(&contracts));
    ExitCode::SUCCESS
}

fn run_inspect_report(report_path: &str) -> ExitCode {
    let report_text = match fs::read_to_string(Path::new(report_path)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("failed to read verify report '{}': {}", report_path, err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };
    let report = match decode_verify_report(&report_text, report_path) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::from(EXIT_INVALID_INPUT);
        }
    };

    println!("{}", format_verify_report_inspect_summary(&report));
    ExitCode::SUCCESS
}

fn run_selftest() -> ExitCode {
    let mut failures = Vec::<String>::new();

    if let Err(err) = selftest_fixture_suites() {
        failures.push(err);
    }
    if let Err(err) = selftest_exact_diagnostics() {
        failures.push(err);
    }
    if let Err(err) = selftest_json_schemas() {
        failures.push(err);
    }
    if let Err(err) = selftest_policy_engine() {
        failures.push(err);
    }

    if failures.is_empty() {
        println!("selftest: PASS");
        ExitCode::SUCCESS
    } else {
        for failure in failures {
            eprintln!("{}", failure);
        }
        ExitCode::from(1)
    }
}

fn run_emit(kind: &str, path: &str) -> ExitCode {
    let module = match compile_file(Path::new(path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    match kind {
        "krir" => print_json_result(emit_krir_json(&module), "KRIR JSON"),
        "caps" => print_json_result(emit_caps_manifest_json(&module), "caps manifest JSON"),
        "lockgraph" => {
            let (report, errs) = analyze(&module);
            if !errs.is_empty() {
                print_errors(&errs);
                return ExitCode::from(1);
            }
            print_json_result(emit_lockgraph_json(&report), "lockgraph JSON")
        }
        "contracts" => {
            let (report, errs) = analyze(&module);
            if !errs.is_empty() {
                print_errors(&errs);
                return ExitCode::from(1);
            }
            print_json_result(
                emit_contracts_json(&module, &report),
                "contracts bundle JSON",
            )
        }
        _ => {
            eprintln!("unsupported emit target '{}'", kind);
            print_usage();
            ExitCode::from(2)
        }
    }
}

fn run_report(metrics_csv: &str, path: &str) -> ExitCode {
    let metrics = metrics_csv
        .split(',')
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>();

    if metrics.is_empty() {
        eprintln!("report metric list is empty");
        print_usage();
        return ExitCode::from(2);
    }

    for metric in &metrics {
        if metric != "max_lock_depth" && metric != "no_yield_spans" {
            eprintln!("unsupported report metric '{}'", metric);
            print_usage();
            return ExitCode::from(2);
        }
    }

    let module = match compile_file(Path::new(path)) {
        Ok(module) => module,
        Err(errs) => {
            print_errors(&errs);
            return ExitCode::from(1);
        }
    };

    let (report, errs) = analyze(&module);
    if !errs.is_empty() {
        print_errors(&errs);
        return ExitCode::from(1);
    }

    match emit_report_json(&report, &metrics) {
        Ok(text) => {
            println!("{}", text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::from(1)
        }
    }
}

fn selftest_fixture_suites() -> Result<(), String> {
    let must_pass = collect_kr_files(Path::new("tests").join("must_pass"))?;
    for file in must_pass {
        if let Err(errs) = check_file(&file) {
            return Err(format!(
                "selftest must_pass failed for '{}': {:?}",
                file.display(),
                errs
            ));
        }
    }

    let must_fail = collect_kr_files(Path::new("tests").join("must_fail"))?;
    for file in must_fail {
        match check_file(&file) {
            Ok(()) => {
                return Err(format!(
                    "selftest must_fail expected error for '{}', got success",
                    file.display()
                ));
            }
            Err(errs) if errs.is_empty() => {
                return Err(format!(
                    "selftest must_fail expected non-empty errors for '{}'",
                    file.display()
                ));
            }
            Err(_) => {}
        }
    }

    Ok(())
}

fn selftest_exact_diagnostics() -> Result<(), String> {
    let exact_cases: [(&str, &[&str]); 5] = [
        (
            "tests/must_fail/extern_missing_eff.kr",
            &["extern 'sleep' must declare @eff(...) facts explicitly"],
        ),
        (
            "tests/must_fail/extern_missing_caps.kr",
            &[
                "EXTERN_CAPS_CONTRACT_REQUIRED: extern 'sleep' must declare @caps(...) facts explicitly",
            ],
        ),
        (
            "tests/must_fail/release_mismatch_nested.kr",
            &[
                "lockgraph: function 'nested_release_mismatch' release mismatch: expected 'SchedLock' on top, found 'ConsoleLock'",
            ],
        ),
        (
            "tests/must_fail/yield_hidden_two_levels.kr",
            &["lockgraph: function 'outer' calls yielding callee 'mid' under lock(s): SchedLock"],
        ),
        (
            "tests/must_fail/yield_hidden_in_leaf_wrapper.kr",
            &[
                "lockgraph: function 'outer' calls yielding callee 'wrapper' under lock(s): SchedLock",
            ],
        ),
    ];

    for (fixture, expected) in exact_cases {
        let errs = match check_file(Path::new(fixture)) {
            Ok(()) => return Err(format!("selftest expected fixture '{}' to fail", fixture)),
            Err(errs) => errs,
        };

        let expected_vec = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        if errs != expected_vec {
            return Err(format!(
                "selftest exact diagnostic mismatch for '{}': got {:?}, expected {:?}",
                fixture, errs, expected_vec
            ));
        }
    }

    Ok(())
}

fn selftest_json_schemas() -> Result<(), String> {
    let callee_module =
        compile_file(Path::new("tests/must_pass/callee_acquires_lock.kr")).map_err(join_errs)?;
    let (report, errs) = analyze(&callee_module);
    if !errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for callee_acquires_lock.kr: {:?}",
            errs
        ));
    }

    let lockgraph_text = emit_lockgraph_json(&report).map_err(|e| e.to_string())?;
    let lockgraph: Value = serde_json::from_str(&lockgraph_text).map_err(|e| e.to_string())?;
    if object_keys(&lockgraph)?
        != BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()])
    {
        return Err("selftest lockgraph top-level keys mismatch".to_string());
    }
    let edges = lockgraph["edges"]
        .as_array()
        .ok_or_else(|| "selftest lockgraph edges must be array".to_string())?;
    if edges.len() != 1 {
        return Err("selftest lockgraph expected exactly one edge".to_string());
    }
    let edge_obj = edges[0]
        .as_object()
        .ok_or_else(|| "selftest lockgraph edge must be object".to_string())?;
    let edge_keys = edge_obj.keys().cloned().collect::<BTreeSet<_>>();
    if edge_keys != BTreeSet::from(["from".to_string(), "to".to_string()]) {
        return Err("selftest lockgraph edge keys mismatch".to_string());
    }
    if edge_obj.get("from").and_then(|v| v.as_str()).is_none()
        || edge_obj.get("to").and_then(|v| v.as_str()).is_none()
    {
        return Err("selftest lockgraph edge from/to must be strings".to_string());
    }

    let locks_module = compile_file(Path::new("tests/must_pass/locks_ok.kr")).map_err(join_errs)?;
    let (locks_report, locks_errs) = analyze(&locks_module);
    if !locks_errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for locks_ok.kr: {:?}",
            locks_errs
        ));
    }
    let report_text = emit_report_json(
        &locks_report,
        &["max_lock_depth".to_string(), "no_yield_spans".to_string()],
    )
    .map_err(|e| e.to_string())?;
    let report_json: Value = serde_json::from_str(&report_text).map_err(|e| e.to_string())?;
    if object_keys(&report_json)?
        != BTreeSet::from(["max_lock_depth".to_string(), "no_yield_spans".to_string()])
    {
        return Err("selftest report keys mismatch".to_string());
    }

    let basic_module = compile_file(Path::new("tests/must_pass/basic.kr")).map_err(join_errs)?;
    let caps_text = emit_caps_manifest_json(&basic_module).map_err(|e| e.to_string())?;
    let caps_json: Value = serde_json::from_str(&caps_text).map_err(|e| e.to_string())?;
    if object_keys(&caps_json)?
        != BTreeSet::from(["module_caps".to_string(), "symbols".to_string()])
    {
        return Err("selftest caps manifest keys mismatch".to_string());
    }

    let contracts_text =
        emit_contracts_json(&locks_module, &locks_report).map_err(|e| e.to_string())?;
    let contracts_json: Value = serde_json::from_str(&contracts_text).map_err(|e| e.to_string())?;
    if object_keys(&contracts_json)?
        != BTreeSet::from([
            "capabilities".to_string(),
            "facts".to_string(),
            "lockgraph".to_string(),
            "report".to_string(),
            "schema_version".to_string(),
        ])
    {
        return Err("selftest contracts keys mismatch".to_string());
    }
    if contracts_json["schema_version"].as_str() != Some("kernrift_contracts_v1") {
        return Err("selftest contracts schema_version mismatch".to_string());
    }
    if object_keys(&contracts_json["facts"])? != BTreeSet::from(["symbols".to_string()]) {
        return Err("selftest contracts facts keys mismatch".to_string());
    }
    let fact_symbols = contracts_json["facts"]["symbols"]
        .as_array()
        .ok_or_else(|| "selftest contracts facts symbols must be array".to_string())?;
    let first_fact = fact_symbols
        .first()
        .ok_or_else(|| "selftest contracts facts symbols must not be empty".to_string())?;
    if object_keys(first_fact)?
        != BTreeSet::from([
            "attrs".to_string(),
            "caps_req".to_string(),
            "ctx_ok".to_string(),
            "eff_used".to_string(),
            "is_extern".to_string(),
            "name".to_string(),
        ])
    {
        return Err("selftest contracts fact symbol keys mismatch".to_string());
    }
    if object_keys(&first_fact["attrs"])?
        != BTreeSet::from([
            "hotpath".to_string(),
            "leaf".to_string(),
            "lock_budget".to_string(),
            "noyield".to_string(),
        ])
    {
        return Err("selftest contracts attrs keys mismatch".to_string());
    }
    validate_json_against_schema_text(
        &contracts_json,
        CONTRACTS_SCHEMA_V1,
        "embedded contracts schema",
        "contracts",
    )?;

    let contracts_v2_text =
        emit_contracts_json_with_schema(&locks_module, &locks_report, EmitContractsSchema::V2)
            .map_err(|e| e.to_string())?;
    let contracts_v2_json: Value =
        serde_json::from_str(&contracts_v2_text).map_err(|e| e.to_string())?;
    if contracts_v2_json["schema_version"].as_str() != Some(CONTRACTS_SCHEMA_VERSION_V2) {
        return Err("selftest contracts v2 schema_version mismatch".to_string());
    }
    if object_keys(&contracts_v2_json["report"])?
        != BTreeSet::from([
            "critical".to_string(),
            "effects".to_string(),
            "max_lock_depth".to_string(),
            "no_yield_spans".to_string(),
        ])
    {
        return Err("selftest contracts v2 report keys mismatch".to_string());
    }
    validate_json_against_schema_text(
        &contracts_v2_json,
        CONTRACTS_SCHEMA_V2,
        "embedded contracts schema v2",
        "contracts",
    )?;

    let krir_text = emit_krir_json(&basic_module).map_err(|e| e.to_string())?;
    let krir_json: Value = serde_json::from_str(&krir_text).map_err(|e| e.to_string())?;
    if object_keys(&krir_json)?
        != BTreeSet::from([
            "call_edges".to_string(),
            "functions".to_string(),
            "module_caps".to_string(),
        ])
    {
        return Err("selftest krir keys mismatch".to_string());
    }

    Ok(())
}

fn selftest_policy_engine() -> Result<(), String> {
    let module =
        compile_file(Path::new("tests/must_pass/callee_acquires_lock.kr")).map_err(join_errs)?;
    let (report, errs) = analyze(&module);
    if !errs.is_empty() {
        return Err(format!(
            "selftest analyze failed for policy fixture callee_acquires_lock.kr: {:?}",
            errs
        ));
    }
    let contracts_text =
        emit_contracts_json_canonical(&module, &report).map_err(|e| e.to_string())?;
    let contracts_json: Value = serde_json::from_str(&contracts_text).map_err(|e| e.to_string())?;
    validate_json_against_schema_text(
        &contracts_json,
        CONTRACTS_SCHEMA_V1,
        "embedded contracts schema",
        "contracts",
    )?;
    let contracts: ContractsBundle = serde_json::from_value(contracts_json)
        .map_err(|e| format!("selftest failed to decode contracts bundle: {}", e))?;

    let pass_policy = parse_policy_text(
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]
"#,
        "selftest-pass-policy",
    )?;
    let pass_errs = evaluate_policy(&pass_policy, &contracts);
    if !pass_errs.is_empty() {
        return Err(format!(
            "selftest policy pass case should have no errors, got {:?}",
            pass_errs
        ));
    }

    let fail_policy = parse_policy_text(
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
        "selftest-fail-policy",
    )?;
    let fail_errs = evaluate_policy(&fail_policy, &contracts)
        .iter()
        .map(format_policy_violation)
        .collect::<Vec<_>>();
    let expected = vec![
        "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1".to_string(),
        "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present"
            .to_string(),
    ];
    if fail_errs != expected {
        return Err(format!(
            "selftest policy deterministic error mismatch: got {:?}, expected {:?}",
            fail_errs, expected
        ));
    }

    let no_yield_policy = parse_policy_text(
        r#"
[limits]
forbid_unbounded_no_yield = true
"#,
        "selftest-no-yield-policy",
    )?;
    let no_yield_errs = evaluate_policy(&no_yield_policy, &contracts)
        .iter()
        .map(format_policy_violation)
        .collect::<Vec<_>>();
    let expected_no_yield = vec![
        "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'inner' is unbounded".to_string(),
        "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'outer' is unbounded".to_string(),
    ];
    if no_yield_errs != expected_no_yield {
        return Err(format!(
            "selftest policy no_yield mismatch: got {:?}, expected {:?}",
            no_yield_errs, expected_no_yield
        ));
    }

    Ok(())
}

fn collect_kr_files(dir: impl AsRef<Path>) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = fs::read_dir(dir.as_ref())
        .map_err(|e| format!("failed to read '{}': {}", dir.as_ref().display(), e))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "failed to read entry in '{}': {}",
                dir.as_ref().display(),
                e
            )
        })?
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("kr"))
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}

fn object_keys(value: &Value) -> Result<BTreeSet<String>, String> {
    Ok(value
        .as_object()
        .ok_or_else(|| "expected json object".to_string())?
        .keys()
        .cloned()
        .collect())
}

fn join_errs(errs: Vec<String>) -> String {
    errs.join("; ")
}

fn write_output_files(outputs: &[(String, String)]) -> Result<(), String> {
    if outputs.is_empty() {
        return Ok(());
    }

    let mut final_paths = BTreeSet::<&str>::new();
    for (path, _) in outputs {
        if !final_paths.insert(path.as_str()) {
            return Err(format!("duplicate output path '{}'", path));
        }
        if Path::new(path).exists() {
            return Err(format!(
                "refusing to overwrite existing output '{}'; remove it first",
                path
            ));
        }
    }

    let mut staged = Vec::<(String, String)>::new();
    for (idx, (path, payload)) in outputs.iter().enumerate() {
        let tmp = format!("{}.kernriftc.tmp.{}.{}", path, std::process::id(), idx);
        fs::write(Path::new(&tmp), payload).map_err(|e| {
            cleanup_temp_paths(&staged);
            format!("failed to stage output '{}': {}", path, e)
        })?;
        staged.push((tmp, path.clone()));
    }

    let mut committed = Vec::<String>::new();
    for (tmp, final_path) in &staged {
        if let Err(err) = fs::rename(Path::new(tmp), Path::new(final_path)) {
            cleanup_temp_paths(&staged);
            cleanup_final_paths(&committed);
            return Err(format!(
                "failed to commit output '{}' from '{}': {}",
                final_path, tmp, err
            ));
        }
        committed.push(final_path.clone());
    }

    Ok(())
}

fn cleanup_temp_paths(staged: &[(String, String)]) {
    for (tmp, _) in staged {
        let _ = fs::remove_file(Path::new(tmp));
    }
}

fn cleanup_final_paths(paths: &[String]) {
    for path in paths {
        let _ = fs::remove_file(Path::new(path));
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

fn nibble_to_hex(n: u8) -> char {
    debug_assert!(n < 16);
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

fn normalize_hex(text: &str, expected_len: usize, label: &str) -> Result<String, String> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.len() != expected_len {
        return Err(format!(
            "invalid hex in '{}': expected {} hex chars, got {}",
            label,
            expected_len,
            normalized.len()
        ));
    }
    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "invalid hex in '{}': value contains non-hex characters",
            label
        ));
    }
    Ok(normalized)
}

fn decode_hex_fixed<const N: usize>(text: &str, label: &str) -> Result<[u8; N], String> {
    let normalized = normalize_hex(text, N * 2, label)?;
    let mut out = [0_u8; N];
    let bytes = normalized.as_bytes();
    for i in 0..N {
        let hi = hex_char_to_nibble(bytes[i * 2] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        let lo = hex_char_to_nibble(bytes[i * 2 + 1] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_char_to_nibble(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

fn load_signing_key_hex(path: &str) -> Result<SigningKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read signing key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

fn load_verifying_key_hex(path: &str) -> Result<VerifyingKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read public key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid public key '{}': {}", path, e))
}

fn load_policy_file(policy_path: &str) -> Result<PolicyFile, String> {
    let policy_text = fs::read_to_string(Path::new(policy_path))
        .map_err(|e| format!("failed to read policy '{}': {}", policy_path, e))?;
    parse_policy_text(&policy_text, policy_path)
}

fn load_profile_policy(profile: CheckProfile) -> Result<PolicyFile, String> {
    match profile {
        CheckProfile::Kernel => materialize_kernel_profile_policy(),
    }
}

fn parse_policy_text(text: &str, source_name: &str) -> Result<PolicyFile, String> {
    let policy: PolicyFile = toml::from_str(text)
        .map_err(|e| format!("failed to parse policy '{}': {}", source_name, e))?;
    normalize_policy(policy, source_name)
}

fn normalize_policy(mut policy: PolicyFile, source_name: &str) -> Result<PolicyFile, String> {
    for edge in &mut policy.locks.forbid_edges {
        edge[0] = edge[0].trim().to_string();
        edge[1] = edge[1].trim().to_string();
        if edge[0].is_empty() || edge[1].is_empty() {
            return Err(format!(
                "invalid policy '{}': forbid_edges entries must contain non-empty lock class names",
                source_name
            ));
        }
    }
    policy.locks.forbid_edges.sort();
    policy.locks.forbid_edges.dedup();

    for cap in &mut policy.caps.allow_module {
        *cap = cap.trim().to_string();
        if cap.is_empty() {
            return Err(format!(
                "invalid policy '{}': allow_module entries must be non-empty strings",
                source_name
            ));
        }
    }
    policy.caps.allow_module.sort();
    policy.caps.allow_module.dedup();

    for effect in &mut policy.kernel.forbid_effects_in_critical {
        *effect = effect.trim().to_ascii_lowercase();
        if effect.is_empty() {
            return Err(format!(
                "invalid policy '{}': forbid_effects_in_critical entries must be non-empty strings",
                source_name
            ));
        }
        if effect != "alloc" && effect != "block" && effect != "yield" {
            return Err(format!(
                "invalid policy '{}': unsupported forbid_effects_in_critical value '{}', expected alloc|block|yield",
                source_name, effect
            ));
        }
    }
    policy.kernel.forbid_effects_in_critical.sort();
    policy.kernel.forbid_effects_in_critical.dedup();

    for cap in &mut policy.kernel.forbid_caps_in_irq {
        *cap = cap.trim().to_string();
        if cap.is_empty() {
            return Err(format!(
                "invalid policy '{}': forbid_caps_in_irq entries must be non-empty strings",
                source_name
            ));
        }
    }
    policy.kernel.forbid_caps_in_irq.sort();
    policy.kernel.forbid_caps_in_irq.dedup();

    for cap in &mut policy.kernel.allow_caps_in_irq {
        *cap = cap.trim().to_string();
        if cap.is_empty() {
            return Err(format!(
                "invalid policy '{}': allow_caps_in_irq entries must be non-empty strings",
                source_name
            ));
        }
    }
    policy.kernel.allow_caps_in_irq.sort();
    policy.kernel.allow_caps_in_irq.dedup();

    Ok(policy)
}

fn kernel_profile_default_rules() -> Vec<PolicyRule> {
    let mut rules = POLICY_RULE_CATALOG
        .iter()
        .filter(|spec| spec.default_enabled_in_profile_kernel)
        .map(|spec| spec.rule)
        .collect::<Vec<_>>();
    rules.sort_by_key(|rule| policy_rule_spec(*rule).sort_rank);
    rules.dedup();
    rules
}

fn materialize_kernel_profile_policy() -> Result<PolicyFile, String> {
    let mut policy = PolicyFile::default();
    for rule in kernel_profile_default_rules() {
        materialize_kernel_profile_rule(&mut policy, rule);
    }
    normalize_policy(policy, "<materialized-kernel-profile>")
}

fn materialize_kernel_profile_rule(policy: &mut PolicyFile, rule: PolicyRule) {
    for action in policy_rule_spec(rule).materialization_actions {
        apply_policy_materialization_action(policy, *action);
    }
}

fn apply_policy_materialization_action(
    policy: &mut PolicyFile,
    action: PolicyMaterializationAction,
) {
    match action {
        PolicyMaterializationAction::AppendCriticalEffect(effect) => policy
            .kernel
            .forbid_effects_in_critical
            .push(effect.to_string()),
        PolicyMaterializationAction::SetForbidAllocInIrq => {
            policy.kernel.forbid_alloc_in_irq = true
        }
        PolicyMaterializationAction::SetForbidBlockInIrq => {
            policy.kernel.forbid_block_in_irq = true
        }
        PolicyMaterializationAction::SetMaxLockDepth(limit) => {
            policy.limits.max_lock_depth = Some(limit);
        }
        PolicyMaterializationAction::SetLockForbidEdges(edges) => {
            policy.locks.forbid_edges = edges
                .iter()
                .map(|(from, to)| [(*from).to_string(), (*to).to_string()])
                .collect::<Vec<_>>();
        }
        PolicyMaterializationAction::SetMaxNoYieldSpan(span) => {
            policy.limits.max_no_yield_span = Some(span);
        }
        PolicyMaterializationAction::SetForbidUnboundedNoYield => {
            policy.limits.forbid_unbounded_no_yield = true;
        }
    }
}

fn policy_rule_is_enabled(policy: &PolicyFile, rule: PolicyRule) -> bool {
    let spec = policy_rule_spec(rule);
    spec.enablement_probes
        .iter()
        .any(|probe| policy_enablement_probe_enabled(policy, *probe))
}

fn policy_enablement_probe_enabled(policy: &PolicyFile, probe: PolicyEnablementProbe) -> bool {
    match probe {
        PolicyEnablementProbe::CapsAllowModuleNonEmpty => !policy.caps.allow_module.is_empty(),
        PolicyEnablementProbe::KernelCriticalEffectPresent(effect) => policy
            .kernel
            .forbid_effects_in_critical
            .iter()
            .any(|configured| configured == effect),
        PolicyEnablementProbe::KernelForbidYieldInCriticalFlag => {
            policy.kernel.forbid_yield_in_critical
        }
        PolicyEnablementProbe::KernelForbidAllocInIrq => policy.kernel.forbid_alloc_in_irq,
        PolicyEnablementProbe::KernelForbidBlockInIrq => policy.kernel.forbid_block_in_irq,
        PolicyEnablementProbe::KernelForbidYieldInIrq => policy.kernel.forbid_yield_in_irq,
        PolicyEnablementProbe::KernelIrqCapsConfigured => {
            !policy.kernel.forbid_caps_in_irq.is_empty()
                || !policy.kernel.allow_caps_in_irq.is_empty()
        }
        PolicyEnablementProbe::LimitMaxLockDepthSet => policy.limits.max_lock_depth.is_some(),
        PolicyEnablementProbe::LockForbidEdgesConfigured => !policy.locks.forbid_edges.is_empty(),
        PolicyEnablementProbe::LimitMaxNoYieldSpanSet => policy.limits.max_no_yield_span.is_some(),
        PolicyEnablementProbe::LimitForbidUnboundedNoYield => {
            policy.limits.forbid_unbounded_no_yield
        }
        PolicyEnablementProbe::KernelV2RulesEnabled => kernel_rules_enabled(policy),
    }
}

#[cfg(test)]
fn enabled_policy_rules(policy: &PolicyFile) -> BTreeSet<PolicyRule> {
    POLICY_RULE_CATALOG
        .iter()
        .filter_map(|spec| policy_rule_is_enabled(policy, spec.rule).then_some(spec.rule))
        .collect::<BTreeSet<_>>()
}

fn kernel_v2_rules_enabled(policy: &PolicyFile) -> bool {
    POLICY_RULE_CATALOG
        .iter()
        .filter(|spec| spec.requires_v2)
        .any(|spec| policy_rule_is_enabled(policy, spec.rule))
}

fn decode_contracts_bundle(
    contracts_text: &str,
    source_name: &str,
) -> Result<ContractsBundle, String> {
    let contracts_json: Value = serde_json::from_str(contracts_text)
        .map_err(|e| format!("failed to parse contracts JSON '{}': {}", source_name, e))?;
    let schema_version = contracts_json
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "failed to decode contracts bundle '{}': missing string field 'schema_version'",
                source_name
            )
        })?;
    let (schema_text, schema_name) = match schema_version {
        CONTRACTS_SCHEMA_VERSION => (CONTRACTS_SCHEMA_V1, "embedded contracts schema v1"),
        CONTRACTS_SCHEMA_VERSION_V2 => (CONTRACTS_SCHEMA_V2, "embedded contracts schema v2"),
        other => {
            return Err(format!(
                "unsupported contracts schema_version '{}', expected '{}' or '{}'",
                other, CONTRACTS_SCHEMA_VERSION, CONTRACTS_SCHEMA_VERSION_V2
            ));
        }
    };
    validate_json_against_schema_text(&contracts_json, schema_text, schema_name, "contracts")?;

    let contracts: ContractsBundle = serde_json::from_value(contracts_json).map_err(|e| {
        format!(
            "failed to decode contracts bundle '{}' into policy model: {}",
            source_name, e
        )
    })?;
    if contracts.schema_version != CONTRACTS_SCHEMA_VERSION
        && contracts.schema_version != CONTRACTS_SCHEMA_VERSION_V2
    {
        return Err(format!(
            "unsupported contracts schema_version '{}', expected '{}' or '{}'",
            contracts.schema_version, CONTRACTS_SCHEMA_VERSION, CONTRACTS_SCHEMA_VERSION_V2
        ));
    }

    Ok(contracts)
}

fn evaluate_policy(policy: &PolicyFile, contracts: &ContractsBundle) -> Vec<PolicyViolation> {
    let view = PolicyEvalView::build(contracts);
    let mut violations = Vec::<PolicyViolation>::new();

    let (context_violations, kernel_v2_allowed) = evaluate_context_rules(policy, contracts);
    violations.extend(context_violations);
    violations.extend(evaluate_lock_rules(policy, contracts));
    violations.extend(evaluate_effect_rules(
        policy,
        contracts,
        &view,
        kernel_v2_allowed,
    ));
    violations.extend(evaluate_region_rules(policy, contracts, kernel_v2_allowed));
    violations.extend(evaluate_capability_rules(
        policy,
        contracts,
        &view,
        kernel_v2_allowed,
    ));

    violations.sort();
    violations.dedup();
    violations
}

struct PolicyEvalView<'a> {
    symbol_by_name: BTreeMap<&'a str, &'a ContractsFactSymbol>,
    irq_symbol_names: Vec<&'a str>,
}

impl<'a> PolicyEvalView<'a> {
    fn build(contracts: &'a ContractsBundle) -> Self {
        let symbol_by_name = contracts
            .facts
            .symbols
            .iter()
            .map(|symbol| (symbol.name.as_str(), symbol))
            .collect::<BTreeMap<_, _>>();

        let mut irq_symbol_names = contracts
            .facts
            .symbols
            .iter()
            .filter(|symbol| symbol.has_ctx_reachable("irq"))
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();
        irq_symbol_names.sort();
        irq_symbol_names.dedup();

        Self {
            symbol_by_name,
            irq_symbol_names,
        }
    }
}

fn kernel_rules_enabled(policy: &PolicyFile) -> bool {
    kernel_v2_rules_enabled(policy)
}

fn evaluate_context_rules(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
) -> (Vec<PolicyViolation>, bool) {
    let mut violations = Vec::<PolicyViolation>::new();
    let requires_v2_rule = policy_rule_spec(PolicyRule::KernelPolicyRequiresV2);
    if let Some(observation) =
        policy_rule_requires_v2_schema_mismatch(policy, contracts, requires_v2_rule.rule)
    {
        violations.push(bind_context_rule_violation(
            requires_v2_rule.rule,
            observation,
        ));
        return (violations, false);
    }
    (violations, true)
}

fn evaluate_lock_rules(policy: &PolicyFile, contracts: &ContractsBundle) -> Vec<PolicyViolation> {
    let mut violations = Vec::<PolicyViolation>::new();

    let observed_edges = contracts
        .lockgraph
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect::<BTreeSet<_>>();

    for spec in
        policy_family_specs(PolicyFamily::Limit).chain(policy_family_specs(PolicyFamily::Lock))
    {
        if !policy_rule_is_enabled(policy, spec.rule) {
            continue;
        }
        for descriptor in spec.condition_descriptors {
            match descriptor {
                PolicyConditionDescriptor::LockDepthAboveConfiguredLimit => {
                    if let Some(observation) =
                        policy_rule_lock_depth_violation(policy, contracts, spec.rule)
                    {
                        violations.push(bind_lock_rule_violation(
                            spec.rule,
                            LockRuleObservation::Depth(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::ForbiddenLockEdgeObserved => {
                    for observation in policy_rule_forbidden_lock_edge_violations(
                        policy,
                        contracts,
                        spec.rule,
                        &observed_edges,
                    ) {
                        violations.push(bind_lock_rule_violation(
                            spec.rule,
                            LockRuleObservation::ForbiddenEdge(observation),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    violations
}

fn evaluate_effect_rules(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    view: &PolicyEvalView<'_>,
    kernel_v2_allowed: bool,
) -> Vec<PolicyViolation> {
    let mut violations = Vec::<PolicyViolation>::new();

    for spec in policy_family_specs(PolicyFamily::Effect) {
        if !policy_rule_is_enabled(policy, spec.rule) {
            continue;
        }
        for descriptor in spec.condition_descriptors {
            match descriptor {
                PolicyConditionDescriptor::NoYieldSpanAboveConfiguredLimit => {
                    for observation in
                        policy_rule_no_yield_limit_violations(policy, contracts, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::NoYieldLimit(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::NoYieldSpanUnbounded => {
                    for observation in
                        policy_rule_no_yield_unbounded_violations(policy, contracts, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::NoYieldUnbounded(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::IrqEffectObserved { effect } if kernel_v2_allowed => {
                    for observation in policy_rule_irq_effect_violations(view, spec.rule) {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::IrqEffect {
                                effect,
                                observation,
                            },
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    violations
}

fn evaluate_region_rules(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    kernel_v2_allowed: bool,
) -> Vec<PolicyViolation> {
    let mut violations = Vec::<PolicyViolation>::new();
    if !kernel_v2_allowed {
        return violations;
    }

    let rules_by_effect = policy_region_rule_observations()
        .into_iter()
        .map(|observation| (observation.effect, observation.rule))
        .collect::<BTreeMap<_, _>>();

    for violation in &contracts.report.critical.violations {
        if let Some(rule) = rules_by_effect.get(violation.effect.as_str())
            && policy_rule_is_enabled(policy, *rule)
        {
            violations.push(bind_region_rule_violation(*rule, violation));
        }
    }

    violations
}

fn evaluate_capability_rules(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    view: &PolicyEvalView<'_>,
    kernel_v2_allowed: bool,
) -> Vec<PolicyViolation> {
    let mut violations = Vec::<PolicyViolation>::new();

    for observation in policy_rule_disallowed_module_capabilities(
        policy,
        contracts,
        PolicyRule::CapModuleAllowlist,
    ) {
        if policy_rule_is_enabled(policy, PolicyRule::CapModuleAllowlist) {
            violations.push(bind_capability_rule_violation(
                PolicyRule::CapModuleAllowlist,
                CapabilityRuleObservation::ModuleCapability(observation),
            ));
        }
    }

    if kernel_v2_allowed && policy_rule_is_enabled(policy, PolicyRule::KernelIrqCapForbid) {
        for observation in
            policy_rule_irq_capability_violations(policy, view, PolicyRule::KernelIrqCapForbid)
        {
            violations.push(bind_capability_rule_violation(
                PolicyRule::KernelIrqCapForbid,
                CapabilityRuleObservation::IrqCapability(observation),
            ));
        }
    }

    violations
}

fn format_provenance(provenance: &ContractsProvenance) -> String {
    let mut via_callee = provenance.via_callee.clone();
    via_callee.sort();
    via_callee.dedup();
    let mut via_extern = provenance.via_extern.clone();
    via_extern.sort();
    via_extern.dedup();
    format!(
        "direct={}, via_callee=[{}], via_extern=[{}]",
        provenance.direct,
        via_callee.join(","),
        via_extern.join(",")
    )
}

fn format_optional_provenance(provenance: Option<&ContractsProvenance>) -> String {
    provenance
        .map(format_provenance)
        .unwrap_or_else(|| "direct=false, via_callee=[], via_extern=[]".to_string())
}

fn canonicalize_provenance_fields(
    provenance: Option<&ContractsProvenance>,
) -> (bool, Vec<String>, Vec<String>) {
    let mut via_callee = provenance.map(|p| p.via_callee.clone()).unwrap_or_default();
    via_callee.sort();
    via_callee.dedup();

    let mut via_extern = provenance.map(|p| p.via_extern.clone()).unwrap_or_default();
    via_extern.sort();
    via_extern.dedup();

    (
        provenance.map(|p| p.direct).unwrap_or(false),
        via_callee,
        via_extern,
    )
}

fn evidence_line(key: &str, value: String) -> String {
    format!("evidence: {}={}", key, value)
}

fn evidence_lines_irq_effect(
    symbol_name: &str,
    effect: &str,
    provenance: Option<&ContractsProvenance>,
) -> Vec<String> {
    let (direct, via_callee, via_extern) = canonicalize_provenance_fields(provenance);
    vec![
        evidence_line("symbol", symbol_name.to_string()),
        evidence_line("effect", effect.to_string()),
        evidence_line("direct", direct.to_string()),
        evidence_line("via_callee", format!("[{}]", via_callee.join(","))),
        evidence_line("via_extern", format!("[{}]", via_extern.join(","))),
    ]
}

fn evidence_lines_irq_capability(
    symbol_name: &str,
    capability: &str,
    provenance: Option<&ContractsProvenance>,
) -> Vec<String> {
    let (direct, via_callee, via_extern) = canonicalize_provenance_fields(provenance);
    vec![
        evidence_line("symbol", symbol_name.to_string()),
        evidence_line("capability", capability.to_string()),
        evidence_line("direct", direct.to_string()),
        evidence_line("via_callee", format!("[{}]", via_callee.join(","))),
        evidence_line("via_extern", format!("[{}]", via_extern.join(","))),
    ]
}

fn evidence_lines_critical_region(
    function: &str,
    effect: &str,
    provenance: &ContractsProvenance,
) -> Vec<String> {
    let (direct, via_callee, via_extern) = canonicalize_provenance_fields(Some(provenance));
    vec![
        evidence_line("function", function.to_string()),
        evidence_line("effect", effect.to_string()),
        evidence_line("direct", direct.to_string()),
        evidence_line("via_callee", format!("[{}]", via_callee.join(","))),
        evidence_line("via_extern", format!("[{}]", via_extern.join(","))),
    ]
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

enum NoYieldLimitObservation<'a> {
    AboveLimit {
        symbol: &'a str,
        span: u64,
        limit: u64,
    },
    Unbounded {
        symbol: &'a str,
        limit: u64,
    },
}

enum LockRuleObservation<'a> {
    Depth(LockDepthObservation),
    ForbiddenEdge(ForbiddenLockEdgeObservation<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SchemaMismatchObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LockDepthObservation {
    observed: u64,
    limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ForbiddenLockEdgeObservation<'a> {
    from: &'a str,
    to: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NoYieldUnboundedObservation<'a> {
    symbol: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IrqEffectObservation<'a> {
    symbol_name: &'a str,
    provenance: Option<&'a ContractsProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CriticalRegionRuleObservation {
    effect: &'static str,
    rule: PolicyRule,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModuleCapabilityObservation {
    capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IrqCapabilityObservation<'a> {
    symbol_name: &'a str,
    capability: String,
    provenance: Option<&'a ContractsProvenance>,
}

enum EffectRuleObservation<'a> {
    NoYieldLimit(NoYieldLimitObservation<'a>),
    NoYieldUnbounded(NoYieldUnboundedObservation<'a>),
    IrqEffect {
        effect: &'a str,
        observation: IrqEffectObservation<'a>,
    },
}

enum CapabilityRuleObservation<'a> {
    ModuleCapability(ModuleCapabilityObservation),
    IrqCapability(IrqCapabilityObservation<'a>),
}

fn policy_rule_requires_v2_schema_mismatch(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    rule: PolicyRule,
) -> Option<SchemaMismatchObservation> {
    (policy_rule_is_enabled(policy, rule)
        && policy_rule_conditions(rule).iter().any(|descriptor| {
            matches!(
                descriptor,
                PolicyConditionDescriptor::SchemaVersionRequiresV2
            )
        })
        && contracts.schema_version != CONTRACTS_SCHEMA_VERSION_V2)
        .then_some(SchemaMismatchObservation)
}

fn policy_rule_lock_depth_violation(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    rule: PolicyRule,
) -> Option<LockDepthObservation> {
    policy_rule_max_lock_depth(policy, rule)
        .filter(|limit| contracts.report.max_lock_depth > *limit)
        .map(|limit| LockDepthObservation {
            observed: contracts.report.max_lock_depth,
            limit,
        })
}

fn policy_rule_forbidden_lock_edge_violations<'a>(
    policy: &'a PolicyFile,
    _contracts: &ContractsBundle,
    rule: PolicyRule,
    observed_edges: &BTreeSet<(&'a str, &'a str)>,
) -> Vec<ForbiddenLockEdgeObservation<'a>> {
    let mut violations = Vec::new();
    if let Some(edges) = policy_rule_forbidden_lock_edges(policy, rule) {
        for edge in edges {
            let pair = (edge[0].as_str(), edge[1].as_str());
            if observed_edges.contains(&pair) {
                violations.push(ForbiddenLockEdgeObservation {
                    from: pair.0,
                    to: pair.1,
                });
            }
        }
    }
    violations
}

fn policy_rule_no_yield_limit_violations<'a>(
    policy: &PolicyFile,
    contracts: &'a ContractsBundle,
    rule: PolicyRule,
) -> Vec<NoYieldLimitObservation<'a>> {
    let mut violations = Vec::new();
    if let Some(limit) = policy_rule_max_no_yield_span(policy, rule) {
        for (symbol, span) in &contracts.report.no_yield_spans {
            match span {
                ContractsNoYieldSpan::Bounded(v) if *v > limit => {
                    violations.push(NoYieldLimitObservation::AboveLimit {
                        symbol: symbol.as_str(),
                        span: *v,
                        limit,
                    });
                }
                ContractsNoYieldSpan::Unbounded(v) if v == "unbounded" => {
                    violations.push(NoYieldLimitObservation::Unbounded {
                        symbol: symbol.as_str(),
                        limit,
                    });
                }
                _ => {}
            }
        }
    }
    violations
}

fn policy_rule_no_yield_unbounded_violations<'a>(
    _policy: &PolicyFile,
    contracts: &'a ContractsBundle,
    rule: PolicyRule,
) -> Vec<NoYieldUnboundedObservation<'a>> {
    if !policy_rule_conditions(rule)
        .iter()
        .any(|descriptor| matches!(descriptor, PolicyConditionDescriptor::NoYieldSpanUnbounded))
    {
        return Vec::new();
    }
    contracts
        .report
        .no_yield_spans
        .iter()
        .filter_map(|(symbol, span)| {
            span.is_unbounded().then_some(NoYieldUnboundedObservation {
                symbol: symbol.as_str(),
            })
        })
        .collect()
}

fn policy_rule_irq_effect_violations<'a>(
    view: &'a PolicyEvalView<'a>,
    rule: PolicyRule,
) -> Vec<IrqEffectObservation<'a>> {
    let Some(effect) = policy_rule_effect_condition(rule) else {
        return Vec::new();
    };
    let mut violations = Vec::new();
    for symbol_name in &view.irq_symbol_names {
        if let Some(symbol) = view.symbol_by_name.get(*symbol_name)
            && symbol.has_eff_transitive(effect)
        {
            violations.push(IrqEffectObservation {
                symbol_name,
                provenance: symbol.eff_provenance(effect),
            });
        }
    }
    violations
}

fn policy_region_rule_observations() -> Vec<CriticalRegionRuleObservation> {
    policy_family_specs(PolicyFamily::Region)
        .filter_map(|spec| {
            policy_rule_conditions(spec.rule)
                .iter()
                .find_map(|descriptor| match descriptor {
                    PolicyConditionDescriptor::CriticalRegionEffectObserved { effect } => {
                        Some(CriticalRegionRuleObservation {
                            effect,
                            rule: spec.rule,
                        })
                    }
                    _ => None,
                })
        })
        .collect()
}

fn policy_rule_disallowed_module_capabilities(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    rule: PolicyRule,
) -> Vec<ModuleCapabilityObservation> {
    let Some(allowed) = policy_rule_module_cap_allowlist(policy, rule) else {
        return Vec::new();
    };
    let allowed_caps = allowed.iter().map(|c| c.as_str()).collect::<BTreeSet<_>>();
    let mut disallowed = contracts
        .capabilities
        .module_caps
        .iter()
        .filter(|cap| !allowed_caps.contains(cap.as_str()))
        .cloned()
        .map(|capability| ModuleCapabilityObservation { capability })
        .collect::<Vec<_>>();
    disallowed.sort();
    disallowed.dedup_by(|left, right| left.capability == right.capability);
    disallowed
}

fn policy_rule_irq_capability_violations<'a>(
    policy: &PolicyFile,
    view: &'a PolicyEvalView<'a>,
    rule: PolicyRule,
) -> Vec<IrqCapabilityObservation<'a>> {
    let Some((allowed_caps, forbidden_caps)) = policy_rule_irq_capability_lists(policy, rule)
    else {
        return Vec::new();
    };
    let mut violations = Vec::new();
    for cap in forbidden_caps {
        if allowed_caps.contains(&cap) {
            continue;
        }
        for symbol_name in &view.irq_symbol_names {
            if let Some(symbol) = view.symbol_by_name.get(*symbol_name)
                && symbol.has_cap_transitive(&cap)
            {
                violations.push(IrqCapabilityObservation {
                    symbol_name,
                    capability: cap.clone(),
                    provenance: symbol.cap_provenance(&cap),
                });
            }
        }
    }
    violations
}

fn bind_schema_mismatch_observation(_observation: SchemaMismatchObservation) -> PolicyViolation {
    violation_kernel_policy_requires_v2()
}

fn bind_context_rule_violation(
    rule: PolicyRule,
    observation: SchemaMismatchObservation,
) -> PolicyViolation {
    let binder_kind = policy_rule_binder_kind(rule);
    debug_assert_eq!(binder_kind, PolicyViolationBinderKind::SchemaMismatch);
    match binder_kind {
        PolicyViolationBinderKind::SchemaMismatch => bind_schema_mismatch_observation(observation),
        _ => unreachable!("unexpected binder kind for {:?}", rule),
    }
}

fn bind_lock_depth_observation(observation: LockDepthObservation) -> PolicyViolation {
    violation_limit_max_lock_depth(observation.observed, observation.limit)
}

fn bind_forbidden_lock_edge_observation(
    observation: ForbiddenLockEdgeObservation<'_>,
) -> PolicyViolation {
    violation_lock_forbid_edge(observation.from, observation.to)
}

fn bind_no_yield_limit_observation(observation: NoYieldLimitObservation<'_>) -> PolicyViolation {
    match observation {
        NoYieldLimitObservation::AboveLimit {
            symbol,
            span,
            limit,
        } => violation_no_yield_span_limit(symbol, span, limit),
        NoYieldLimitObservation::Unbounded { symbol, limit } => {
            violation_no_yield_unbounded_with_limit(symbol, limit)
        }
    }
}

fn bind_no_yield_unbounded_observation(
    observation: NoYieldUnboundedObservation<'_>,
) -> PolicyViolation {
    violation_no_yield_unbounded(observation.symbol)
}

fn bind_lock_rule_violation(
    rule: PolicyRule,
    observation: LockRuleObservation<'_>,
) -> PolicyViolation {
    let binder_kind = policy_rule_binder_kind(rule);
    match observation {
        LockRuleObservation::Depth(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::LockDepth);
            match binder_kind {
                PolicyViolationBinderKind::LockDepth => bind_lock_depth_observation(observation),
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        LockRuleObservation::ForbiddenEdge(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::ForbiddenLockEdge);
            match binder_kind {
                PolicyViolationBinderKind::ForbiddenLockEdge => {
                    bind_forbidden_lock_edge_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
    }
}

fn bind_irq_effect_observation(
    rule: PolicyRule,
    effect: &str,
    observation: IrqEffectObservation<'_>,
) -> PolicyViolation {
    violation_kernel_irq_effect(
        rule,
        observation.symbol_name,
        effect,
        observation.provenance,
    )
}

fn bind_effect_rule_violation(
    rule: PolicyRule,
    observation: EffectRuleObservation<'_>,
) -> PolicyViolation {
    let binder_kind = policy_rule_binder_kind(rule);
    match observation {
        EffectRuleObservation::NoYieldLimit(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::NoYieldLimit);
            match binder_kind {
                PolicyViolationBinderKind::NoYieldLimit => {
                    bind_no_yield_limit_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::NoYieldUnbounded(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::NoYieldUnbounded);
            match binder_kind {
                PolicyViolationBinderKind::NoYieldUnbounded => {
                    bind_no_yield_unbounded_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::IrqEffect {
            effect,
            observation,
        } => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::IrqEffect);
            match binder_kind {
                PolicyViolationBinderKind::IrqEffect => {
                    bind_irq_effect_observation(rule, effect, observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
    }
}

fn bind_critical_region_violation(
    rule: PolicyRule,
    violation: &ContractsCriticalViolation,
) -> PolicyViolation {
    violation_kernel_critical_region_effect(
        rule,
        violation.function.as_str(),
        violation.effect.as_str(),
        &violation.provenance,
    )
}

fn bind_region_rule_violation(
    rule: PolicyRule,
    violation: &ContractsCriticalViolation,
) -> PolicyViolation {
    let binder_kind = policy_rule_binder_kind(rule);
    debug_assert_eq!(
        binder_kind,
        PolicyViolationBinderKind::CriticalRegionViolation
    );
    match binder_kind {
        PolicyViolationBinderKind::CriticalRegionViolation => {
            bind_critical_region_violation(rule, violation)
        }
        _ => unreachable!("unexpected binder kind for {:?}", rule),
    }
}

fn bind_module_capability_observation(observation: ModuleCapabilityObservation) -> PolicyViolation {
    violation_cap_module_allowlist(observation.capability.as_str())
}

fn bind_irq_capability_observation(observation: IrqCapabilityObservation<'_>) -> PolicyViolation {
    violation_kernel_irq_cap_forbid(
        observation.symbol_name,
        observation.capability.as_str(),
        observation.provenance,
    )
}

fn bind_capability_rule_violation(
    rule: PolicyRule,
    observation: CapabilityRuleObservation<'_>,
) -> PolicyViolation {
    let binder_kind = policy_rule_binder_kind(rule);
    match observation {
        CapabilityRuleObservation::ModuleCapability(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::ModuleCapability);
            match binder_kind {
                PolicyViolationBinderKind::ModuleCapability => {
                    bind_module_capability_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        CapabilityRuleObservation::IrqCapability(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::IrqCapability);
            match binder_kind {
                PolicyViolationBinderKind::IrqCapability => {
                    bind_irq_capability_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
    }
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
            | PolicyArtifactDependency::FactsSymbolsCapsTransitive
            | PolicyArtifactDependency::FactsSymbolsCapsProvenance
            | PolicyArtifactDependency::ReportCriticalViolations
    )
}

fn policy_violation(rule: PolicyRule, msg: String) -> PolicyViolation {
    policy_violation_with_evidence(rule, msg, Vec::new())
}

fn policy_violation_with_evidence(
    rule: PolicyRule,
    msg: String,
    evidence: Vec<String>,
) -> PolicyViolation {
    let spec = policy_rule_spec(rule);
    PolicyViolation {
        rule: spec.rule,
        family: spec.family,
        sort_rank: spec.sort_rank,
        requires_v2: spec.requires_v2,
        default_enabled_in_profile_kernel: spec.default_enabled_in_profile_kernel,
        diagnostic_template_id: spec.diagnostic_template_id,
        code: spec.code,
        msg,
        evidence,
    }
}

fn violation_kernel_policy_requires_v2() -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelPolicyRequiresV2,
        format!(
            "kernel policy rules require contracts schema '{}'",
            CONTRACTS_SCHEMA_VERSION_V2
        ),
    )
}

fn violation_limit_max_lock_depth(observed: u64, limit: u64) -> PolicyViolation {
    policy_violation(
        PolicyRule::LimitMaxLockDepth,
        format!("max_lock_depth {} exceeds limit {}", observed, limit),
    )
}

fn violation_lock_forbid_edge(from: &str, to: &str) -> PolicyViolation {
    policy_violation(
        PolicyRule::LockForbidEdge,
        format!("forbidden lock edge '{} -> {}' is present", from, to),
    )
}

fn violation_no_yield_span_limit(symbol: &str, span: u64, limit: u64) -> PolicyViolation {
    policy_violation(
        PolicyRule::NoYieldSpanLimit,
        format!(
            "no_yield_spans '{}' has span {} above limit {}",
            symbol, span, limit
        ),
    )
}

fn violation_no_yield_unbounded_with_limit(symbol: &str, limit: u64) -> PolicyViolation {
    policy_violation(
        PolicyRule::NoYieldUnbounded,
        format!(
            "no_yield_spans '{}' is unbounded and violates max_no_yield_span {}",
            symbol, limit
        ),
    )
}

fn violation_no_yield_unbounded(symbol: &str) -> PolicyViolation {
    policy_violation(
        PolicyRule::NoYieldUnbounded,
        format!("no_yield_spans '{}' is unbounded", symbol),
    )
}

fn violation_kernel_irq_effect(
    rule: PolicyRule,
    symbol_name: &str,
    effect: &str,
    provenance: Option<&ContractsProvenance>,
) -> PolicyViolation {
    policy_violation_with_evidence(
        rule,
        format!(
            "function '{}' is irq-reachable and uses {} effect ({})",
            symbol_name,
            effect,
            format_optional_provenance(provenance)
        ),
        evidence_lines_irq_effect(symbol_name, effect, provenance),
    )
}

fn violation_kernel_critical_region_effect(
    rule: PolicyRule,
    function: &str,
    effect: &str,
    provenance: &ContractsProvenance,
) -> PolicyViolation {
    policy_violation_with_evidence(
        rule,
        format!(
            "function '{}' uses {} effect in critical region ({})",
            function,
            effect,
            format_provenance(provenance)
        ),
        evidence_lines_critical_region(function, effect, provenance),
    )
}

fn violation_cap_module_allowlist(capability: &str) -> PolicyViolation {
    policy_violation(
        PolicyRule::CapModuleAllowlist,
        format!("module capability '{}' is not in allow_module", capability),
    )
}

fn violation_kernel_irq_cap_forbid(
    symbol_name: &str,
    capability: &str,
    provenance: Option<&ContractsProvenance>,
) -> PolicyViolation {
    policy_violation_with_evidence(
        PolicyRule::KernelIrqCapForbid,
        format!(
            "function '{}' is irq-reachable and uses forbidden capability '{}' ({})",
            symbol_name,
            capability,
            format_optional_provenance(provenance)
        ),
        evidence_lines_irq_capability(symbol_name, capability, provenance),
    )
}

fn format_context_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_lock_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_effect_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_region_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_capability_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_limit_violation(violation: &PolicyViolation) -> String {
    format!("policy: {}: {}", violation.code, violation.msg)
}

fn format_policy_violation(violation: &PolicyViolation) -> String {
    match violation.family {
        PolicyFamily::Context => format_context_violation(violation),
        PolicyFamily::Lock => format_lock_violation(violation),
        PolicyFamily::Effect => format_effect_violation(violation),
        PolicyFamily::Region => format_region_violation(violation),
        PolicyFamily::Capability => format_capability_violation(violation),
        PolicyFamily::Limit => format_limit_violation(violation),
    }
}

fn print_policy_violations(violations: &[PolicyViolation], evidence: bool) {
    for violation in violations {
        eprintln!("{}", format_policy_violation(violation));
        if evidence {
            for line in &violation.evidence {
                eprintln!("{}", line);
            }
        }
    }
}

fn validate_json_against_schema_text(
    instance: &Value,
    schema_text: &str,
    schema_name: &str,
    label: &str,
) -> Result<(), String> {
    let schema_json: Value = serde_json::from_str(schema_text)
        .map_err(|e| format!("failed to parse schema '{}': {}", schema_name, e))?;
    let compiled = JSONSchema::compile(&schema_json)
        .map_err(|e| format!("failed to compile schema '{}': {}", schema_name, e))?;

    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        return Err(format!(
            "{} schema validation failed: {}",
            label,
            details.join(" | ")
        ));
    }

    Ok(())
}

fn print_json_result<E: std::fmt::Display>(result: Result<String, E>, label: &str) -> ExitCode {
    match result {
        Ok(text) => {
            println!("{}", text);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to serialize {}: {}", label, err);
            ExitCode::from(1)
        }
    }
}

fn print_errors(errs: &[String]) {
    for err in errs {
        eprintln!("{}", err);
    }
}

fn format_contracts_inspect_summary(contracts: &ContractsBundle) -> String {
    let irq_reachable = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_ctx_reachable("irq")
    });
    let critical_functions =
        collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| symbol.attrs.critical);
    let alloc_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("alloc")
    });
    let block_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("block")
    });
    let yield_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        symbol.has_eff_transitive("yield")
    });
    let cap_symbols = collect_sorted_symbol_names_by(&contracts.facts.symbols, |symbol| {
        !symbol.caps_transitive.is_empty()
    });

    let mut critical_violations = contracts.report.critical.violations.clone();
    critical_violations.sort();
    critical_violations.dedup();

    let mut lines = vec![
        format!("schema: {}", contracts.schema_version),
        format!("symbols: total={}", contracts.facts.symbols.len()),
        "contexts:".to_string(),
        format!(
            "irq_reachable: {} {}",
            irq_reachable.len(),
            format_list(&irq_reachable)
        ),
        format!(
            "critical_functions: {} {}",
            critical_functions.len(),
            format_list(&critical_functions)
        ),
        "effects:".to_string(),
        format!(
            "alloc: {} {}",
            alloc_symbols.len(),
            format_list(&alloc_symbols)
        ),
        format!(
            "block: {} {}",
            block_symbols.len(),
            format_list(&block_symbols)
        ),
        format!(
            "yield: {} {}",
            yield_symbols.len(),
            format_list(&yield_symbols)
        ),
        "capabilities:".to_string(),
        format!(
            "symbols_with_caps: {} {}",
            cap_symbols.len(),
            format_list(&cap_symbols)
        ),
        "critical_report:".to_string(),
        format!("violations: {}", critical_violations.len()),
    ];

    for violation in critical_violations {
        let (direct, via_callee, via_extern) =
            canonicalize_provenance_fields(Some(&violation.provenance));
        lines.push(format!(
            "violation: function={} effect={} direct={} via_callee={} via_extern={}",
            violation.function,
            violation.effect,
            direct,
            format_list(&via_callee),
            format_list(&via_extern)
        ));
    }

    lines.join("\n")
}

fn format_verify_report_inspect_summary(report: &DecodedVerifyReport) -> String {
    let mut lines = vec![
        format!("schema: {}", report.schema_version),
        format!("result: {}", report.result),
        "inputs:".to_string(),
        format!("contracts: {}", report.inputs.contracts),
        format!("hash: {}", report.inputs.hash),
        format!("sig: {}", format_option_value(report.inputs.sig.as_deref())),
        format!(
            "pubkey: {}",
            format_option_value(report.inputs.pubkey.as_deref())
        ),
        "hash_status:".to_string(),
        format!("matched: {}", report.hash.matched),
        format!(
            "expected_sha256: {}",
            format_option_value(report.hash.expected_sha256.as_deref())
        ),
        format!(
            "computed_sha256: {}",
            format_option_value(report.hash.computed_sha256.as_deref())
        ),
        "contracts_status:".to_string(),
        format!("utf8_valid: {}", report.contracts.utf8_valid),
        format!("schema_valid: {}", report.contracts.schema_valid),
        format!(
            "schema_version: {}",
            format_option_value(report.contracts.schema_version.as_deref())
        ),
        "signature_status:".to_string(),
        format!("checked: {}", report.signature.checked),
        format!("valid: {}", format_option_bool(report.signature.valid)),
        format!("diagnostics: {}", report.diagnostics.len()),
    ];

    for diagnostic in &report.diagnostics {
        lines.push(format!("diagnostic: {}", diagnostic));
    }

    lines.join("\n")
}

fn collect_sorted_symbol_names_by<F>(symbols: &[ContractsFactSymbol], predicate: F) -> Vec<String>
where
    F: Fn(&ContractsFactSymbol) -> bool,
{
    symbols
        .iter()
        .filter(|symbol| predicate(symbol))
        .map(|symbol| symbol.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn format_list(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

fn format_option_value(value: Option<&str>) -> &str {
    value.unwrap_or("<none>")
}

fn format_option_bool(value: Option<bool>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  kernriftc --version");
    eprintln!("  kernriftc check <file.kr>");
    eprintln!("  kernriftc check --profile kernel <file.kr>");
    eprintln!("  kernriftc check --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --profile kernel --contracts-schema v2 <file.kr>");
    eprintln!("  kernriftc check --policy <policy.toml> <file.kr>");
    eprintln!("  kernriftc check --contracts-out <contracts.json> <file.kr>");
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>"
    );
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> <file.kr>"
    );
    eprintln!(
        "  kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> --sign-ed25519 <secret.hex> --sig-out <contracts.sig> <file.kr>"
    );
    eprintln!("  kernriftc policy --policy <policy.toml> --contracts <contracts.json>");
    eprintln!("  kernriftc policy --evidence --policy <policy.toml> --contracts <contracts.json>");
    eprintln!("  kernriftc inspect --contracts <contracts.json>");
    eprintln!("  kernriftc inspect-report --report <verify-report.json>");
    eprintln!("  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256>");
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --sig <contracts.sig> --pubkey <pubkey.hex>"
    );
    eprintln!(
        "  kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --report <verify-report.json>"
    );
    eprintln!("  kernriftc --selftest");
    eprintln!("  kernriftc --emit krir <file.kr>");
    eprintln!("  kernriftc --emit lockgraph <file.kr>");
    eprintln!("  kernriftc --emit caps <file.kr>");
    eprintln!("  kernriftc --emit contracts <file.kr>");
    eprintln!("  kernriftc --emit report --metrics max_lock_depth,no_yield_spans <file.kr>");
    eprintln!("  kernriftc --report max_lock_depth,no_yield_spans <file.kr>");
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
                (109, RULE_LIMIT_MAX_LOCK_DEPTH, "x"),
                (110, RULE_LOCK_FORBID_EDGE, "z"),
            ]
        );
    }
}
