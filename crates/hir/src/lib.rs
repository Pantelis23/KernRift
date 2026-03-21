use std::collections::{BTreeMap, BTreeSet};

use krir::{
    ArithOp as KrirArithOp, CallEdge, Ctx, Eff, ExecutableBlock, ExecutableExternDecl,
    ExecutableFacts, ExecutableFunction, ExecutableKrirModule, ExecutableOp as KrExecutableOp,
    ExecutableSignature, ExecutableTerminator, ExecutableValue, ExecutableValueType, Function,
    FunctionAttrs, KrirModule, KrirOp, KrirParamTy, MmioAddrExpr as KrirMmioAddrExpr,
    MmioBaseDecl as KrirMmioBaseDecl, MmioRegAccess as KrirMmioRegAccess,
    MmioRegisterDecl as KrirMmioRegisterDecl, MmioScalarType as KrirMmioScalarType,
    MmioValueExpr as KrirMmioValueExpr, PercpuDecl as KrirPercpuDecl, SchedHook,
};
use parser::{
    ArithOp as ParserArithOp, FnAst, MmioAddrExpr as ParserMmioAddrExpr,
    MmioBaseDecl as ParserMmioBaseDecl, MmioRegAccess as ParserMmioRegAccess,
    MmioRegisterDecl as ParserMmioRegisterDecl, MmioScalarType as ParserMmioScalarType,
    MmioValueExpr as ParserMmioValueExpr, ModuleAst, ParamTy as ParserParamTy, RawAttr, Stmt,
    format_source_diagnostic, int_literal_numeric_value, split_csv_allow_trailing_comma,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceProfile {
    #[default]
    Stable,
    Experimental,
}

impl SurfaceProfile {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "stable" => Ok(Self::Stable),
            "experimental" => Ok(Self::Experimental),
            other => Err(format!(
                "invalid surface mode '{}', expected 'stable' or 'experimental'",
                other
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptiveFeatureStatus {
    Experimental,
    Stable,
    Deprecated,
}

impl AdaptiveFeatureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptiveAliasClassification {
    CompatibilityAlias,
    DeprecatedAlias,
}

impl AdaptiveAliasClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompatibilityAlias => "compatibility_alias",
            Self::DeprecatedAlias => "deprecated_alias",
        }
    }

    pub fn diagnostic_label(self) -> &'static str {
        match self {
            Self::CompatibilityAlias => "compatibility alias",
            Self::DeprecatedAlias => "deprecated alias",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdaptiveSurfaceFeature {
    pub id: &'static str,
    pub proposal_id: &'static str,
    pub surface_form: &'static str,
    pub status: AdaptiveFeatureStatus,
    pub lowering_target: &'static str,
    pub safety_notes: &'static str,
    pub migration_supported: bool,
    pub migration_note: &'static str,
    pub canonical_replacement: &'static str,
    pub migration_safe: bool,
    pub rewrite_intent: &'static str,
    pub surface_profile_gate: SurfaceProfile,
    #[serde(skip_serializing)]
    lowering_rule: AdaptiveLoweringRule,
}

impl AdaptiveSurfaceFeature {
    pub fn alias_classification(self) -> AdaptiveAliasClassification {
        match self.status {
            AdaptiveFeatureStatus::Deprecated => AdaptiveAliasClassification::DeprecatedAlias,
            AdaptiveFeatureStatus::Experimental | AdaptiveFeatureStatus::Stable => {
                AdaptiveAliasClassification::CompatibilityAlias
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdaptiveFeatureProposal {
    pub id: &'static str,
    pub title: &'static str,
    pub motivation: &'static str,
    pub syntax_before: &'static str,
    pub syntax_after: &'static str,
    pub lowering_description: &'static str,
    pub compatibility_risk: &'static str,
    pub migration_plan: &'static str,
    pub status: AdaptiveFeatureStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveMigrationPreviewEntry {
    pub function_name: String,
    pub feature: &'static AdaptiveSurfaceFeature,
    pub enabled_under_surface: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendMigrationFeature {
    pub id: &'static str,
    pub status: AdaptiveFeatureStatus,
    pub classification: AdaptiveAliasClassification,
    pub surface_form: &'static str,
    pub lowering_target: &'static str,
    pub proposal_id: &'static str,
    pub migration_safe: bool,
    pub canonical_replacement: &'static str,
    pub rewrite_intent: &'static str,
}

impl FrontendMigrationFeature {
    fn from_adaptive(feature: &'static AdaptiveSurfaceFeature) -> Self {
        Self {
            id: feature.id,
            status: feature.status,
            classification: feature.alias_classification(),
            surface_form: feature.surface_form,
            lowering_target: feature.lowering_target,
            proposal_id: feature.proposal_id,
            migration_safe: feature.migration_safe,
            canonical_replacement: feature.canonical_replacement,
            rewrite_intent: feature.rewrite_intent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendMigrationPreviewEntry {
    pub function_name: String,
    pub feature: FrontendMigrationFeature,
    pub enabled_under_surface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrontendCanonicalFinding {
    pub function_name: String,
    pub classification: AdaptiveAliasClassification,
    pub surface_form: &'static str,
    pub canonical_replacement: &'static str,
    pub migration_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrontendCanonicalEditPlanEntry {
    pub function_name: String,
    pub classification: AdaptiveAliasClassification,
    pub surface_form: &'static str,
    pub canonical_replacement: &'static str,
    pub migration_safe: bool,
    pub rewrite_intent: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendCanonicalRewrite {
    pub function_name: String,
    pub classification: AdaptiveAliasClassification,
    pub surface_form: &'static str,
    pub canonical_replacement: &'static str,
    pub migration_safe: bool,
    pub rewrite_intent: &'static str,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveFeatureProposalSummary {
    pub feature: &'static AdaptiveSurfaceFeature,
    pub proposal: &'static AdaptiveFeatureProposal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveFeaturePromotionReadiness {
    pub feature_id: &'static str,
    pub current_status: AdaptiveFeatureStatus,
    pub promotable_to_stable: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveFeaturePromotionPlan {
    pub feature_id: &'static str,
    pub proposal_id: &'static str,
    pub normalized_proposal_title: String,
    pub normalized_compatibility_risk: String,
    pub normalized_migration_plan: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalExecutableValueType {
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalExecutableTerminator {
    ReturnUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CanonicalExecutableOp {
    Call { callee: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalExecutableSignature {
    pub params: Vec<CanonicalExecutableValueType>,
    pub result: CanonicalExecutableValueType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalExecutableFacts {
    pub ctx_ok: Vec<Ctx>,
    pub eff_used: Vec<Eff>,
    pub caps_req: Vec<String>,
    pub attrs: FunctionAttrs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalExecutableBody {
    pub ops: Vec<CanonicalExecutableOp>,
    pub terminator: CanonicalExecutableTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalExecutableFunction {
    pub name: String,
    pub signature: CanonicalExecutableSignature,
    pub facts: CanonicalExecutableFacts,
    pub body: CanonicalExecutableBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalExecutableExternDecl {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct CanonicalExecutableModule {
    pub module_caps: Vec<String>,
    pub functions: Vec<CanonicalExecutableFunction>,
    pub extern_declarations: Vec<CanonicalExecutableExternDecl>,
}

impl CanonicalExecutableModule {
    pub fn canonicalize(&mut self) {
        self.module_caps.sort();
        self.module_caps.dedup();

        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        self.extern_declarations.sort_by(|a, b| a.name.cmp(&b.name));
        for function in &mut self.functions {
            function.facts.ctx_ok.sort_by_key(|ctx| ctx.as_str());
            function.facts.ctx_ok.dedup();
            function.facts.eff_used.sort_by_key(|eff| eff.as_str());
            function.facts.eff_used.dedup();
            function.facts.caps_req.sort();
            function.facts.caps_req.dedup();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdaptiveLoweringRule {
    ContextAlias(&'static [Ctx]),
    EffectAlias(&'static [Eff]),
}

const ADAPTIVE_SURFACE_FEATURES: [AdaptiveSurfaceFeature; 4] = [
    AdaptiveSurfaceFeature {
        id: "irq_handler_alias",
        proposal_id: "irq_handler_alias",
        surface_form: "irq_handler",
        status: AdaptiveFeatureStatus::Experimental,
        lowering_target: "@ctx(irq)",
        safety_notes: "Pure surface alias; lowers to the existing irq context declaration.",
        migration_supported: true,
        migration_note: "Replace with @ctx(irq) when pinning code back to stable.",
        canonical_replacement: "@ctx(irq)",
        migration_safe: true,
        rewrite_intent: "Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
        surface_profile_gate: SurfaceProfile::Experimental,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Irq]),
    },
    AdaptiveSurfaceFeature {
        id: "thread_entry_alias",
        proposal_id: "thread_entry_alias",
        surface_form: "thread_entry",
        status: AdaptiveFeatureStatus::Stable,
        lowering_target: "@ctx(thread)",
        safety_notes: "Pure surface alias; lowers to the existing thread context declaration.",
        migration_supported: true,
        migration_note: "Replace with @ctx(thread) when pinning code back to stable.",
        canonical_replacement: "@ctx(thread)",
        migration_safe: true,
        rewrite_intent: "Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        surface_profile_gate: SurfaceProfile::Stable,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Thread]),
    },
    AdaptiveSurfaceFeature {
        id: "may_block_alias",
        proposal_id: "may_block_alias",
        surface_form: "may_block",
        status: AdaptiveFeatureStatus::Experimental,
        lowering_target: "@eff(block)",
        safety_notes: "Pure surface alias; lowers to the existing block effect declaration.",
        migration_supported: true,
        migration_note: "Replace with @eff(block) when pinning code back to stable.",
        canonical_replacement: "@eff(block)",
        migration_safe: true,
        rewrite_intent: "Replace the attribute token `@may_block` with `@eff(block)`.",
        surface_profile_gate: SurfaceProfile::Experimental,
        lowering_rule: AdaptiveLoweringRule::EffectAlias(&[Eff::Block]),
    },
    AdaptiveSurfaceFeature {
        id: "irq_legacy_alias",
        proposal_id: "irq_legacy_alias",
        surface_form: "irq_legacy",
        status: AdaptiveFeatureStatus::Deprecated,
        lowering_target: "@ctx(irq)",
        safety_notes: "Deprecated surface alias kept only to prove centralized lifecycle gating.",
        migration_supported: true,
        migration_note: "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
        canonical_replacement: "@ctx(irq)",
        migration_safe: true,
        rewrite_intent: "Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
        surface_profile_gate: SurfaceProfile::Stable,
        lowering_rule: AdaptiveLoweringRule::ContextAlias(&[Ctx::Irq]),
    },
];

const LEGACY_FRONTEND_MIGRATION_FEATURES: [FrontendMigrationFeature; 5] = [
    FrontendMigrationFeature {
        id: "legacy_alloc_effect_shorthand",
        status: AdaptiveFeatureStatus::Stable,
        classification: AdaptiveAliasClassification::CompatibilityAlias,
        surface_form: "alloc",
        lowering_target: "@eff(alloc)",
        proposal_id: "-",
        migration_safe: true,
        canonical_replacement: "@eff(alloc)",
        rewrite_intent: "Replace the attribute token `@alloc` with `@eff(alloc)`.",
    },
    FrontendMigrationFeature {
        id: "legacy_block_effect_shorthand",
        status: AdaptiveFeatureStatus::Stable,
        classification: AdaptiveAliasClassification::CompatibilityAlias,
        surface_form: "block",
        lowering_target: "@eff(block)",
        proposal_id: "-",
        migration_safe: true,
        canonical_replacement: "@eff(block)",
        rewrite_intent: "Replace the attribute token `@block` with `@eff(block)`.",
    },
    FrontendMigrationFeature {
        id: "legacy_irq_context_shorthand",
        status: AdaptiveFeatureStatus::Stable,
        classification: AdaptiveAliasClassification::CompatibilityAlias,
        surface_form: "irq",
        lowering_target: "@ctx(irq)",
        proposal_id: "-",
        migration_safe: true,
        canonical_replacement: "@ctx(irq)",
        rewrite_intent: "Replace the attribute token `@irq` with `@ctx(irq)`.",
    },
    FrontendMigrationFeature {
        id: "legacy_noirq_context_shorthand",
        status: AdaptiveFeatureStatus::Stable,
        classification: AdaptiveAliasClassification::CompatibilityAlias,
        surface_form: "noirq",
        lowering_target: "@ctx(thread, boot)",
        proposal_id: "-",
        migration_safe: true,
        canonical_replacement: "@ctx(thread, boot)",
        rewrite_intent: "Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
    },
    FrontendMigrationFeature {
        id: "legacy_preempt_off_effect_shorthand",
        status: AdaptiveFeatureStatus::Stable,
        classification: AdaptiveAliasClassification::CompatibilityAlias,
        surface_form: "preempt_off",
        lowering_target: "@eff(preempt_off)",
        proposal_id: "-",
        migration_safe: true,
        canonical_replacement: "@eff(preempt_off)",
        rewrite_intent: "Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
    },
];

pub fn adaptive_surface_features() -> &'static [AdaptiveSurfaceFeature] {
    &ADAPTIVE_SURFACE_FEATURES
}

pub fn adaptive_surface_features_for_profile(
    surface_profile: SurfaceProfile,
) -> Vec<&'static AdaptiveSurfaceFeature> {
    let mut features = ADAPTIVE_SURFACE_FEATURES
        .iter()
        .filter(|feature| surface_profile_enables_feature(surface_profile, feature))
        .collect::<Vec<_>>();
    features.sort_by(|a, b| a.id.cmp(b.id));
    features
}

pub fn adaptive_surface_migration_preview(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Vec<AdaptiveMigrationPreviewEntry> {
    let mut entries = Vec::new();

    for item in &ast.items {
        for (ordinal, attr) in item.attrs.iter().enumerate() {
            if let Some(feature) = adaptive_surface_feature(&attr.name) {
                entries.push((
                    item.name.clone(),
                    feature,
                    surface_profile_enables_feature(surface_profile, feature),
                    ordinal,
                ));
            }
        }
    }

    entries.sort_by(|a, b| (a.0.as_str(), a.1.id, a.3).cmp(&(b.0.as_str(), b.1.id, b.3)));

    entries
        .into_iter()
        .map(
            |(function_name, feature, enabled_under_surface, _)| AdaptiveMigrationPreviewEntry {
                function_name,
                feature,
                enabled_under_surface,
            },
        )
        .collect()
}

fn legacy_frontend_migration_feature(attr_name: &str) -> Option<&'static FrontendMigrationFeature> {
    LEGACY_FRONTEND_MIGRATION_FEATURES
        .iter()
        .find(|feature| feature.surface_form == attr_name)
}

pub fn frontend_migration_features_for_profile(
    surface_profile: SurfaceProfile,
) -> Vec<FrontendMigrationFeature> {
    let mut features = ADAPTIVE_SURFACE_FEATURES
        .iter()
        .filter(|feature| surface_profile_enables_feature(surface_profile, feature))
        .map(FrontendMigrationFeature::from_adaptive)
        .chain(LEGACY_FRONTEND_MIGRATION_FEATURES.iter().copied())
        .collect::<Vec<_>>();
    features.sort_by(|a, b| a.id.cmp(b.id));
    features
}

pub fn frontend_migration_preview(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Vec<FrontendMigrationPreviewEntry> {
    let mut entries = Vec::new();

    for item in &ast.items {
        for (ordinal, attr) in item.attrs.iter().enumerate() {
            if let Some(feature) = adaptive_surface_feature(&attr.name) {
                entries.push((
                    item.name.clone(),
                    FrontendMigrationFeature::from_adaptive(feature),
                    surface_profile_enables_feature(surface_profile, feature),
                    ordinal,
                ));
                continue;
            }
            if let Some(feature) = legacy_frontend_migration_feature(&attr.name) {
                entries.push((item.name.clone(), *feature, true, ordinal));
            }
        }
    }

    entries.sort_by(|a, b| (a.0.as_str(), a.1.id, a.3).cmp(&(b.0.as_str(), b.1.id, b.3)));

    entries
        .into_iter()
        .map(
            |(function_name, feature, enabled_under_surface, _)| FrontendMigrationPreviewEntry {
                function_name,
                feature,
                enabled_under_surface,
            },
        )
        .collect()
}

pub fn frontend_canonical_findings(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Vec<FrontendCanonicalFinding> {
    frontend_migration_preview(ast, surface_profile)
        .into_iter()
        .filter(|entry| entry.enabled_under_surface)
        .map(|entry| FrontendCanonicalFinding {
            function_name: entry.function_name,
            classification: entry.feature.classification,
            surface_form: entry.feature.surface_form,
            canonical_replacement: entry.feature.canonical_replacement,
            migration_safe: entry.feature.migration_safe,
        })
        .collect()
}

pub fn frontend_canonical_edit_plan(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Vec<FrontendCanonicalEditPlanEntry> {
    frontend_migration_preview(ast, surface_profile)
        .into_iter()
        .filter(|entry| entry.enabled_under_surface && entry.feature.migration_safe)
        .map(|entry| FrontendCanonicalEditPlanEntry {
            function_name: entry.function_name,
            classification: entry.feature.classification,
            surface_form: entry.feature.surface_form,
            canonical_replacement: entry.feature.canonical_replacement,
            migration_safe: entry.feature.migration_safe,
            rewrite_intent: entry.feature.rewrite_intent,
        })
        .collect()
}

pub fn frontend_canonical_rewrites(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Vec<FrontendCanonicalRewrite> {
    let mut rewrites = Vec::new();

    for item in &ast.items {
        for attr in &item.attrs {
            let feature = if let Some(feature) = adaptive_surface_feature(&attr.name) {
                let feature = FrontendMigrationFeature::from_adaptive(feature);
                if !surface_profile_enables_feature(
                    surface_profile,
                    adaptive_surface_feature(feature.surface_form)
                        .expect("adaptive feature lookup by surface form"),
                ) {
                    continue;
                }
                feature
            } else if let Some(feature) = legacy_frontend_migration_feature(&attr.name) {
                *feature
            } else {
                continue;
            };

            if !feature.migration_safe {
                continue;
            }

            rewrites.push(FrontendCanonicalRewrite {
                function_name: item.name.clone(),
                classification: feature.classification,
                surface_form: feature.surface_form,
                canonical_replacement: feature.canonical_replacement,
                migration_safe: feature.migration_safe,
                rewrite_intent: feature.rewrite_intent,
                byte_offset: attr.source.byte_offset,
            });
        }
    }

    rewrites.sort_by(|a, b| a.byte_offset.cmp(&b.byte_offset));
    rewrites
}

pub fn adaptive_feature_proposal_summaries() -> Vec<AdaptiveFeatureProposalSummary> {
    let mut summaries = ADAPTIVE_SURFACE_FEATURES
        .iter()
        .map(|feature| AdaptiveFeatureProposalSummary {
            feature,
            proposal: adaptive_feature_proposal(feature.id).expect("feature proposal"),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| a.feature.id.cmp(b.feature.id));
    summaries
}

const ADAPTIVE_FEATURE_PROPOSALS: [AdaptiveFeatureProposal; 4] = [
    AdaptiveFeatureProposal {
        id: "irq_handler_alias",
        title: "Experimental @irq_handler surface alias",
        motivation: "Provide a governed surface-only shorthand for irq-context entry points.",
        syntax_before: "@ctx(irq) fn isr() { }",
        syntax_after: "@irq_handler fn isr() { }",
        lowering_description: "Lower @irq_handler to the existing canonical @ctx(irq) representation during HIR lowering.",
        compatibility_risk: "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
        migration_plan: "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
        status: AdaptiveFeatureStatus::Experimental,
    },
    AdaptiveFeatureProposal {
        id: "thread_entry_alias",
        title: "Stable @thread_entry surface alias",
        motivation: "Provide a governed surface-only shorthand for thread-only entry points.",
        syntax_before: "@ctx(thread) fn worker() { }",
        syntax_after: "@thread_entry fn worker() { }",
        lowering_description: "Lower @thread_entry to the existing canonical @ctx(thread) representation during HIR lowering.",
        compatibility_risk: "Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
        migration_plan: "No migration required; the alias is now stable and remains interchangeable with @ctx(thread).",
        status: AdaptiveFeatureStatus::Stable,
    },
    AdaptiveFeatureProposal {
        id: "may_block_alias",
        title: "Experimental @may_block surface alias",
        motivation: "Provide a governed surface-only shorthand for declaring block effects.",
        syntax_before: "@eff(block) fn worker() { }",
        syntax_after: "@may_block fn worker() { }",
        lowering_description: "Lower @may_block to the existing canonical @eff(block) representation during HIR lowering.",
        compatibility_risk: "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
        migration_plan: "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
        status: AdaptiveFeatureStatus::Experimental,
    },
    AdaptiveFeatureProposal {
        id: "irq_legacy_alias",
        title: "Deprecated @irq_legacy surface alias",
        motivation: "Preserve a historical alias only long enough to exercise deterministic lifecycle gating.",
        syntax_before: "@ctx(irq) fn legacy_isr() { }",
        syntax_after: "@irq_legacy fn legacy_isr() { }",
        lowering_description: "Would lower to the existing canonical @ctx(irq) representation if lifecycle policy allowed it.",
        compatibility_risk: "Medium; the alias is deprecated and intentionally unavailable under current surface profiles.",
        migration_plan: "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
        status: AdaptiveFeatureStatus::Deprecated,
    },
];

pub fn adaptive_feature_proposals() -> &'static [AdaptiveFeatureProposal] {
    &ADAPTIVE_FEATURE_PROPOSALS
}

pub fn adaptive_feature_proposal(feature_id: &str) -> Option<&'static AdaptiveFeatureProposal> {
    let feature = ADAPTIVE_SURFACE_FEATURES
        .iter()
        .find(|feature| feature.id == feature_id)?;
    ADAPTIVE_FEATURE_PROPOSALS
        .iter()
        .find(|proposal| proposal.id == feature.proposal_id)
}

pub fn validate_adaptive_feature_governance() -> Vec<String> {
    validate_adaptive_feature_governance_with(
        &ADAPTIVE_SURFACE_FEATURES,
        &ADAPTIVE_FEATURE_PROPOSALS,
    )
}

pub fn adaptive_feature_promotion_readiness() -> Vec<AdaptiveFeaturePromotionReadiness> {
    adaptive_feature_promotion_readiness_with(
        &ADAPTIVE_SURFACE_FEATURES,
        &ADAPTIVE_FEATURE_PROPOSALS,
    )
}

pub fn adaptive_feature_promotion_plan(
    feature_id: &str,
) -> Result<AdaptiveFeaturePromotionPlan, String> {
    adaptive_feature_promotion_plan_with(
        &ADAPTIVE_SURFACE_FEATURES,
        &ADAPTIVE_FEATURE_PROPOSALS,
        feature_id,
    )
}

fn adaptive_feature_promotion_readiness_with(
    features: &[AdaptiveSurfaceFeature],
    proposals: &[AdaptiveFeatureProposal],
) -> Vec<AdaptiveFeaturePromotionReadiness> {
    let governance_errors = validate_adaptive_feature_governance_with(features, proposals);
    let mut readiness = features
        .iter()
        .map(|feature| {
            let reason = promotion_readiness_reason(feature, proposals, &governance_errors);
            AdaptiveFeaturePromotionReadiness {
                feature_id: feature.id,
                current_status: feature.status,
                promotable_to_stable: reason.is_none(),
                reason: reason.unwrap_or_else(|| {
                    "proposal status aligns, migration metadata is complete, proposal linked exactly once".to_string()
                }),
            }
        })
        .collect::<Vec<_>>();
    readiness.sort_by(|a, b| a.feature_id.cmp(b.feature_id));
    readiness
}

fn promotion_readiness_reason(
    feature: &AdaptiveSurfaceFeature,
    proposals: &[AdaptiveFeatureProposal],
    governance_errors: &[String],
) -> Option<String> {
    match feature.status {
        AdaptiveFeatureStatus::Stable => return Some("already stable".to_string()),
        AdaptiveFeatureStatus::Deprecated => {
            return Some("deprecated features are not promotable".to_string());
        }
        AdaptiveFeatureStatus::Experimental => {}
    }

    let references = proposals
        .iter()
        .filter(|proposal| proposal.id == feature.proposal_id)
        .count();
    if references == 0 {
        return Some("missing linked proposal".to_string());
    }
    if references != 1 {
        return Some(format!(
            "linked proposal '{}' is referenced {} times",
            feature.proposal_id, references
        ));
    }

    let Some(proposal) = proposals
        .iter()
        .find(|proposal| proposal.id == feature.proposal_id)
    else {
        return Some("missing linked proposal".to_string());
    };

    if proposal.status != AdaptiveFeatureStatus::Experimental {
        return Some("proposal status mismatch".to_string());
    }

    if governance_errors
        .iter()
        .any(|err| err.contains(&format!("feature '{}'", feature.id)))
    {
        return Some("proposal linkage/consistency is invalid".to_string());
    }

    if feature.canonical_replacement.trim().is_empty()
        || feature.rewrite_intent.trim().is_empty()
        || !feature.migration_safe
    {
        return Some("migration metadata incomplete".to_string());
    }

    if !surface_profile_enables_feature(SurfaceProfile::Experimental, feature) {
        return Some("feature is not enabled under experimental".to_string());
    }

    None
}

fn adaptive_feature_promotion_plan_with(
    features: &[AdaptiveSurfaceFeature],
    proposals: &[AdaptiveFeatureProposal],
    feature_id: &str,
) -> Result<AdaptiveFeaturePromotionPlan, String> {
    let readiness = adaptive_feature_promotion_readiness_with(features, proposals)
        .into_iter()
        .find(|entry| entry.feature_id == feature_id)
        .ok_or_else(|| format!("proposal-promotion: unknown feature '{}'", feature_id))?;
    if !readiness.promotable_to_stable {
        return Err(format!(
            "proposal-promotion: feature '{}' is not promotable: {}",
            feature_id, readiness.reason
        ));
    }

    let Some(feature) = features.iter().find(|feature| feature.id == feature_id) else {
        return Err(format!(
            "proposal-promotion: unknown feature '{}'",
            feature_id
        ));
    };
    let Some(proposal) = proposals
        .iter()
        .find(|proposal| proposal.id == feature.proposal_id)
    else {
        return Err(format!(
            "proposal-promotion: feature '{}' references missing proposal '{}'",
            feature_id, feature.proposal_id
        ));
    };

    let mut promoted_features = features.to_vec();
    let feature_idx = promoted_features
        .iter()
        .position(|entry| entry.id == feature_id)
        .expect("promotion feature position");
    promoted_features[feature_idx].status = AdaptiveFeatureStatus::Stable;

    let mut promoted_proposals = proposals.to_vec();
    let proposal_idx = promoted_proposals
        .iter()
        .position(|entry| entry.id == proposal.id)
        .expect("promotion proposal position");
    promoted_proposals[proposal_idx].status = AdaptiveFeatureStatus::Stable;

    let governance_errors =
        validate_adaptive_feature_governance_with(&promoted_features, &promoted_proposals);
    if let Some(err) = governance_errors.first() {
        return Err(format!(
            "proposal-promotion: post-promotion governance validation failed: {}",
            err
        ));
    }

    let post_promotion =
        adaptive_feature_promotion_readiness_with(&promoted_features, &promoted_proposals);
    let promoted_entry = post_promotion
        .iter()
        .find(|entry| entry.feature_id == feature_id)
        .expect("post-promotion feature readiness");
    if promoted_entry.promotable_to_stable || promoted_entry.reason != "already stable" {
        return Err(format!(
            "proposal-promotion: post-promotion readiness validation failed for '{}'",
            feature_id
        ));
    }

    Ok(AdaptiveFeaturePromotionPlan {
        feature_id: feature.id,
        proposal_id: proposal.id,
        normalized_proposal_title: format!("Stable @{} surface alias", feature.surface_form),
        normalized_compatibility_risk:
            "Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles."
                .to_string(),
        normalized_migration_plan: format!(
            "No migration required; the alias is now stable and remains interchangeable with {}.",
            feature.canonical_replacement
        ),
    })
}

fn validate_adaptive_feature_governance_with(
    features: &[AdaptiveSurfaceFeature],
    proposals: &[AdaptiveFeatureProposal],
) -> Vec<String> {
    let mut errors = Vec::new();

    for feature in features {
        let Some(proposal) = proposals
            .iter()
            .find(|proposal| proposal.id == feature.proposal_id)
        else {
            errors.push(format!(
                "proposal-validation: feature '{}' references missing proposal '{}'",
                feature.id, feature.proposal_id
            ));
            continue;
        };

        if feature.status != proposal.status {
            errors.push(format!(
                "proposal-validation: feature '{}' status mismatch with proposal",
                feature.id
            ));
        }

        if !proposal
            .syntax_after
            .contains(&format!("@{}", feature.surface_form))
        {
            errors.push(format!(
                "proposal-validation: feature '{}' syntax_after mismatch for surface form '@{}'",
                feature.id, feature.surface_form
            ));
        }

        if !proposal.syntax_before.contains(feature.lowering_target) {
            errors.push(format!(
                "proposal-validation: feature '{}' syntax_before mismatch for lowering target '{}'",
                feature.id, feature.lowering_target
            ));
        }

        if !proposal
            .lowering_description
            .contains(feature.lowering_target)
        {
            errors.push(format!(
                "proposal-validation: feature '{}' lowering description mismatch for '{}'",
                feature.id, feature.lowering_target
            ));
        }

        if !feature
            .rewrite_intent
            .contains(feature.canonical_replacement)
        {
            errors.push(format!(
                "proposal-validation: feature '{}' rewrite intent mismatch for canonical replacement '{}'",
                feature.id, feature.canonical_replacement
            ));
        }

        match feature.status {
            AdaptiveFeatureStatus::Stable => {
                if !surface_profile_enables_feature(SurfaceProfile::Stable, feature) {
                    errors.push(format!(
                        "proposal-validation: stable feature '{}' is not enabled under stable",
                        feature.id
                    ));
                }
            }
            AdaptiveFeatureStatus::Deprecated => {
                if surface_profile_enables_feature(SurfaceProfile::Stable, feature)
                    || surface_profile_enables_feature(SurfaceProfile::Experimental, feature)
                {
                    errors.push(format!(
                        "proposal-validation: deprecated feature '{}' must be disabled under all profiles",
                        feature.id
                    ));
                }
            }
            AdaptiveFeatureStatus::Experimental => {}
        }
    }

    for proposal in proposals {
        let references = features
            .iter()
            .filter(|feature| feature.proposal_id == proposal.id)
            .count();
        if references == 0 {
            errors.push(format!(
                "proposal-validation: proposal '{}' is unreferenced",
                proposal.id
            ));
        } else if references > 1 {
            errors.push(format!(
                "proposal-validation: proposal '{}' is referenced {} times",
                proposal.id, references
            ));
        }
    }

    errors.sort();
    errors
}

fn adaptive_surface_feature(attr_name: &str) -> Option<&'static AdaptiveSurfaceFeature> {
    ADAPTIVE_SURFACE_FEATURES
        .iter()
        .find(|feature| feature.surface_form == attr_name)
}

fn surface_profile_enables_feature(
    surface_profile: SurfaceProfile,
    feature: &AdaptiveSurfaceFeature,
) -> bool {
    match surface_profile {
        SurfaceProfile::Stable => matches!(feature.status, AdaptiveFeatureStatus::Stable),
        SurfaceProfile::Experimental => matches!(
            feature.status,
            AdaptiveFeatureStatus::Stable | AdaptiveFeatureStatus::Experimental
        ),
    }
}

fn feature_unavailability_error(
    surface_profile: SurfaceProfile,
    feature: &AdaptiveSurfaceFeature,
    function_name: &str,
) -> String {
    match feature.status {
        AdaptiveFeatureStatus::Experimental => format!(
            "surface feature '@{}' is a compatibility alias and requires --surface experimental for '{}'",
            feature.surface_form, function_name
        ),
        AdaptiveFeatureStatus::Deprecated => format!(
            "surface feature '@{}' is a deprecated alias and is unavailable under --surface {} for '{}'",
            feature.surface_form,
            match surface_profile {
                SurfaceProfile::Stable => "stable",
                SurfaceProfile::Experimental => "experimental",
            },
            function_name
        ),
        AdaptiveFeatureStatus::Stable => format!(
            "surface feature '@{}' is a compatibility alias and is unavailable under --surface {} for '{}'",
            feature.surface_form,
            match surface_profile {
                SurfaceProfile::Stable => "stable",
                SurfaceProfile::Experimental => "experimental",
            },
            function_name
        ),
    }
}

fn format_function_diagnostic(item: &FnAst, message: &str, help: Option<&str>) -> String {
    format_source_diagnostic(&item.source, message, help)
}

fn format_attr_diagnostic(
    _item: &FnAst,
    attr: &RawAttr,
    message: &str,
    help: Option<&str>,
) -> String {
    format_source_diagnostic(&attr.source, message, help)
}

fn canonical_spelling_help(spelling: &str, classification: AdaptiveAliasClassification) -> String {
    format!(
        "did you mean the canonical spelling {}? this {} is kept only for migration guidance.",
        spelling,
        classification.diagnostic_label()
    )
}

fn legacy_noncanonical_spelling_help(attr_name: &str) -> Option<&'static str> {
    match attr_name {
        "yieldpoint" => Some(
            "did you mean the canonical spelling yieldpoint()? control-point markers use statement form, not attributes.",
        ),
        _ => None,
    }
}

fn extern_template_help(name: &str) -> String {
    format!(
        "use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn {}();",
        name
    )
}

pub fn irq_handler_alias_proposal() -> AdaptiveFeatureProposal {
    *adaptive_feature_proposal("irq_handler_alias").expect("irq_handler_alias proposal")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedFunctionFacts {
    ctx_ok: BTreeSet<Ctx>,
    eff_used: BTreeSet<Eff>,
    caps_req: BTreeSet<String>,
    attrs: FunctionAttrs,
}

pub fn lower_to_canonical_executable(
    ast: &ModuleAst,
) -> Result<CanonicalExecutableModule, Vec<String>> {
    lower_to_canonical_executable_with_surface(ast, SurfaceProfile::Stable)
}

pub fn lower_to_canonical_executable_with_surface(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Result<CanonicalExecutableModule, Vec<String>> {
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut extern_declarations = Vec::new();
    let mut names = BTreeSet::new();
    let mut executable_names = BTreeSet::new();
    let mut extern_names = BTreeSet::new();

    for item in &ast.items {
        if !names.insert(item.name.clone()) {
            errors.push(format!("duplicate symbol '{}'", item.name));
        }
        if item.is_extern {
            extern_names.insert(item.name.clone());
        } else {
            executable_names.insert(item.name.clone());
        }
    }

    for item in &ast.items {
        let facts = match normalize_function_facts(item, surface_profile) {
            Ok(facts) => facts,
            Err(errs) => {
                errors.extend(errs);
                continue;
            }
        };

        if item.is_extern {
            extern_declarations.push(CanonicalExecutableExternDecl {
                name: item.name.clone(),
            });
            continue;
        }

        match lower_function_to_canonical_executable(
            item,
            &names,
            &executable_names,
            &extern_names,
            facts,
        ) {
            Ok(function) => functions.push(function),
            Err(errs) => errors.extend(errs),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut module = CanonicalExecutableModule {
        module_caps: ast.module_caps.clone(),
        functions,
        extern_declarations,
    };
    module.canonicalize();
    Ok(module)
}

pub fn lower_canonical_executable_to_krir(
    module: &CanonicalExecutableModule,
) -> Result<ExecutableKrirModule, Vec<String>> {
    let errors = validate_canonical_executable_module(module);
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut call_edges = Vec::new();
    let mut functions = Vec::new();

    for function in &module.functions {
        for op in &function.body.ops {
            match op {
                CanonicalExecutableOp::Call { callee } => call_edges.push(CallEdge {
                    caller: function.name.clone(),
                    callee: callee.clone(),
                }),
            }
        }

        functions.push(ExecutableFunction {
            name: function.name.clone(),
            is_extern: false,
            signature: ExecutableSignature {
                params: function
                    .signature
                    .params
                    .iter()
                    .map(|param| match param {
                        CanonicalExecutableValueType::Unit => ExecutableValueType::Unit,
                    })
                    .collect(),
                result: match function.signature.result {
                    CanonicalExecutableValueType::Unit => ExecutableValueType::Unit,
                },
            },
            facts: ExecutableFacts {
                ctx_ok: function.facts.ctx_ok.clone(),
                eff_used: function.facts.eff_used.clone(),
                caps_req: function.facts.caps_req.clone(),
                attrs: function.facts.attrs.clone(),
            },
            entry_block: "entry".to_string(),
            blocks: vec![ExecutableBlock {
                label: "entry".to_string(),
                ops: function
                    .body
                    .ops
                    .iter()
                    .map(|op| match op {
                        CanonicalExecutableOp::Call { callee } => KrExecutableOp::Call {
                            callee: callee.clone(),
                        },
                    })
                    .collect(),
                terminator: match function.body.terminator {
                    CanonicalExecutableTerminator::ReturnUnit => ExecutableTerminator::Return {
                        value: ExecutableValue::Unit,
                    },
                },
            }],
        });
    }

    let mut lowered = ExecutableKrirModule {
        module_caps: module.module_caps.clone(),
        functions,
        extern_declarations: module
            .extern_declarations
            .iter()
            .map(|decl| ExecutableExternDecl {
                name: decl.name.clone(),
            })
            .collect(),
        call_edges,
    };
    lowered.canonicalize();
    lowered
        .validate()
        .map_err(|err| vec![format!("canonical-exec->krir: {}", err)])?;
    Ok(lowered)
}

fn validate_canonical_executable_module(module: &CanonicalExecutableModule) -> Vec<String> {
    let mut errors = Vec::new();
    let mut function_names = BTreeSet::new();
    let mut extern_names = BTreeSet::new();

    for function in &module.functions {
        if !function_names.insert(function.name.as_str()) {
            errors.push(format!(
                "canonical-exec->krir: duplicate canonical executable function '{}'",
                function.name
            ));
        }

        if !function.signature.params.is_empty() {
            errors.push(format!(
                "canonical-exec->krir: canonical executable function '{}' must not declare parameters in v0.1",
                function.name
            ));
        }
    }

    for extern_decl in &module.extern_declarations {
        if !extern_names.insert(extern_decl.name.as_str()) {
            errors.push(format!(
                "canonical-exec->krir: duplicate canonical executable extern declaration '{}'",
                extern_decl.name
            ));
        }
        if function_names.contains(extern_decl.name.as_str()) {
            errors.push(format!(
                "canonical-exec->krir: extern declaration '{}' duplicates executable function",
                extern_decl.name
            ));
        }
    }

    for function in &module.functions {
        for op in &function.body.ops {
            match op {
                CanonicalExecutableOp::Call { callee } => {
                    if !function_names.contains(callee.as_str())
                        && !extern_names.contains(callee.as_str())
                    {
                        errors.push(format!(
                            "canonical-exec->krir: canonical executable function '{}' calls undeclared target '{}'",
                            function.name, callee
                        ));
                    }
                }
            }
        }
    }

    errors
}

pub fn lower_to_krir(ast: &ModuleAst) -> Result<KrirModule, Vec<String>> {
    lower_to_krir_with_surface(ast, SurfaceProfile::Stable)
}

pub fn lower_to_krir_with_surface(
    ast: &ModuleAst,
    surface_profile: SurfaceProfile,
) -> Result<KrirModule, Vec<String>> {
    let mut errors = Vec::new();
    let mut functions = Vec::new();
    let mut names = BTreeSet::new();
    let module_declares_mmio_structure = module_declares_mmio_structure(ast);
    let module_allows_raw_mmio_literals = module_allows_raw_mmio_literals(&ast.module_caps);
    let (mmio_bases, mmio_base_names, mmio_errors) = collect_mmio_bases(ast);
    let mmio_base_numeric_addrs = collect_mmio_base_numeric_addrs(&mmio_bases);
    let (mmio_registers, mmio_register_rules, mmio_absolute_register_rules, mmio_register_errors) =
        collect_mmio_registers(ast, &mmio_base_names, &mmio_base_numeric_addrs);
    errors.extend(mmio_errors);
    errors.extend(mmio_register_errors);

    for item in &ast.items {
        if !names.insert(item.name.clone()) {
            errors.push(format!("duplicate symbol '{}'", item.name));
        }
    }

    // Build enum_addr_keys: the set of "ENUM::VARIANT" strings that are valid as raw addresses.
    let enum_addr_keys: std::collections::BTreeSet<String> = ast
        .enums
        .iter()
        .flat_map(|e| {
            e.variants
                .iter()
                .map(move |v| format!("{}::{}", e.name, v.name))
        })
        .collect();

    let validation_ctx = MmioValidationCtx {
        declared_bases: &mmio_base_names,
        declared_registers: &mmio_register_rules,
        declared_absolute_registers: &mmio_absolute_register_rules,
        module_declares_mmio_structure,
        module_allows_raw_mmio_literals,
        enum_addr_keys: &enum_addr_keys,
    };
    for item in &ast.items {
        let param_names: std::collections::BTreeSet<&str> =
            item.params.iter().map(|(n, _)| n.as_str()).collect();
        validate_mmio_address_bases_in_stmts(
            &item.body,
            &validation_ctx,
            &param_names,
            &mut errors,
        );
    }

    let mut const_map: std::collections::BTreeMap<String, String> = ast
        .constants
        .iter()
        .map(|c| (c.name.clone(), c.value.clone()))
        .collect();
    for enum_decl in &ast.enums {
        for variant in &enum_decl.variants {
            let key = format!("{}::{}", enum_decl.name, variant.name);
            const_map.insert(key, variant.value.clone());
        }
    }
    for struct_decl in &ast.structs {
        for field in &struct_decl.fields {
            if let Some(offset) = struct_decl.field_offset(&field.name) {
                let key = format!("{}::{}", struct_decl.name, field.name);
                const_map.insert(key, offset.to_string());
            }
        }
    }

    for item in &ast.items {
        match lower_function(item, surface_profile, &const_map) {
            Ok(function) => functions.push(function),
            Err(errs) => errors.extend(errs),
        }
    }

    let mut call_edges = Vec::new();

    for function in &functions {
        if function.is_extern {
            continue;
        }
        for op in &function.ops {
            match op {
                KrirOp::Call { callee } => {
                    if !names.contains(callee) {
                        errors.push(format!(
                            "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                            callee
                        ));
                        continue;
                    }
                    call_edges.push(CallEdge {
                        caller: function.name.clone(),
                        callee: callee.clone(),
                    });
                }
                KrirOp::CallCapture { callee, .. } => {
                    if !names.contains(callee) {
                        errors.push(format!(
                            "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                            callee
                        ));
                        continue;
                    }
                    call_edges.push(CallEdge {
                        caller: function.name.clone(),
                        callee: callee.clone(),
                    });
                }
                KrirOp::BranchIfZero {
                    then_callee,
                    else_callee,
                    ..
                } => {
                    for callee in [then_callee, else_callee] {
                        if !names.contains(callee) {
                            errors.push(format!(
                                "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                                callee
                            ));
                            continue;
                        }
                        call_edges.push(CallEdge {
                            caller: function.name.clone(),
                            callee: callee.clone(),
                        });
                    }
                }
                KrirOp::BranchIfEq {
                    then_callee,
                    else_callee,
                    ..
                } => {
                    for callee in [then_callee, else_callee] {
                        if !names.contains(callee) {
                            errors.push(format!(
                                "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                                callee
                            ));
                            continue;
                        }
                        call_edges.push(CallEdge {
                            caller: function.name.clone(),
                            callee: callee.clone(),
                        });
                    }
                }
                KrirOp::BranchIfMaskNonZero {
                    then_callee,
                    else_callee,
                    ..
                } => {
                    for callee in [then_callee, else_callee] {
                        if !names.contains(callee) {
                            errors.push(format!(
                                "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                                callee
                            ));
                            continue;
                        }
                        call_edges.push(CallEdge {
                            caller: function.name.clone(),
                            callee: callee.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Collect and validate spinlock / per-cpu declarations.
    let declared_lock_classes: BTreeSet<&str> = ast.spinlocks.iter().map(String::as_str).collect();
    let declared_percpu: BTreeSet<&str> = ast.percpu_vars.iter().map(|d| d.name.as_str()).collect();

    for function in &functions {
        for op in &function.ops {
            match op {
                KrirOp::Acquire { lock_class } | KrirOp::Release { lock_class } => {
                    if !declared_lock_classes.contains(lock_class.as_str()) {
                        errors.push(format!(
                            "function '{}' uses undeclared lock class '{}'; add 'spinlock {};' at module scope",
                            function.name, lock_class, lock_class
                        ));
                    }
                }
                KrirOp::PercpuRead { name, .. } | KrirOp::PercpuWrite { name, .. } => {
                    if !declared_percpu.contains(name.as_str()) {
                        errors.push(format!(
                            "function '{}' uses undeclared per-cpu variable '{}'; add 'percpu {}: T;' at module scope",
                            function.name, name, name
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    let lock_classes: Vec<String> = ast.spinlocks.clone();
    let percpu_vars: Vec<KrirPercpuDecl> = ast
        .percpu_vars
        .iter()
        .map(|d| KrirPercpuDecl {
            name: d.name.clone(),
            ty: lower_mmio_scalar_type(d.ty),
        })
        .collect();

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut module = KrirModule {
        module_caps: ast.module_caps.clone(),
        lock_classes,
        percpu_vars,
        mmio_bases,
        mmio_registers,
        functions,
        call_edges,
    };
    module.canonicalize();
    Ok(module)
}

fn collect_mmio_base_numeric_addrs(mmio_bases: &[KrirMmioBaseDecl]) -> BTreeMap<String, u128> {
    mmio_bases
        .iter()
        .filter_map(|base| {
            int_literal_numeric_value(&base.addr).map(|addr| (base.name.clone(), addr))
        })
        .collect()
}

fn lower_function(
    item: &FnAst,
    surface_profile: SurfaceProfile,
    const_map: &std::collections::BTreeMap<String, String>,
) -> Result<Function, Vec<String>> {
    let facts = normalize_function_facts(item, surface_profile)?;

    let mut ops = Vec::new();
    let mut eff_used = facts.eff_used;
    let mut stmt_errors: Vec<String> = Vec::new();
    for stmt in &item.body {
        lower_stmt(stmt, &mut ops, &mut eff_used, const_map, surface_profile, &mut stmt_errors);
    }
    if !stmt_errors.is_empty() {
        return Err(stmt_errors);
    }

    Ok(Function {
        name: item.name.clone(),
        is_extern: item.is_extern,
        params: item
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), lower_param_ty(*ty)))
            .collect(),
        ctx_ok: facts.ctx_ok.into_iter().collect(),
        eff_used: eff_used.into_iter().collect(),
        caps_req: facts.caps_req.into_iter().collect(),
        attrs: facts.attrs,
        ops,
    })
}

fn collect_mmio_bases(ast: &ModuleAst) -> (Vec<KrirMmioBaseDecl>, BTreeSet<String>, Vec<String>) {
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    let mut errors = Vec::new();

    for ParserMmioBaseDecl { name, addr } in &ast.mmio_bases {
        if !names.insert(name.clone()) {
            errors.push(format!("duplicate mmio base '{}'", name));
            continue;
        }
        out.push(KrirMmioBaseDecl {
            name: name.clone(),
            addr: addr.clone(),
        });
    }

    (out, names, errors)
}

#[derive(Debug, Clone)]
struct MmioRegisterRule {
    full_name: String,
    ty: ParserMmioScalarType,
    access: ParserMmioRegAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum MmioOffsetKey {
    Numeric(u128),
    Raw(String),
}

impl MmioOffsetKey {
    fn from_literal(raw: &str) -> Self {
        int_literal_numeric_value(raw)
            .map(Self::Numeric)
            .unwrap_or_else(|| Self::Raw(raw.to_string()))
    }
}

type MmioRegisterRules = BTreeMap<String, BTreeMap<MmioOffsetKey, MmioRegisterRule>>;
type MmioAbsoluteRegisterRules = BTreeMap<u128, MmioRegisterRule>;
const MMIO_SYMBOLIC_BASE_ZERO_OFFSET: &str = "0";
const MMIO_RAW_MODULE_CAP: &str = "MmioRaw";

fn collect_mmio_registers(
    ast: &ModuleAst,
    declared_bases: &BTreeSet<String>,
    declared_base_numeric_addrs: &BTreeMap<String, u128>,
) -> (
    Vec<KrirMmioRegisterDecl>,
    MmioRegisterRules,
    MmioAbsoluteRegisterRules,
    Vec<String>,
) {
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    let mut by_base_and_offset = MmioRegisterRules::new();
    let mut by_absolute_addr = MmioAbsoluteRegisterRules::new();
    let mut errors = Vec::new();

    for ParserMmioRegisterDecl {
        base,
        name,
        offset,
        ty,
        access,
    } in &ast.mmio_registers
    {
        let full_name = format!("{}.{}", base, name);
        if !declared_bases.contains(base) {
            errors.push(format!(
                "undeclared mmio base '{}' in register declaration '{}'",
                base, full_name
            ));
            continue;
        }
        if !names.insert(full_name.clone()) {
            errors.push(format!("duplicate mmio register '{}'", full_name));
            continue;
        }

        let normalized_offset = MmioOffsetKey::from_literal(offset);
        let per_base = by_base_and_offset.entry(base.clone()).or_default();
        if per_base.contains_key(&normalized_offset) {
            errors.push(format!(
                "duplicate mmio register offset '{}' for base '{}'",
                offset, base
            ));
            continue;
        }
        let rule = MmioRegisterRule {
            full_name: full_name.clone(),
            ty: *ty,
            access: *access,
        };

        if let (Some(base_addr), Some(reg_offset)) = (
            declared_base_numeric_addrs.get(base).copied(),
            int_literal_numeric_value(offset),
        ) {
            let Some(abs_addr) = base_addr.checked_add(reg_offset) else {
                errors.push(format!(
                    "mmio register absolute address overflow for '{}'",
                    full_name
                ));
                continue;
            };
            if let Some(existing) = by_absolute_addr.get(&abs_addr) {
                errors.push(format!(
                    "duplicate mmio register absolute address '{}' between '{}' and '{}'",
                    format_mmio_absolute_addr(abs_addr),
                    existing.full_name,
                    full_name
                ));
                continue;
            }
            by_absolute_addr.insert(abs_addr, rule.clone());
        }

        per_base.insert(normalized_offset, rule);
        out.push(KrirMmioRegisterDecl {
            base: base.clone(),
            name: name.clone(),
            offset: offset.clone(),
            ty: lower_mmio_scalar_type(*ty),
            access: lower_mmio_reg_access(*access),
        });
    }

    (out, by_base_and_offset, by_absolute_addr, errors)
}

/// Module-level context shared across all validation calls within one module.
struct MmioValidationCtx<'a> {
    declared_bases: &'a BTreeSet<String>,
    declared_registers: &'a MmioRegisterRules,
    declared_absolute_registers: &'a MmioAbsoluteRegisterRules,
    module_declares_mmio_structure: bool,
    module_allows_raw_mmio_literals: bool,
    enum_addr_keys: &'a std::collections::BTreeSet<String>,
}

fn validate_mmio_address_bases_in_stmts(
    stmts: &[Stmt],
    ctx: &MmioValidationCtx<'_>,
    param_names: &std::collections::BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Critical(inner) => {
                validate_mmio_address_bases_in_stmts(inner, ctx, param_names, errors)
            }
            // unsafe { } suppresses the module-level MmioRaw capability requirement,
            // allowing raw_mmio_read/write within the block without @module_caps(MmioRaw).
            Stmt::Unsafe(inner) => {
                let unsafe_ctx = MmioValidationCtx {
                    module_allows_raw_mmio_literals: true, // unsafe block grants raw MMIO access
                    ..*ctx
                };
                validate_mmio_address_bases_in_stmts(inner, &unsafe_ctx, param_names, errors)
            }
            Stmt::MmioRead { ty, addr, capture } => {
                if let Some(base) = mmio_addr_base_name(addr)
                    && !ctx.declared_bases.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_mmio_read_invocation(*ty, addr, capture.as_deref())
                    ));
                } else {
                    match addr {
                        ParserMmioAddrExpr::Ident(base) => validate_mmio_register_read(
                            base,
                            MMIO_SYMBOLIC_BASE_ZERO_OFFSET,
                            *ty,
                            addr,
                            capture.as_deref(),
                            ctx.declared_registers,
                            errors,
                        ),
                        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => {
                            validate_mmio_register_read(
                                base,
                                offset,
                                *ty,
                                addr,
                                capture.as_deref(),
                                ctx.declared_registers,
                                errors,
                            );
                        }
                        ParserMmioAddrExpr::IntLiteral(literal) => {
                            let matched = validate_mmio_register_read_absolute(
                                literal,
                                *ty,
                                addr,
                                capture.as_deref(),
                                ctx.declared_absolute_registers,
                                errors,
                            );
                            if !matched
                                && ctx.module_declares_mmio_structure
                                && !ctx.module_allows_raw_mmio_literals
                            {
                                errors.push(format!(
                                    "unresolved raw mmio address '{}'; declare a matching mmio_reg or enable raw mmio access",
                                    literal
                                ));
                            }
                        }
                    }
                }
            }
            Stmt::RawMmioRead { ty, addr, capture } => {
                if !ctx.module_allows_raw_mmio_literals {
                    errors.push(format!(
                        "{} requires @module_caps({})",
                        format_raw_mmio_read_invocation(*ty, addr, capture.as_deref()),
                        MMIO_RAW_MODULE_CAP
                    ));
                    continue;
                }
                if let Some(base) = mmio_addr_base_name(addr)
                    && !ctx.declared_bases.contains(base)
                    && !param_names.contains(base)
                    && !ctx.enum_addr_keys.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_raw_mmio_read_invocation(*ty, addr, capture.as_deref())
                    ));
                }
            }
            Stmt::MmioWrite { ty, addr, value } => {
                if let Some(base) = mmio_addr_base_name(addr)
                    && !ctx.declared_bases.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_mmio_write_invocation(*ty, addr, value)
                    ));
                } else {
                    match addr {
                        ParserMmioAddrExpr::Ident(base) => validate_mmio_register_write(
                            base,
                            MMIO_SYMBOLIC_BASE_ZERO_OFFSET,
                            *ty,
                            addr,
                            value,
                            ctx.declared_registers,
                            errors,
                        ),
                        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => {
                            validate_mmio_register_write(
                                base,
                                offset,
                                *ty,
                                addr,
                                value,
                                ctx.declared_registers,
                                errors,
                            );
                        }
                        ParserMmioAddrExpr::IntLiteral(literal) => {
                            let matched = validate_mmio_register_write_absolute(
                                literal,
                                *ty,
                                addr,
                                value,
                                ctx.declared_absolute_registers,
                                errors,
                            );
                            if !matched
                                && ctx.module_declares_mmio_structure
                                && !ctx.module_allows_raw_mmio_literals
                            {
                                errors.push(format!(
                                    "unresolved raw mmio address '{}'; declare a matching mmio_reg or enable raw mmio access",
                                    literal
                                ));
                            }
                        }
                    }
                }
            }
            Stmt::RawMmioWrite { ty, addr, value } => {
                if !ctx.module_allows_raw_mmio_literals {
                    errors.push(format!(
                        "{} requires @module_caps({})",
                        format_raw_mmio_write_invocation(*ty, addr, value),
                        MMIO_RAW_MODULE_CAP
                    ));
                    continue;
                }
                if let Some(base) = mmio_addr_base_name(addr)
                    && !ctx.declared_bases.contains(base)
                    && !param_names.contains(base)
                    && !ctx.enum_addr_keys.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_raw_mmio_write_invocation(*ty, addr, value)
                    ));
                }
            }
            _ => {}
        }
    }
}

fn validate_mmio_register_read(
    base: &str,
    offset: &str,
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    capture: Option<&str>,
    declared_registers: &MmioRegisterRules,
    errors: &mut Vec<String>,
) {
    let invocation = format_mmio_read_invocation(ty, addr, capture);
    let normalized_offset = MmioOffsetKey::from_literal(offset);
    let Some(rule) = declared_registers
        .get(base)
        .and_then(|offsets| offsets.get(&normalized_offset))
    else {
        errors.push(format!(
            "undeclared mmio register offset '{}' for base '{}'",
            offset, base
        ));
        return;
    };

    validate_mmio_read_rule(rule, &invocation, ty, errors);
}

fn validate_mmio_register_write(
    base: &str,
    offset: &str,
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    value: &ParserMmioValueExpr,
    declared_registers: &MmioRegisterRules,
    errors: &mut Vec<String>,
) {
    let invocation = format_mmio_write_invocation(ty, addr, value);
    let normalized_offset = MmioOffsetKey::from_literal(offset);
    let Some(rule) = declared_registers
        .get(base)
        .and_then(|offsets| offsets.get(&normalized_offset))
    else {
        errors.push(format!(
            "undeclared mmio register offset '{}' for base '{}'",
            offset, base
        ));
        return;
    };

    validate_mmio_write_rule(rule, &invocation, ty, errors);
}

fn validate_mmio_register_read_absolute(
    literal: &str,
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    capture: Option<&str>,
    declared_absolute_registers: &MmioAbsoluteRegisterRules,
    errors: &mut Vec<String>,
) -> bool {
    let Some(addr_numeric) = int_literal_numeric_value(literal) else {
        return false;
    };
    let Some(rule) = declared_absolute_registers.get(&addr_numeric) else {
        return false;
    };
    let invocation = format_mmio_read_invocation(ty, addr, capture);
    validate_mmio_read_rule(rule, &invocation, ty, errors);
    true
}

fn validate_mmio_register_write_absolute(
    literal: &str,
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    value: &ParserMmioValueExpr,
    declared_absolute_registers: &MmioAbsoluteRegisterRules,
    errors: &mut Vec<String>,
) -> bool {
    let Some(addr_numeric) = int_literal_numeric_value(literal) else {
        return false;
    };
    let Some(rule) = declared_absolute_registers.get(&addr_numeric) else {
        return false;
    };
    let invocation = format_mmio_write_invocation(ty, addr, value);
    validate_mmio_write_rule(rule, &invocation, ty, errors);
    true
}

fn module_declares_mmio_structure(ast: &ModuleAst) -> bool {
    !ast.mmio_bases.is_empty() || !ast.mmio_registers.is_empty()
}

fn module_allows_raw_mmio_literals(module_caps: &[String]) -> bool {
    module_caps.iter().any(|cap| cap == MMIO_RAW_MODULE_CAP)
}

fn validate_mmio_read_rule(
    rule: &MmioRegisterRule,
    invocation: &str,
    ty: ParserMmioScalarType,
    errors: &mut Vec<String>,
) {
    if rule.access == ParserMmioRegAccess::Wo {
        errors.push(format!(
            "{} violates register access: '{}' is write-only",
            invocation, rule.full_name
        ));
    }
    if ty != rule.ty {
        errors.push(format!(
            "{} width mismatch: register '{}' is {}",
            invocation,
            rule.full_name,
            rule.ty.as_str()
        ));
    }
}

fn validate_mmio_write_rule(
    rule: &MmioRegisterRule,
    invocation: &str,
    ty: ParserMmioScalarType,
    errors: &mut Vec<String>,
) {
    if rule.access == ParserMmioRegAccess::Ro {
        errors.push(format!(
            "{} violates register access: '{}' is read-only",
            invocation, rule.full_name
        ));
    }
    if ty != rule.ty {
        errors.push(format!(
            "{} width mismatch: register '{}' is {}",
            invocation,
            rule.full_name,
            rule.ty.as_str()
        ));
    }
}

fn format_mmio_absolute_addr(addr: u128) -> String {
    format!("0x{addr:x}")
}

fn mmio_addr_base_name(addr: &ParserMmioAddrExpr) -> Option<&str> {
    match addr {
        ParserMmioAddrExpr::Ident(name) => Some(name.as_str()),
        ParserMmioAddrExpr::IdentPlusOffset { base, .. } => Some(base.as_str()),
        ParserMmioAddrExpr::IntLiteral(_) => None,
    }
}

fn normalize_function_facts(
    item: &FnAst,
    surface_profile: SurfaceProfile,
) -> Result<NormalizedFunctionFacts, Vec<String>> {
    let mut errors = Vec::new();
    let mut ctx_ok = BTreeSet::new();
    let mut eff_used = BTreeSet::new();
    let mut caps_req = BTreeSet::new();
    let mut attrs = FunctionAttrs::default();
    let mut saw_ctx = false;
    let mut saw_eff = false;
    let mut saw_caps = false;

    for attr in &item.attrs {
        let name = attr.name.to_ascii_lowercase();
        match name.as_str() {
            "ctx" => {
                saw_ctx = true;
                match parse_ctx_attr(attr) {
                    Ok(values) => ctx_ok.extend(values),
                    Err(msg) => errors.push(format_attr_diagnostic(
                        item,
                        attr,
                        &format!("{} for '{}'", msg, item.name),
                        None,
                    )),
                }
            }
            "eff" => {
                saw_eff = true;
                match parse_eff_attr(attr) {
                    Ok(values) => eff_used.extend(values),
                    Err(msg) => errors.push(format_attr_diagnostic(
                        item,
                        attr,
                        &format!("{} for '{}'", msg, item.name),
                        None,
                    )),
                }
            }
            "caps" => {
                saw_caps = true;
                if let Some(raw) = &attr.args {
                    match split_csv_allow_trailing_comma(raw) {
                        Ok(values) => caps_req.extend(values),
                        Err(_) => errors.push(format_attr_diagnostic(
                            item,
                            attr,
                            &format!(
                                "@caps(...) contains an empty capability entry for '{}'",
                                item.name
                            ),
                            None,
                        )),
                    }
                }
            }
            "irq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Irq);
            }
            "noirq" => {
                saw_ctx = true;
                ctx_ok.clear();
                ctx_ok.insert(Ctx::Boot);
                ctx_ok.insert(Ctx::Thread);
            }
            "alloc" => {
                saw_eff = true;
                eff_used.insert(Eff::Alloc);
            }
            "block" => {
                saw_eff = true;
                eff_used.insert(Eff::Block);
            }
            "preempt_off" => {
                saw_eff = true;
                eff_used.insert(Eff::PreemptOff);
            }
            "noyield" => attrs.noyield = true,
            "critical" => attrs.critical = true,
            "leaf" => attrs.leaf = true,
            "hotpath" => attrs.hotpath = true,
            "lock_budget" => match parse_lock_budget(attr) {
                Ok(v) => attrs.lock_budget = Some(v),
                Err(msg) => errors.push(format_attr_diagnostic(
                    item,
                    attr,
                    &format!("{} for '{}'", msg, item.name),
                    None,
                )),
            },
            "hook" => match attr.args.as_deref() {
                Some("sched_in") => attrs.hook = Some(SchedHook::SchedIn),
                Some("sched_out") => attrs.hook = Some(SchedHook::SchedOut),
                other => errors.push(format_attr_diagnostic(
                    item,
                    attr,
                    &format!(
                        "@hook({}) is invalid for '{}'; expected sched_in or sched_out",
                        other.unwrap_or(""),
                        item.name
                    ),
                    None,
                )),
            },
            "module_caps" => {}
            other => {
                if let Some(feature) = adaptive_surface_feature(other) {
                    if !surface_profile_enables_feature(surface_profile, feature) {
                        errors.push(format_attr_diagnostic(
                            item,
                            attr,
                            &feature_unavailability_error(surface_profile, feature, &item.name),
                            Some(&canonical_spelling_help(
                                feature.canonical_replacement,
                                feature.alias_classification(),
                            )),
                        ));
                        continue;
                    }
                    match feature.lowering_rule {
                        AdaptiveLoweringRule::ContextAlias(ctxs) => {
                            saw_ctx = true;
                            ctx_ok.clear();
                            ctx_ok.extend(ctxs.iter().copied());
                        }
                        AdaptiveLoweringRule::EffectAlias(effs) => {
                            saw_eff = true;
                            eff_used.extend(effs.iter().copied());
                        }
                    }
                } else {
                    let help = legacy_noncanonical_spelling_help(other);
                    let message = if help.is_some() {
                        format!(
                            "legacy spelling '@{}' is non-canonical and is not accepted on function '{}'",
                            other, item.name
                        )
                    } else {
                        format!("unknown attribute '@{}' on function '{}'", other, item.name)
                    };
                    errors.push(format_attr_diagnostic(item, attr, &message, help));
                }
            }
        }
    }

    if item.is_extern {
        if !saw_ctx {
            let help = extern_template_help(&item.name);
            errors.push(format_function_diagnostic(
                item,
                &format!(
                    "extern '{}' must declare @ctx(...) facts explicitly",
                    item.name
                ),
                Some(&help),
            ));
        }
        if !saw_eff {
            let help = extern_template_help(&item.name);
            errors.push(format_function_diagnostic(
                item,
                &format!(
                    "extern '{}' must declare @eff(...) facts explicitly",
                    item.name
                ),
                Some(&help),
            ));
        }
        if !saw_caps {
            let help = extern_template_help(&item.name);
            errors.push(format_function_diagnostic(
                item,
                &format!(
                    "EXTERN_CAPS_CONTRACT_REQUIRED: extern '{}' must declare @caps(...) facts explicitly",
                    item.name
                ),
                Some(&help),
            ));
        }
    } else {
        if !saw_ctx {
            // Default is conservative and excludes IRQ/NMI.
            ctx_ok.insert(Ctx::Boot);
            ctx_ok.insert(Ctx::Thread);
        }
        if !saw_eff {
            // Default is no declared effects.
        }
        if !saw_caps {
            // Default is no required capabilities.
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(NormalizedFunctionFacts {
        ctx_ok,
        eff_used,
        caps_req,
        attrs,
    })
}

fn lower_function_to_canonical_executable(
    item: &FnAst,
    all_names: &BTreeSet<String>,
    executable_names: &BTreeSet<String>,
    extern_names: &BTreeSet<String>,
    facts: NormalizedFunctionFacts,
) -> Result<CanonicalExecutableFunction, Vec<String>> {
    let mut ops = Vec::new();
    let mut errors = Vec::new();
    lower_stmts_to_canonical_executable(
        &item.body,
        &item.name,
        all_names,
        executable_names,
        extern_names,
        &mut ops,
        &mut errors,
    );

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(CanonicalExecutableFunction {
        name: item.name.clone(),
        signature: CanonicalExecutableSignature {
            params: vec![],
            result: CanonicalExecutableValueType::Unit,
        },
        facts: CanonicalExecutableFacts {
            ctx_ok: facts.ctx_ok.into_iter().collect(),
            eff_used: facts.eff_used.into_iter().collect(),
            caps_req: facts.caps_req.into_iter().collect(),
            attrs: facts.attrs,
        },
        body: CanonicalExecutableBody {
            ops,
            terminator: CanonicalExecutableTerminator::ReturnUnit,
        },
    })
}

fn lower_stmts_to_canonical_executable(
    stmts: &[Stmt],
    function_name: &str,
    all_names: &BTreeSet<String>,
    executable_names: &BTreeSet<String>,
    extern_names: &BTreeSet<String>,
    ops: &mut Vec<CanonicalExecutableOp>,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Call(callee) => {
                if executable_names.contains(callee) || extern_names.contains(callee) {
                    ops.push(CanonicalExecutableOp::Call {
                        callee: callee.clone(),
                    });
                } else if !all_names.contains(callee) {
                    errors.push(format!(
                        "undefined symbol '{}': add extern declaration with canonical facts (@ctx/@eff/@caps)",
                        callee
                    ));
                } else {
                    errors.push(format!(
                        "canonical-exec: function '{}' calls undeclared target '{}'",
                        function_name, callee
                    ));
                }
            }
            Stmt::CallCapture { callee, slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_call_capture_invocation(callee, slot)
            )),
            Stmt::Critical(_) => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported critical region",
                function_name
            )),
            Stmt::Unsafe(inner) => lower_stmts_to_canonical_executable(
                inner,
                function_name,
                all_names,
                executable_names,
                extern_names,
                ops,
                errors,
            ),
            Stmt::YieldPoint => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported yieldpoint()",
                function_name
            )),
            Stmt::AllocPoint => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported allocpoint()",
                function_name
            )),
            Stmt::BlockPoint => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported blockpoint()",
                function_name
            )),
            Stmt::Acquire(lock_class) => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported acquire({})",
                function_name, lock_class
            )),
            Stmt::Release(lock_class) => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported release({})",
                function_name, lock_class
            )),
            Stmt::ReturnSlot { slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_return_slot_invocation(slot)
            )),
            Stmt::BranchIfZero {
                slot,
                then_callee,
                else_callee,
            } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_branch_if_zero_invocation(slot, then_callee, else_callee)
            )),
            Stmt::BranchIfEq {
                slot,
                compare_value,
                then_callee,
                else_callee,
            } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_branch_if_eq_invocation(slot, compare_value, then_callee, else_callee)
            )),
            Stmt::BranchIfMaskNonZero {
                slot,
                mask_value,
                then_callee,
                else_callee,
            } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_branch_if_mask_nonzero_invocation(
                    slot,
                    mask_value,
                    then_callee,
                    else_callee
                )
            )),
            Stmt::StackCell { ty, cell } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_stack_cell_invocation(*ty, cell)
            )),
            Stmt::CellStore { ty, cell, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_cell_store_invocation(*ty, cell, value)
            )),
            Stmt::CellLoad { ty, cell, slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_cell_load_invocation(*ty, cell, slot)
            )),
            Stmt::MmioRead { ty, addr, capture } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_mmio_read_invocation(*ty, addr, capture.as_deref())
            )),
            Stmt::MmioWrite { ty, addr, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_mmio_write_invocation(*ty, addr, value)
            )),
            Stmt::RawMmioRead { ty, addr, capture } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_raw_mmio_read_invocation(*ty, addr, capture.as_deref())
            )),
            Stmt::RawMmioWrite { ty, addr, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_raw_mmio_write_invocation(*ty, addr, value)
            )),
            Stmt::SliceLen { slice, slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported slice_len({}, {})",
                function_name, slice, slot
            )),
            Stmt::SlicePtr { slice, slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported slice_ptr({}, {})",
                function_name, slice, slot
            )),
            Stmt::PercpuRead { ty, name, slot } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported percpu_read<{}>({}, {})",
                function_name,
                ty.as_str(),
                name,
                slot
            )),
            Stmt::PercpuWrite { ty, name, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported percpu_write<{}>({}, {})",
                function_name,
                ty.as_str(),
                name,
                value.as_source()
            )),
            Stmt::CellArithImm { ty, cell, op, .. } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported cell_{}<{}>({})",
                function_name,
                op.as_str(),
                ty.as_str(),
                cell
            )),
        }
    }
}

fn lower_stmt(
    stmt: &Stmt,
    ops: &mut Vec<KrirOp>,
    eff_used: &mut BTreeSet<Eff>,
    const_map: &std::collections::BTreeMap<String, String>,
    surface_profile: SurfaceProfile,
    errors: &mut Vec<String>,
) {
    match stmt {
        Stmt::Call(callee) => ops.push(KrirOp::Call {
            callee: callee.clone(),
        }),
        Stmt::CallCapture { callee, slot } => ops.push(KrirOp::CallCapture {
            callee: callee.clone(),
            capture_slot: slot.clone(),
        }),
        Stmt::Critical(inner) => {
            ops.push(KrirOp::CriticalEnter);
            for stmt in inner {
                lower_stmt(stmt, ops, eff_used, const_map, surface_profile, errors);
            }
            ops.push(KrirOp::CriticalExit);
        }
        Stmt::Unsafe(inner) => {
            for stmt in inner {
                lower_stmt(stmt, ops, eff_used, const_map, surface_profile, errors);
            }
        }
        Stmt::YieldPoint => {
            ops.push(KrirOp::YieldPoint);
            eff_used.insert(Eff::Yield);
        }
        Stmt::AllocPoint => {
            ops.push(KrirOp::AllocPoint);
            eff_used.insert(Eff::Alloc);
        }
        Stmt::BlockPoint => {
            ops.push(KrirOp::BlockPoint);
            eff_used.insert(Eff::Block);
        }
        Stmt::Acquire(lock_class) => ops.push(KrirOp::Acquire {
            lock_class: lock_class.clone(),
        }),
        Stmt::Release(lock_class) => ops.push(KrirOp::Release {
            lock_class: lock_class.clone(),
        }),
        Stmt::ReturnSlot { slot } => ops.push(KrirOp::ReturnSlot { slot: slot.clone() }),
        Stmt::BranchIfZero {
            slot,
            then_callee,
            else_callee,
        } => ops.push(KrirOp::BranchIfZero {
            slot: slot.clone(),
            then_callee: then_callee.clone(),
            else_callee: else_callee.clone(),
        }),
        Stmt::BranchIfEq {
            slot,
            compare_value,
            then_callee,
            else_callee,
        } => ops.push(KrirOp::BranchIfEq {
            slot: slot.clone(),
            compare_value: compare_value.clone(),
            then_callee: then_callee.clone(),
            else_callee: else_callee.clone(),
        }),
        Stmt::BranchIfMaskNonZero {
            slot,
            mask_value,
            then_callee,
            else_callee,
        } => ops.push(KrirOp::BranchIfMaskNonZero {
            slot: slot.clone(),
            mask_value: mask_value.clone(),
            then_callee: then_callee.clone(),
            else_callee: else_callee.clone(),
        }),
        Stmt::StackCell { ty, cell } => ops.push(KrirOp::StackCell {
            ty: lower_mmio_scalar_type(*ty),
            cell: cell.clone(),
        }),
        Stmt::CellStore { ty, cell, value } => ops.push(KrirOp::StackStore {
            ty: lower_mmio_scalar_type(*ty),
            cell: cell.clone(),
            value: lower_mmio_value_expr(value, const_map),
        }),
        Stmt::CellLoad { ty, cell, slot } => ops.push(KrirOp::StackLoad {
            ty: lower_mmio_scalar_type(*ty),
            cell: cell.clone(),
            slot: slot.clone(),
        }),
        Stmt::MmioRead { ty, addr, capture } => {
            ops.push(KrirOp::MmioRead {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr, const_map),
                capture_slot: capture.clone(),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::MmioWrite { ty, addr, value } => {
            ops.push(KrirOp::MmioWrite {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr, const_map),
                value: lower_mmio_value_expr(value, const_map),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::RawMmioRead { ty, addr, capture } => {
            ops.push(KrirOp::RawMmioRead {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr, const_map),
                capture_slot: capture.clone(),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::RawMmioWrite { ty, addr, value } => {
            ops.push(KrirOp::RawMmioWrite {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr, const_map),
                value: lower_mmio_value_expr(value, const_map),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::SliceLen { slice, slot } => ops.push(KrirOp::SliceLen {
            slice: slice.clone(),
            slot: slot.clone(),
        }),
        Stmt::SlicePtr { slice, slot } => ops.push(KrirOp::SlicePtr {
            slice: slice.clone(),
            slot: slot.clone(),
        }),
        Stmt::PercpuRead { ty, name, slot } => ops.push(KrirOp::PercpuRead {
            ty: lower_mmio_scalar_type(*ty),
            name: name.clone(),
            slot: slot.clone(),
        }),
        Stmt::PercpuWrite { ty, name, value } => ops.push(KrirOp::PercpuWrite {
            ty: lower_mmio_scalar_type(*ty),
            name: name.clone(),
            value: lower_mmio_value_expr(value, const_map),
        }),
        Stmt::CellArithImm { ty, cell, op, imm } => {
            if surface_profile != SurfaceProfile::Experimental {
                errors.push(format!(
                    "cell_{}<{}> requires --surface experimental",
                    op.as_str(),
                    ty.as_str()
                ));
                return;
            }
            ops.push(KrirOp::CellArithImm {
                ty: lower_mmio_scalar_type(*ty),
                cell: cell.clone(),
                arith_op: lower_arith_op(*op),
                imm: *imm,
            });
        }
    }
}

fn lower_mmio_scalar_type(ty: ParserMmioScalarType) -> KrirMmioScalarType {
    match ty {
        ParserMmioScalarType::U8 => KrirMmioScalarType::U8,
        ParserMmioScalarType::U16 => KrirMmioScalarType::U16,
        ParserMmioScalarType::U32 => KrirMmioScalarType::U32,
        ParserMmioScalarType::U64 => KrirMmioScalarType::U64,
    }
}

fn lower_arith_op(op: ParserArithOp) -> KrirArithOp {
    match op {
        ParserArithOp::Add => KrirArithOp::Add,
        ParserArithOp::Sub => KrirArithOp::Sub,
        ParserArithOp::And => KrirArithOp::And,
        ParserArithOp::Or => KrirArithOp::Or,
        ParserArithOp::Xor => KrirArithOp::Xor,
        ParserArithOp::Shl => KrirArithOp::Shl,
        ParserArithOp::Shr => KrirArithOp::Shr,
    }
}

fn lower_param_ty(ty: ParserParamTy) -> KrirParamTy {
    match ty {
        ParserParamTy::Scalar(s) => KrirParamTy::Scalar {
            ty: lower_mmio_scalar_type(s),
        },
        ParserParamTy::Slice(elem) => KrirParamTy::Slice {
            elem: lower_mmio_scalar_type(elem),
        },
    }
}

fn lower_mmio_reg_access(access: ParserMmioRegAccess) -> KrirMmioRegAccess {
    match access {
        ParserMmioRegAccess::Ro => KrirMmioRegAccess::Ro,
        ParserMmioRegAccess::Wo => KrirMmioRegAccess::Wo,
        ParserMmioRegAccess::Rw => KrirMmioRegAccess::Rw,
    }
}

fn lower_mmio_addr_expr(
    expr: &ParserMmioAddrExpr,
    const_map: &std::collections::BTreeMap<String, String>,
) -> KrirMmioAddrExpr {
    match expr {
        ParserMmioAddrExpr::Ident(name) => {
            if let Some(lit) = const_map.get(name.as_str()) {
                KrirMmioAddrExpr::IntLiteral { value: lit.clone() }
            } else {
                KrirMmioAddrExpr::Ident { name: name.clone() }
            }
        }
        ParserMmioAddrExpr::IntLiteral(value) => KrirMmioAddrExpr::IntLiteral {
            value: value.clone(),
        },
        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => {
            // Resolve base and offset independently from const_map (catches
            // enum variants and struct field offsets on either side).
            let base_resolved = const_map.get(base.as_str()).map(String::as_str);
            let offset_resolved = const_map.get(offset.as_str()).map(String::as_str);

            let base_num = base_resolved
                .and_then(int_literal_numeric_value)
                .or_else(|| int_literal_numeric_value(base));
            let off_num = offset_resolved
                .and_then(int_literal_numeric_value)
                .or_else(|| int_literal_numeric_value(offset));

            match (base_num, off_num) {
                (Some(b), Some(o)) => {
                    // Both sides resolve to numbers — fold to a single IntLiteral.
                    KrirMmioAddrExpr::IntLiteral {
                        value: format!("0x{:X}", b.wrapping_add(o) as u64),
                    }
                }
                _ => {
                    // At least one side is a symbolic mmio-base ident; emit
                    // IdentPlusOffset with the offset resolved to its literal
                    // string if possible (so the KRIR backend sees a number).
                    let resolved_offset = offset_resolved.unwrap_or(offset.as_str()).to_string();
                    KrirMmioAddrExpr::IdentPlusOffset {
                        base: base.clone(),
                        offset: resolved_offset,
                    }
                }
            }
        }
    }
}

fn lower_mmio_value_expr(
    expr: &ParserMmioValueExpr,
    const_map: &std::collections::BTreeMap<String, String>,
) -> KrirMmioValueExpr {
    match expr {
        ParserMmioValueExpr::Ident(name) => {
            // Resolve named constants to integer literals at HIR lowering time.
            if let Some(lit) = const_map.get(name.as_str()) {
                KrirMmioValueExpr::IntLiteral { value: lit.clone() }
            } else {
                KrirMmioValueExpr::Ident { name: name.clone() }
            }
        }
        ParserMmioValueExpr::IntLiteral(value) => KrirMmioValueExpr::IntLiteral {
            value: value.clone(),
        },
    }
}

fn format_mmio_read_invocation(
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    capture: Option<&str>,
) -> String {
    match capture {
        Some(capture) => format!(
            "mmio_read<{}>({}, {})",
            ty.as_str(),
            addr.as_source(),
            capture
        ),
        None => format!("mmio_read<{}>({})", ty.as_str(), addr.as_source()),
    }
}

fn format_mmio_write_invocation(
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    value: &ParserMmioValueExpr,
) -> String {
    format!(
        "mmio_write<{}>({}, {})",
        ty.as_str(),
        addr.as_source(),
        value.as_source()
    )
}

fn format_raw_mmio_read_invocation(
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    capture: Option<&str>,
) -> String {
    match capture {
        Some(capture) => format!(
            "raw_mmio_read<{}>({}, {})",
            ty.as_str(),
            addr.as_source(),
            capture
        ),
        None => format!("raw_mmio_read<{}>({})", ty.as_str(), addr.as_source()),
    }
}

fn format_raw_mmio_write_invocation(
    ty: ParserMmioScalarType,
    addr: &ParserMmioAddrExpr,
    value: &ParserMmioValueExpr,
) -> String {
    format!(
        "raw_mmio_write<{}>({}, {})",
        ty.as_str(),
        addr.as_source(),
        value.as_source()
    )
}

fn format_branch_if_zero_invocation(slot: &str, then_callee: &str, else_callee: &str) -> String {
    format!("branch_if_zero({}, {}, {})", slot, then_callee, else_callee)
}

fn format_call_capture_invocation(callee: &str, slot: &str) -> String {
    format!("call_capture({}, {})", callee, slot)
}

fn format_return_slot_invocation(slot: &str) -> String {
    format!("return_slot({})", slot)
}

fn format_stack_cell_invocation(ty: ParserMmioScalarType, cell: &str) -> String {
    format!("stack_cell<{}>({})", ty.as_str(), cell)
}

fn format_cell_store_invocation(
    ty: ParserMmioScalarType,
    cell: &str,
    value: &ParserMmioValueExpr,
) -> String {
    format!(
        "cell_store<{}>({}, {})",
        ty.as_str(),
        cell,
        value.as_source()
    )
}

fn format_cell_load_invocation(ty: ParserMmioScalarType, cell: &str, slot: &str) -> String {
    format!("cell_load<{}>({}, {})", ty.as_str(), cell, slot)
}

fn format_branch_if_eq_invocation(
    slot: &str,
    compare_value: &str,
    then_callee: &str,
    else_callee: &str,
) -> String {
    format!(
        "branch_if_eq({}, {}, {}, {})",
        slot, compare_value, then_callee, else_callee
    )
}

fn format_branch_if_mask_nonzero_invocation(
    slot: &str,
    mask_value: &str,
    then_callee: &str,
    else_callee: &str,
) -> String {
    format!(
        "branch_if_mask_nonzero({}, {}, {}, {})",
        slot, mask_value, then_callee, else_callee
    )
}

fn parse_ctx_attr(attr: &RawAttr) -> Result<Vec<Ctx>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@ctx(...) requires a context list".to_string());
    };

    let mut out = Vec::new();
    let tokens = split_csv_allow_trailing_comma(args)
        .map_err(|_| "@ctx(...) contains an empty context entry".to_string())?;
    for token in tokens {
        let ctx = match token.trim().to_ascii_lowercase().as_str() {
            "boot" => Ctx::Boot,
            "thread" => Ctx::Thread,
            "irq" => Ctx::Irq,
            "nmi" => Ctx::Nmi,
            _ => return Err(format!("unknown context '{}'", token)),
        };
        out.push(ctx);
    }
    Ok(out)
}

fn parse_eff_attr(attr: &RawAttr) -> Result<Vec<Eff>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@eff(...) requires an effect list".to_string());
    };

    let mut out = Vec::new();
    let tokens = split_csv_allow_trailing_comma(args)
        .map_err(|_| "@eff(...) contains an empty effect entry".to_string())?;
    for token in tokens {
        let eff = match token.trim().to_ascii_lowercase().as_str() {
            "alloc" => Eff::Alloc,
            "block" => Eff::Block,
            "preempt_off" => Eff::PreemptOff,
            "ioport" => Eff::Ioport,
            "mmio" => Eff::Mmio,
            "dma_map" => Eff::DmaMap,
            "yield" => Eff::Yield,
            _ => return Err(format!("unknown effect '{}'", token)),
        };
        out.push(eff);
    }
    Ok(out)
}

fn parse_lock_budget(attr: &RawAttr) -> Result<u64, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@lock_budget(N) requires a number".to_string());
    };
    let trimmed = args.trim();
    trimmed
        .parse::<u64>()
        .map_err(|_| format!("invalid lock budget '{}'", trimmed))
}

#[cfg(test)]
mod tests {
    use super::{
        AdaptiveFeatureProposal, AdaptiveFeatureStatus, AdaptiveLoweringRule,
        AdaptiveSurfaceFeature, CanonicalExecutableTerminator, SurfaceProfile,
        adaptive_feature_promotion_plan_with, adaptive_feature_promotion_readiness_with,
        adaptive_feature_proposal, adaptive_feature_proposals, adaptive_surface_features,
        frontend_canonical_edit_plan, frontend_canonical_findings,
        frontend_migration_features_for_profile, frontend_migration_preview,
        lower_canonical_executable_to_krir, lower_to_canonical_executable_with_surface,
        lower_to_krir, lower_to_krir_with_surface, surface_profile_enables_feature,
        validate_adaptive_feature_governance, validate_adaptive_feature_governance_with,
    };
    use parser::parse_module;
    use proptest::prelude::*;
    use serde_json::json;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn lower_to_krir_never_panics_on_random_parseable_input(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let input = String::from_utf8_lossy(&bytes).to_string();
            let parsed = std::panic::catch_unwind(|| parse_module(&input));
            prop_assert!(parsed.is_ok());

            let parse_outcome = match parsed {
                Ok(value) => value,
                Err(_) => return Ok(()),
            };
            if let Ok(ast) = parse_outcome {
                let lowered = std::panic::catch_unwind(|| lower_to_krir(&ast));
                prop_assert!(lowered.is_ok());
            }
        }
    }

    #[test]
    fn irq_handler_alias_is_rejected_in_stable_surface() {
        let ast = parse_module("@irq_handler fn isr() { }").expect("parse");
        let errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("stable surface must reject irq_handler");
        assert_eq!(
            errs,
            vec![
                "surface feature '@irq_handler' is a compatibility alias and requires --surface experimental for 'isr' at 1:1\n  1 | @irq_handler fn isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)? this compatibility alias is kept only for migration guidance."
            ]
        );
    }

    #[test]
    fn adaptive_surface_aliases_lower_identically_to_canonical_forms() {
        let cases = [
            ("@irq_handler fn isr() { }", "@ctx(irq) fn isr() { }"),
            (
                "@thread_entry fn worker() { }",
                "@ctx(thread) fn worker() { }",
            ),
            ("@may_block fn worker() { }", "@eff(block) fn worker() { }"),
        ];

        for (alias_src, canonical_src) in cases {
            let alias_ast = parse_module(alias_src).expect("parse alias");
            let canonical_ast = parse_module(canonical_src).expect("parse canonical");
            let alias = lower_to_krir_with_surface(&alias_ast, SurfaceProfile::Experimental)
                .expect("experimental alias lowering");
            let canonical =
                lower_to_krir_with_surface(&canonical_ast, SurfaceProfile::Stable).expect("lower");
            assert_eq!(alias, canonical, "alias '{}' drifted", alias_src);
        }
    }

    #[test]
    fn canonical_fact_lists_allow_optional_trailing_commas_without_semantic_drift() {
        let trailing_src = "@module_caps(PhysMap,);\nextern @ctx(thread, boot,) @eff(block,) @caps() fn sleep();\n@ctx(thread, boot,)\n@caps(PhysMap,)\nfn entry() { helper(); }\n@ctx(thread,)\nfn helper() { sleep(); }";
        let canonical_src = "@module_caps(PhysMap);\nextern @ctx(thread, boot) @eff(block) @caps() fn sleep();\n@ctx(thread, boot)\n@caps(PhysMap)\nfn entry() { helper(); }\n@ctx(thread)\nfn helper() { sleep(); }";

        let trailing_ast = parse_module(trailing_src).expect("parse trailing comma facts");
        let canonical_ast = parse_module(canonical_src).expect("parse canonical facts");
        let trailing = lower_to_krir(&trailing_ast).expect("lower trailing comma facts");
        let canonical = lower_to_krir(&canonical_ast).expect("lower canonical facts");

        assert_eq!(trailing, canonical);
    }

    #[test]
    fn malformed_canonical_fact_lists_reject_empty_entries_deterministically() {
        let cases = [
            (
                "@ctx(,thread)\nfn entry() { }",
                "@ctx(...) contains an empty context entry for 'entry' at 1:1\n  1 | @ctx(,thread)",
            ),
            (
                "@eff(block,, yield)\nfn worker() { }",
                "@eff(...) contains an empty effect entry for 'worker' at 1:1\n  1 | @eff(block,, yield)",
            ),
            (
                "@caps(PhysMap, , MmioRaw)\nfn entry() { }",
                "@caps(...) contains an empty capability entry for 'entry' at 1:1\n  1 | @caps(PhysMap, , MmioRaw)",
            ),
        ];

        for (src, expected) in cases {
            let ast = parse_module(src).expect("parse malformed attribute list");
            let errs = lower_to_krir(&ast).expect_err("empty fact entries must fail");
            assert_eq!(errs, vec![expected.to_string()], "source '{}'", src);
        }
    }

    #[test]
    fn canonical_executable_surface_aliases_normalize_to_supported_semantics() {
        let ast = parse_module("@thread_entry fn worker() { helper(); }\nfn helper() { }")
            .expect("parse");
        let lowered = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical executable lowering");

        assert_eq!(lowered.functions.len(), 2);
        let worker = lowered
            .functions
            .iter()
            .find(|function| function.name == "worker")
            .expect("worker");
        assert_eq!(worker.facts.ctx_ok, vec![krir::Ctx::Thread]);
        assert_eq!(
            serde_json::to_value(&worker.body.ops).expect("serialize ops"),
            json!([{"op": "call", "callee": "helper"}])
        );
        assert_eq!(
            worker.body.terminator,
            CanonicalExecutableTerminator::ReturnUnit
        );
    }

    #[test]
    fn canonical_executable_semantics_preserve_call_order() {
        let ast =
            parse_module("fn entry() { first(); second(); }\nfn first() { }\nfn second() { }")
                .expect("parse");
        let mut lowered = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical executable lowering");
        lowered.canonicalize();

        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry");
        let callees = entry
            .body
            .ops
            .iter()
            .map(|op| match op {
                super::CanonicalExecutableOp::Call { callee } => callee.as_str(),
            })
            .collect::<Vec<_>>();
        assert_eq!(callees, vec!["first", "second"]);
    }

    #[test]
    fn canonical_executable_rejects_unsupported_critical_region_deterministically() {
        let ast =
            parse_module("fn entry() { critical { helper(); } }\nfn helper() { }").expect("parse");
        let errs = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("critical region must be rejected");
        assert_eq!(
            errs,
            vec!["canonical-exec: function 'entry' contains unsupported critical region"]
        );
    }

    #[test]
    fn canonical_executable_rejects_typed_mmio_deterministically() {
        let ast = parse_module("fn entry() { mmio_read<u16>(mmio_addr); }").expect("parse");
        let errs = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("typed mmio must be rejected in canonical executable lowering");
        assert_eq!(
            errs,
            vec!["canonical-exec: function 'entry' contains unsupported mmio_read<u16>(mmio_addr)"]
        );
    }

    #[test]
    fn analysis_krir_lowering_preserves_typed_mmio_ops() {
        let ast = parse_module(
            "mmio base = 0x1000;\nmmio_reg base.SR = 4 : u16 ro;\nmmio_reg base.DR = 8 : u64 rw;\nfn entry() { mmio_read<u16>(base + 4); mmio_write<u64>(base + 8, payload); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([
                {
                    "op": "mmio_read",
                    "ty": "u16",
                    "addr": {"kind": "ident_plus_offset", "base": "base", "offset": "4"}
                },
                {
                    "op": "mmio_write",
                    "ty": "u64",
                    "addr": {"kind": "ident_plus_offset", "base": "base", "offset": "8"},
                    "value": {"kind": "ident", "name": "payload"}
                }
            ])
        );
        assert_eq!(entry.eff_used, vec![krir::Eff::Mmio]);
    }

    #[test]
    fn analysis_krir_lowering_resolves_declared_mmio_bases() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.SR = 0x00 : u32 ro;\nmmio_reg UART0.DR = 4 : u8 rw;\nfn entry() { mmio_read<u32>(UART0); mmio_write<u8>(UART0 + 4, 0xff); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        assert_eq!(
            lowered.mmio_bases,
            vec![krir::MmioBaseDecl {
                name: "UART0".to_string(),
                addr: "0x1000".to_string()
            }]
        );
    }

    #[test]
    fn analysis_krir_lowering_allows_symbolic_mmio_base_as_offset_zero_register() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 0x00 : u32 rw;\nfn entry() { mmio_write<u32>(UART0, value); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([{
                "op": "mmio_write",
                "ty": "u32",
                "addr": {"kind": "ident", "name": "UART0"},
                "value": {"kind": "ident", "name": "value"}
            }])
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_symbolic_mmio_base_without_offset_zero_register() {
        let ast = parse_module("mmio UART0 = 0x1000;\nfn entry() { mmio_write<u32>(UART0, x); }")
            .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec!["undeclared mmio register offset '0' for base 'UART0'".to_string()]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_symbolic_mmio_base_offset_zero_access_mismatch() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.SR = 0 : u32 ro;\nfn entry() { mmio_write<u32>(UART0, x); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "mmio_write<u32>(UART0, x) violates register access: 'UART0.SR' is read-only"
                    .to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_symbolic_mmio_base_offset_zero_width_mismatch() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.CR = 0x00 : u16 rw;\nfn entry() { mmio_write<u32>(UART0, x); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "mmio_write<u32>(UART0, x) width mismatch: register 'UART0.CR' is u16".to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_allows_int_literal_mmio_address_when_matching_declared_register() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 0x00 : u32 rw;\nfn entry() { mmio_write<u32>(0x1000, x); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([{
                "op": "mmio_write",
                "ty": "u32",
                "addr": {"kind": "int_literal", "value": "0x1000"},
                "value": {"kind": "ident", "name": "x"}
            }])
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_int_literal_mmio_address_access_and_width_mismatch() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.SR = 0x04 : u32 ro;\nmmio_reg UART0.CR = 0x08 : u16 rw;\nfn entry() { mmio_write<u32>(0x1004, x); mmio_write<u32>(0x1008, x); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "mmio_write<u32>(0x1004, x) violates register access: 'UART0.SR' is read-only"
                    .to_string(),
                "mmio_write<u32>(0x1008, x) width mismatch: register 'UART0.CR' is u16".to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_matches_mmio_register_offsets_across_literal_spellings() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 4 : u32 rw;\nfn entry() { mmio_write<u32>(UART0 + 0x04, value); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([{
                "op": "mmio_write",
                "ty": "u32",
                "addr": {"kind": "ident_plus_offset", "base": "UART0", "offset": "0x04"},
                "value": {"kind": "ident", "name": "value"}
            }])
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_duplicate_mmio_register_semantic_offset() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 4 : u32 rw;\nmmio_reg UART0.SR = 0x04 : u32 ro;\nfn entry() { mmio_read<u32>(UART0 + 4); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec!["duplicate mmio register offset '0x04' for base 'UART0'".to_string()]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_duplicate_mmio_register_absolute_address_collision() {
        let ast = parse_module(
            "mmio A = 0x1000;\nmmio B = 0x0FFC;\nmmio_reg A.R0 = 0x04 : u32 rw;\nmmio_reg B.R1 = 0x08 : u32 rw;\nfn entry() { mmio_read<u32>(A + 4); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "duplicate mmio register absolute address '0x1004' between 'A.R0' and 'B.R1'"
                    .to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_preserves_mmio_register_metadata() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 0x00 : u32 rw;\nmmio_reg UART0.SR = 0x04 : u32 ro;\nfn entry() { mmio_read<u32>(UART0 + 0x04); mmio_write<u32>(UART0 + 0x00, value); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        assert_eq!(
            lowered.mmio_registers,
            vec![
                krir::MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "DR".to_string(),
                    offset: "0x00".to_string(),
                    ty: krir::MmioScalarType::U32,
                    access: krir::MmioRegAccess::Rw,
                },
                krir::MmioRegisterDecl {
                    base: "UART0".to_string(),
                    name: "SR".to_string(),
                    offset: "0x04".to_string(),
                    ty: krir::MmioScalarType::U32,
                    access: krir::MmioRegAccess::Ro,
                },
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_undeclared_mmio_register_offset() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.DR = 0x00 : u32 rw;\nfn entry() { mmio_read<u32>(UART0 + 0x44); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec!["undeclared mmio register offset '0x44' for base 'UART0'".to_string()]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_mmio_register_access_and_width_mismatch() {
        let ast = parse_module(
            "mmio UART0 = 0x1000;\nmmio_reg UART0.SR = 0x04 : u32 ro;\nmmio_reg UART0.CR = 0x08 : u16 rw;\nfn entry() { mmio_write<u32>(UART0 + 0x04, x); mmio_write<u32>(UART0 + 0x08, x); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "mmio_write<u32>(UART0 + 0x04, x) violates register access: 'UART0.SR' is read-only"
                    .to_string(),
                "mmio_write<u32>(UART0 + 0x08, x) width mismatch: register 'UART0.CR' is u16"
                    .to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_mmio_register_with_undeclared_base() {
        let ast = parse_module(
            "mmio_reg UART0.DR = 0x00 : u32 rw;\nfn entry() { mmio_read<u32>(0x1000); }",
        )
        .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "undeclared mmio base 'UART0' in register declaration 'UART0.DR'".to_string(),
                "unresolved raw mmio address '0x1000'; declare a matching mmio_reg or enable raw mmio access"
                    .to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_undeclared_mmio_bases() {
        let ast =
            parse_module("fn entry() { mmio_read<u32>(UART0); mmio_write<u8>(UART0 + 4, 0xff); }")
                .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "undeclared mmio base 'UART0' used in mmio_read<u32>(UART0)".to_string(),
                "undeclared mmio base 'UART0' used in mmio_write<u8>(UART0 + 4, 0xff)".to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_allows_int_literal_mmio_address_without_declaration() {
        let ast = parse_module("fn entry() { mmio_read<u32>(0x1000); }").expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([{
                "op": "mmio_read",
                "ty": "u32",
                "addr": {"kind": "int_literal", "value": "0x1000"}
            }])
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_unmatched_raw_mmio_literal_when_mmio_structure_declared() {
        let ast = parse_module("mmio UART0 = 0x1000;\nfn entry() { mmio_write<u32>(0x1014, x); }")
            .expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec![
                "unresolved raw mmio address '0x1014'; declare a matching mmio_reg or enable raw mmio access"
                    .to_string()
            ]
        );
    }

    #[test]
    fn analysis_krir_lowering_allows_unmatched_raw_mmio_literal_with_mmioraw_module_cap() {
        let ast = parse_module(
            "@module_caps(MmioRaw);\nmmio UART0 = 0x1000;\nfn entry() { mmio_write<u32>(0x1014, x); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([{
                "op": "mmio_write",
                "ty": "u32",
                "addr": {"kind": "int_literal", "value": "0x1014"},
                "value": {"kind": "ident", "name": "x"}
            }])
        );
    }

    #[test]
    fn analysis_krir_lowering_rejects_raw_mmio_without_mmioraw_module_cap() {
        let ast = parse_module("fn entry() { raw_mmio_write<u32>(0x1014, x); }").expect("parse");
        let errs =
            lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect_err("should reject");
        assert_eq!(
            errs,
            vec!["raw_mmio_write<u32>(0x1014, x) requires @module_caps(MmioRaw)".to_string()]
        );
    }

    #[test]
    fn analysis_krir_lowering_allows_raw_mmio_literal_exact_match_without_access_width_checks() {
        let ast = parse_module(
            "@module_caps(MmioRaw);\nmmio UART0 = 0x1000;\nmmio_reg UART0.SR = 0x04 : u32 ro;\nmmio_reg UART0.CR = 0x08 : u16 rw;\nfn entry() { raw_mmio_write<u32>(0x1004, x); raw_mmio_write<u32>(0x1008, x); }",
        )
        .expect("parse");
        let lowered = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("lower");
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry function");
        assert_eq!(
            serde_json::to_value(&entry.ops).expect("serialize ops"),
            json!([
                {
                    "op": "raw_mmio_write",
                    "ty": "u32",
                    "addr": {"kind": "int_literal", "value": "0x1004"},
                    "value": {"kind": "ident", "name": "x"}
                },
                {
                    "op": "raw_mmio_write",
                    "ty": "u32",
                    "addr": {"kind": "int_literal", "value": "0x1008"},
                    "value": {"kind": "ident", "name": "x"}
                }
            ])
        );
    }

    #[test]
    fn canonical_executable_rejects_raw_mmio_deterministically() {
        let ast = parse_module("fn entry() { raw_mmio_read<u32>(0x1000); }").expect("parse");
        let errs = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("canonical executable should reject raw mmio");
        assert_eq!(
            errs,
            vec![
                "canonical-exec: function 'entry' contains unsupported raw_mmio_read<u32>(0x1000)"
                    .to_string()
            ]
        );
    }

    #[test]
    fn canonical_executable_rejects_extern_call_targets_deterministically() {
        let ast =
            parse_module("fn entry() { ext(); }\nextern @ctx(thread) @eff() @caps() fn ext();")
                .expect("parse");
        let lowered = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("extern call target must be accepted");
        assert_eq!(
            lowered.extern_declarations,
            vec![super::CanonicalExecutableExternDecl {
                name: "ext".to_string(),
            }]
        );
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry");
        assert_eq!(
            serde_json::to_value(&entry.body.ops).expect("serialize ops"),
            json!([{"op": "call", "callee": "ext"}])
        );
    }

    #[test]
    fn canonical_executable_rejects_undeclared_call_targets_deterministically() {
        let ast = parse_module("fn entry() { missing(); }").expect("parse");
        let errs = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("undeclared call target must be rejected");
        assert_eq!(
            errs,
            vec![
                "undefined symbol 'missing': add extern declaration with canonical facts (@ctx/@eff/@caps)"
            ]
        );
    }

    #[test]
    fn canonical_executable_extern_declarations_remain_separate_from_functions() {
        let ast =
            parse_module("extern @ctx(thread) @eff() @caps() fn ext();\nfn entry() { ext(); }")
                .expect("parse");
        let lowered = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical executable lowering");

        assert_eq!(
            lowered
                .functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            vec!["entry"]
        );
        assert_eq!(
            lowered
                .extern_declarations
                .iter()
                .map(|decl| decl.name.as_str())
                .collect::<Vec<_>>(),
            vec!["ext"]
        );
    }

    #[test]
    fn canonical_executable_semantics_are_distinct_from_analysis_krir() {
        let ast = parse_module("fn entry() { helper(); }\nfn helper() { }").expect("parse");
        let canonical = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical executable");
        let analysis = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable).expect("analysis");

        let canonical_entry = canonical
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("canonical entry");
        let analysis_entry = analysis
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("analysis entry");

        assert_eq!(
            canonical_entry.body.terminator,
            CanonicalExecutableTerminator::ReturnUnit
        );
        assert_eq!(
            serde_json::to_value(&canonical_entry.body.ops).expect("canonical ops"),
            json!([{"op": "call", "callee": "helper"}])
        );
        assert_eq!(
            serde_json::to_value(&analysis_entry.ops).expect("analysis ops"),
            json!([{"op": "call", "callee": "helper"}])
        );
        assert_ne!(
            serde_json::to_value(canonical_entry).expect("canonical"),
            serde_json::to_value(analysis_entry).expect("analysis")
        );
    }

    #[test]
    fn canonical_executable_lowers_simple_function_to_executable_krir() {
        let ast = parse_module("fn entry() { }").expect("parse");
        let canonical = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical");
        let lowered =
            lower_canonical_executable_to_krir(&canonical).expect("lower executable krir");

        assert_eq!(
            serde_json::to_value(&lowered).expect("serialize"),
            json!({
                "module_caps": [],
                "extern_declarations": [],
                "functions": [{
                    "name": "entry",
                    "is_extern": false,
                    "signature": {
                        "params": [],
                        "result": "unit"
                    },
                    "facts": {
                        "ctx_ok": ["boot", "thread"],
                        "eff_used": [],
                        "caps_req": [],
                        "attrs": {
                            "noyield": false,
                            "critical": false,
                            "leaf": false,
                            "hotpath": false,
                            "lock_budget": null
                        }
                    },
                    "entry_block": "entry",
                    "blocks": [{
                        "label": "entry",
                        "ops": [],
                        "terminator": {
                            "terminator": "return",
                            "value": {
                                "kind": "unit"
                            }
                        }
                    }]
                }],
                "call_edges": []
            })
        );
    }

    #[test]
    fn canonical_executable_lowers_direct_call_to_entry_block_ops() {
        let ast = parse_module("fn entry() { helper(); }\nfn helper() { }").expect("parse");
        let canonical = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical");
        let lowered =
            lower_canonical_executable_to_krir(&canonical).expect("lower executable krir");

        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry");
        assert_eq!(entry.entry_block, "entry");
        assert_eq!(
            serde_json::to_value(&entry.blocks[0].ops).expect("serialize ops"),
            json!([{"op": "call", "callee": "helper"}])
        );
        assert_eq!(
            lowered.call_edges,
            vec![krir::CallEdge {
                caller: "entry".to_string(),
                callee: "helper".to_string()
            }]
        );
    }

    #[test]
    fn canonical_executable_lowers_declared_extern_target_to_executable_krir() {
        let ast =
            parse_module("extern @ctx(thread) @eff() @caps() fn ext();\nfn entry() { ext(); }")
                .expect("parse");
        let canonical = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical");
        let lowered =
            lower_canonical_executable_to_krir(&canonical).expect("lower executable krir");

        assert_eq!(
            lowered
                .extern_declarations
                .iter()
                .map(|decl| decl.name.as_str())
                .collect::<Vec<_>>(),
            vec!["ext"]
        );
        let entry = lowered
            .functions
            .iter()
            .find(|function| function.name == "entry")
            .expect("entry");
        assert_eq!(
            serde_json::to_value(&entry.blocks[0].ops).expect("serialize ops"),
            json!([{"op": "call", "callee": "ext"}])
        );
        assert_eq!(
            lowered.call_edges,
            vec![krir::CallEdge {
                caller: "entry".to_string(),
                callee: "ext".to_string()
            }]
        );
    }

    #[test]
    fn canonical_executable_lowering_keeps_function_order_deterministic() {
        let ast = parse_module("fn zeta() { }\nfn alpha() { }").expect("parse");
        let canonical = lower_to_canonical_executable_with_surface(&ast, SurfaceProfile::Stable)
            .expect("canonical");
        let lowered =
            lower_canonical_executable_to_krir(&canonical).expect("lower executable krir");

        assert_eq!(
            lowered
                .functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
    }

    #[test]
    fn canonical_executable_lowering_rejects_unsupported_param_shape_deterministically() {
        let module = super::CanonicalExecutableModule {
            module_caps: vec![],
            extern_declarations: vec![],
            functions: vec![super::CanonicalExecutableFunction {
                name: "entry".to_string(),
                signature: super::CanonicalExecutableSignature {
                    params: vec![super::CanonicalExecutableValueType::Unit],
                    result: super::CanonicalExecutableValueType::Unit,
                },
                facts: super::CanonicalExecutableFacts {
                    ctx_ok: vec![krir::Ctx::Thread],
                    eff_used: vec![],
                    caps_req: vec![],
                    attrs: krir::FunctionAttrs::default(),
                },
                body: super::CanonicalExecutableBody {
                    ops: vec![],
                    terminator: CanonicalExecutableTerminator::ReturnUnit,
                },
            }],
        };

        assert_eq!(
            lower_canonical_executable_to_krir(&module),
            Err(vec![
                "canonical-exec->krir: canonical executable function 'entry' must not declare parameters in v0.1"
                    .to_string()
            ])
        );
    }

    #[test]
    fn additional_adaptive_aliases_are_rejected_in_stable_surface() {
        let cases = [(
            "@may_block fn worker() { }",
            "surface feature '@may_block' is a compatibility alias and requires --surface experimental for 'worker' at 1:1\n  1 | @may_block fn worker() { }\n  = help: did you mean the canonical spelling @eff(block)? this compatibility alias is kept only for migration guidance.",
        )];

        for (src, expected) in cases {
            let ast = parse_module(src).expect("parse");
            let errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
                .expect_err("stable surface must reject alias");
            assert_eq!(
                errs,
                vec![expected],
                "stable rejection drifted for '{}'",
                src
            );
        }
    }

    #[test]
    fn deprecated_adaptive_alias_is_rejected_in_all_profiles() {
        let ast = parse_module("@irq_legacy fn legacy_isr() { }").expect("parse");
        let stable_errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Stable)
            .expect_err("stable must reject deprecated alias");
        let experimental_errs = lower_to_krir_with_surface(&ast, SurfaceProfile::Experimental)
            .expect_err("experimental must reject deprecated alias");

        assert_eq!(
            stable_errs,
            vec![
                "surface feature '@irq_legacy' is a deprecated alias and is unavailable under --surface stable for 'legacy_isr' at 1:1\n  1 | @irq_legacy fn legacy_isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)? this deprecated alias is kept only for migration guidance."
            ]
        );
        assert_eq!(
            experimental_errs,
            vec![
                "surface feature '@irq_legacy' is a deprecated alias and is unavailable under --surface experimental for 'legacy_isr' at 1:1\n  1 | @irq_legacy fn legacy_isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)? this deprecated alias is kept only for migration guidance."
            ]
        );
    }

    #[test]
    fn legacy_noncanonical_yieldpoint_attribute_reports_canonical_replacement() {
        let ast = parse_module("@yieldpoint fn pump() { }").expect("parse");
        let errs = lower_to_krir(&ast).expect_err("legacy attribute must reject");
        assert_eq!(
            errs,
            vec![
                "legacy spelling '@yieldpoint' is non-canonical and is not accepted on function 'pump' at 1:1\n  1 | @yieldpoint fn pump() { }\n  = help: did you mean the canonical spelling yieldpoint()? control-point markers use statement form, not attributes."
            ]
        );
    }

    #[test]
    fn frontend_migration_feature_registry_includes_accepted_legacy_unary_shorthands() {
        let stable = frontend_migration_features_for_profile(SurfaceProfile::Stable);
        let experimental = frontend_migration_features_for_profile(SurfaceProfile::Experimental);

        for id in [
            "legacy_alloc_effect_shorthand",
            "legacy_block_effect_shorthand",
            "legacy_irq_context_shorthand",
            "legacy_noirq_context_shorthand",
            "legacy_preempt_off_effect_shorthand",
        ] {
            assert!(
                stable.iter().any(|feature| feature.id == id),
                "stable migration features must include '{}'",
                id
            );
            assert!(
                experimental.iter().any(|feature| feature.id == id),
                "experimental migration features must include '{}'",
                id
            );
        }
    }

    #[test]
    fn frontend_migration_preview_surfaces_legacy_unary_shorthands_deterministically() {
        let ast = parse_module(
            "@alloc fn alloc_worker() { }\n@block fn block_worker() { }\n@irq fn irq_entry() { }\n@noirq fn noirq_worker() { }\n@preempt_off fn preempt_guarded() { }",
        )
        .expect("parse");
        let entries = frontend_migration_preview(&ast, SurfaceProfile::Stable);
        assert_eq!(
            entries
                .iter()
                .map(|entry| {
                    (
                        entry.function_name.as_str(),
                        entry.feature.id,
                        entry.feature.surface_form,
                        entry.feature.canonical_replacement,
                        entry.enabled_under_surface,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "alloc_worker",
                    "legacy_alloc_effect_shorthand",
                    "alloc",
                    "@eff(alloc)",
                    true,
                ),
                (
                    "block_worker",
                    "legacy_block_effect_shorthand",
                    "block",
                    "@eff(block)",
                    true,
                ),
                (
                    "irq_entry",
                    "legacy_irq_context_shorthand",
                    "irq",
                    "@ctx(irq)",
                    true,
                ),
                (
                    "noirq_worker",
                    "legacy_noirq_context_shorthand",
                    "noirq",
                    "@ctx(thread, boot)",
                    true,
                ),
                (
                    "preempt_guarded",
                    "legacy_preempt_off_effect_shorthand",
                    "preempt_off",
                    "@eff(preempt_off)",
                    true,
                ),
            ]
        );
    }

    #[test]
    fn frontend_canonical_findings_include_accepted_legacy_unary_shorthands() {
        let ast = parse_module(
            "@alloc fn alloc_worker() { }\n@block fn block_worker() { }\n@irq fn irq_entry() { }\n@noirq fn noirq_worker() { }\n@preempt_off fn preempt_guarded() { }",
        )
        .expect("parse");
        let findings = frontend_canonical_findings(&ast, SurfaceProfile::Stable);
        assert_eq!(
            findings
                .iter()
                .map(|finding| (
                    finding.function_name.as_str(),
                    finding.classification.as_str(),
                    finding.surface_form,
                    finding.canonical_replacement,
                    finding.migration_safe,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "alloc_worker",
                    "compatibility_alias",
                    "alloc",
                    "@eff(alloc)",
                    true,
                ),
                (
                    "block_worker",
                    "compatibility_alias",
                    "block",
                    "@eff(block)",
                    true,
                ),
                ("irq_entry", "compatibility_alias", "irq", "@ctx(irq)", true,),
                (
                    "noirq_worker",
                    "compatibility_alias",
                    "noirq",
                    "@ctx(thread, boot)",
                    true,
                ),
                (
                    "preempt_guarded",
                    "compatibility_alias",
                    "preempt_off",
                    "@eff(preempt_off)",
                    true,
                ),
            ]
        );
    }

    #[test]
    fn frontend_canonical_findings_ignore_disabled_aliases_under_stable_surface() {
        let ast = parse_module("@irq_handler fn isr() { }\n@thread_entry fn worker() { }")
            .expect("parse");
        let findings = frontend_canonical_findings(&ast, SurfaceProfile::Stable);
        assert_eq!(
            findings
                .iter()
                .map(|finding| {
                    (
                        finding.function_name.as_str(),
                        finding.surface_form,
                        finding.canonical_replacement,
                    )
                })
                .collect::<Vec<_>>(),
            vec![("worker", "thread_entry", "@ctx(thread)")]
        );
    }

    #[test]
    fn frontend_canonical_edit_plan_includes_safe_rewrite_metadata() {
        let ast = parse_module(
            "@alloc fn alloc_worker() { }\n@block fn block_worker() { }\n@irq fn irq_entry() { }\n@noirq fn noirq_worker() { }\n@preempt_off fn preempt_guarded() { }",
        )
        .expect("parse");
        let edits = frontend_canonical_edit_plan(&ast, SurfaceProfile::Stable);
        assert_eq!(
            edits
                .iter()
                .map(|edit| (
                    edit.function_name.as_str(),
                    edit.classification.as_str(),
                    edit.surface_form,
                    edit.canonical_replacement,
                    edit.migration_safe,
                    edit.rewrite_intent,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "alloc_worker",
                    "compatibility_alias",
                    "alloc",
                    "@eff(alloc)",
                    true,
                    "Replace the attribute token `@alloc` with `@eff(alloc)`.",
                ),
                (
                    "block_worker",
                    "compatibility_alias",
                    "block",
                    "@eff(block)",
                    true,
                    "Replace the attribute token `@block` with `@eff(block)`.",
                ),
                (
                    "irq_entry",
                    "compatibility_alias",
                    "irq",
                    "@ctx(irq)",
                    true,
                    "Replace the attribute token `@irq` with `@ctx(irq)`.",
                ),
                (
                    "noirq_worker",
                    "compatibility_alias",
                    "noirq",
                    "@ctx(thread, boot)",
                    true,
                    "Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
                ),
                (
                    "preempt_guarded",
                    "compatibility_alias",
                    "preempt_off",
                    "@eff(preempt_off)",
                    true,
                    "Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
                ),
            ]
        );
    }

    #[test]
    fn frontend_canonical_edit_plan_ignores_disabled_aliases_under_stable_surface() {
        let ast = parse_module("@irq_handler fn isr() { }\n@thread_entry fn worker() { }")
            .expect("parse");
        let edits = frontend_canonical_edit_plan(&ast, SurfaceProfile::Stable);
        assert_eq!(
            edits
                .iter()
                .map(|edit| {
                    (
                        edit.function_name.as_str(),
                        edit.surface_form,
                        edit.canonical_replacement,
                        edit.rewrite_intent,
                    )
                })
                .collect::<Vec<_>>(),
            vec![(
                "worker",
                "thread_entry",
                "@ctx(thread)",
                "Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
            )]
        );
    }

    #[test]
    fn adaptive_feature_registry_and_proposal_are_deterministic() {
        assert_eq!(
            serde_json::to_value(adaptive_surface_features()).expect("registry json"),
            json!([
                {
                    "id": "irq_handler_alias",
                    "proposal_id": "irq_handler_alias",
                    "surface_form": "irq_handler",
                    "status": "experimental",
                    "lowering_target": "@ctx(irq)",
                    "safety_notes": "Pure surface alias; lowers to the existing irq context declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(irq) when pinning code back to stable.",
                    "canonical_replacement": "@ctx(irq)",
                    "migration_safe": true,
                    "rewrite_intent": "Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
                    "surface_profile_gate": "experimental"
                },
                {
                    "id": "thread_entry_alias",
                    "proposal_id": "thread_entry_alias",
                    "surface_form": "thread_entry",
                    "status": "stable",
                    "lowering_target": "@ctx(thread)",
                    "safety_notes": "Pure surface alias; lowers to the existing thread context declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(thread) when pinning code back to stable.",
                    "canonical_replacement": "@ctx(thread)",
                    "migration_safe": true,
                    "rewrite_intent": "Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
                    "surface_profile_gate": "stable"
                },
                {
                    "id": "may_block_alias",
                    "proposal_id": "may_block_alias",
                    "surface_form": "may_block",
                    "status": "experimental",
                    "lowering_target": "@eff(block)",
                    "safety_notes": "Pure surface alias; lowers to the existing block effect declaration.",
                    "migration_supported": true,
                    "migration_note": "Replace with @eff(block) when pinning code back to stable.",
                    "canonical_replacement": "@eff(block)",
                    "migration_safe": true,
                    "rewrite_intent": "Replace the attribute token `@may_block` with `@eff(block)`.",
                    "surface_profile_gate": "experimental"
                },
                {
                    "id": "irq_legacy_alias",
                    "proposal_id": "irq_legacy_alias",
                    "surface_form": "irq_legacy",
                    "status": "deprecated",
                    "lowering_target": "@ctx(irq)",
                    "safety_notes": "Deprecated surface alias kept only to prove centralized lifecycle gating.",
                    "migration_supported": true,
                    "migration_note": "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
                    "canonical_replacement": "@ctx(irq)",
                    "migration_safe": true,
                    "rewrite_intent": "Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
                    "surface_profile_gate": "stable"
                }
            ])
        );

        assert_eq!(
            serde_json::to_value(adaptive_feature_proposals()).expect("proposal json"),
            json!([
                {
                    "id": "irq_handler_alias",
                    "title": "Experimental @irq_handler surface alias",
                    "motivation": "Provide a governed surface-only shorthand for irq-context entry points.",
                    "syntax_before": "@ctx(irq) fn isr() { }",
                    "syntax_after": "@irq_handler fn isr() { }",
                    "lowering_description": "Lower @irq_handler to the existing canonical @ctx(irq) representation during HIR lowering.",
                    "compatibility_risk": "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
                    "migration_plan": "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
                    "status": "experimental"
                },
                {
                    "id": "thread_entry_alias",
                    "title": "Stable @thread_entry surface alias",
                    "motivation": "Provide a governed surface-only shorthand for thread-only entry points.",
                    "syntax_before": "@ctx(thread) fn worker() { }",
                    "syntax_after": "@thread_entry fn worker() { }",
                    "lowering_description": "Lower @thread_entry to the existing canonical @ctx(thread) representation during HIR lowering.",
                    "compatibility_risk": "Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
                    "migration_plan": "No migration required; the alias is now stable and remains interchangeable with @ctx(thread).",
                    "status": "stable"
                },
                {
                    "id": "may_block_alias",
                    "title": "Experimental @may_block surface alias",
                    "motivation": "Provide a governed surface-only shorthand for declaring block effects.",
                    "syntax_before": "@eff(block) fn worker() { }",
                    "syntax_after": "@may_block fn worker() { }",
                    "lowering_description": "Lower @may_block to the existing canonical @eff(block) representation during HIR lowering.",
                    "compatibility_risk": "Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
                    "migration_plan": "Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
                    "status": "experimental"
                },
                {
                    "id": "irq_legacy_alias",
                    "title": "Deprecated @irq_legacy surface alias",
                    "motivation": "Preserve a historical alias only long enough to exercise deterministic lifecycle gating.",
                    "syntax_before": "@ctx(irq) fn legacy_isr() { }",
                    "syntax_after": "@irq_legacy fn legacy_isr() { }",
                    "lowering_description": "Would lower to the existing canonical @ctx(irq) representation if lifecycle policy allowed it.",
                    "compatibility_risk": "Medium; the alias is deprecated and intentionally unavailable under current surface profiles.",
                    "migration_plan": "Replace with @ctx(irq) or @irq_handler depending on the chosen profile policy.",
                    "status": "deprecated"
                }
            ])
        );
    }

    #[test]
    fn every_adaptive_feature_links_to_one_canonical_proposal() {
        for feature in adaptive_surface_features() {
            let proposal = adaptive_feature_proposal(feature.id)
                .unwrap_or_else(|| panic!("missing proposal for feature '{}'", feature.id));
            assert_eq!(proposal.id, feature.proposal_id);
        }
    }

    #[test]
    fn surface_profiles_resolve_feature_sets_centrally() {
        let features = adaptive_surface_features();
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Stable))
                .all(|feature| surface_profile_enables_feature(SurfaceProfile::Stable, feature)),
            "stable must enable stable features"
        );
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Experimental))
                .all(|feature| !surface_profile_enables_feature(SurfaceProfile::Stable, feature)),
            "stable must not enable experimental-only features"
        );
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Experimental))
                .all(|feature| surface_profile_enables_feature(
                    SurfaceProfile::Experimental,
                    feature
                )),
            "experimental must enable experimental features"
        );
        assert!(
            features
                .iter()
                .filter(|feature| matches!(feature.status, AdaptiveFeatureStatus::Deprecated))
                .all(|feature| !surface_profile_enables_feature(
                    SurfaceProfile::Experimental,
                    feature
                )),
            "experimental must not enable deprecated features by default"
        );

        let stable_feature = super::AdaptiveSurfaceFeature {
            id: "stable_alias",
            proposal_id: "stable_alias",
            surface_form: "stable_alias",
            status: AdaptiveFeatureStatus::Stable,
            lowering_target: "@ctx(thread)",
            safety_notes: "test-only stable feature",
            migration_supported: true,
            migration_note: "none",
            canonical_replacement: "@ctx(thread)",
            migration_safe: true,
            rewrite_intent: "Replace the attribute token `@stable_alias` with `@ctx(thread)`.",
            surface_profile_gate: SurfaceProfile::Stable,
            lowering_rule: super::AdaptiveLoweringRule::ContextAlias(&[krir::Ctx::Thread]),
        };

        assert!(surface_profile_enables_feature(
            SurfaceProfile::Stable,
            &stable_feature
        ));
        assert!(surface_profile_enables_feature(
            SurfaceProfile::Experimental,
            &stable_feature
        ));
    }

    #[test]
    fn proposal_example_file_matches_serialized_proposal() {
        let cases = [
            (
                "irq_handler_alias",
                include_str!("../../../docs/design/examples/irq_handler_alias.proposal.json"),
            ),
            (
                "thread_entry_alias",
                include_str!("../../../docs/design/examples/thread_entry_alias.proposal.json"),
            ),
            (
                "may_block_alias",
                include_str!("../../../docs/design/examples/may_block_alias.proposal.json"),
            ),
            (
                "irq_legacy_alias",
                include_str!("../../../docs/design/examples/irq_legacy_alias.proposal.json"),
            ),
        ];

        for (feature_id, expected) in cases {
            let actual = serde_json::to_string_pretty(
                adaptive_feature_proposal(feature_id).expect("proposal"),
            )
            .expect("proposal");
            assert_eq!(
                actual.trim_end(),
                expected.trim_end(),
                "proposal file drifted"
            );
        }
    }

    #[test]
    fn adaptive_feature_governance_validation_is_deterministic() {
        assert!(validate_adaptive_feature_governance().is_empty());

        let bad_features = [AdaptiveSurfaceFeature {
            id: "broken_alias",
            proposal_id: "missing_alias_proposal",
            surface_form: "broken",
            status: AdaptiveFeatureStatus::Stable,
            lowering_target: "@ctx(thread)",
            safety_notes: "test-only",
            migration_supported: true,
            migration_note: "test-only",
            canonical_replacement: "@ctx(thread)",
            migration_safe: true,
            rewrite_intent: "Replace the attribute token `@broken` with `@ctx(thread)`.",
            surface_profile_gate: SurfaceProfile::Stable,
            lowering_rule: AdaptiveLoweringRule::ContextAlias(&[super::Ctx::Thread]),
        }];
        let bad_proposals = [AdaptiveFeatureProposal {
            id: "shared_proposal",
            title: "Broken proposal",
            motivation: "test-only",
            syntax_before: "@ctx(thread) fn worker() { }",
            syntax_after: "@other_alias fn worker() { }",
            lowering_description: "Lower to a mismatched target.",
            compatibility_risk: "test-only",
            migration_plan: "test-only",
            status: AdaptiveFeatureStatus::Experimental,
        }];

        assert_eq!(
            validate_adaptive_feature_governance_with(&bad_features, &bad_proposals),
            vec![
                "proposal-validation: feature 'broken_alias' references missing proposal 'missing_alias_proposal'",
                "proposal-validation: proposal 'shared_proposal' is unreferenced",
            ]
        );
    }

    #[test]
    fn adaptive_feature_promotion_readiness_failure_reasons_are_deterministic() {
        let bad_features = [AdaptiveSurfaceFeature {
            id: "broken_alias",
            proposal_id: "broken_alias",
            surface_form: "broken",
            status: AdaptiveFeatureStatus::Experimental,
            lowering_target: "@ctx(thread)",
            safety_notes: "test-only",
            migration_supported: true,
            migration_note: "test-only",
            canonical_replacement: "@ctx(thread)",
            migration_safe: false,
            rewrite_intent: "Replace the attribute token `@broken` with `@ctx(thread)`.",
            surface_profile_gate: SurfaceProfile::Experimental,
            lowering_rule: AdaptiveLoweringRule::ContextAlias(&[super::Ctx::Thread]),
        }];
        let bad_proposals = [AdaptiveFeatureProposal {
            id: "broken_alias",
            title: "Broken proposal",
            motivation: "test-only",
            syntax_before: "@ctx(thread) fn worker() { }",
            syntax_after: "@broken fn worker() { }",
            lowering_description: "Lower @broken to the existing canonical @ctx(thread) representation during HIR lowering.",
            compatibility_risk: "test-only",
            migration_plan: "test-only",
            status: AdaptiveFeatureStatus::Experimental,
        }];

        let readiness = adaptive_feature_promotion_readiness_with(&bad_features, &bad_proposals);
        assert_eq!(readiness.len(), 1);
        assert!(!readiness[0].promotable_to_stable);
        assert_eq!(readiness[0].reason, "migration metadata incomplete");
    }

    #[test]
    fn adaptive_feature_promotion_plan_normalizes_stable_proposal_text() {
        let plan = adaptive_feature_promotion_plan_with(
            adaptive_surface_features(),
            adaptive_feature_proposals(),
            "irq_handler_alias",
        )
        .expect("promotion plan");
        assert_eq!(plan.feature_id, "irq_handler_alias");
        assert_eq!(plan.proposal_id, "irq_handler_alias");
        assert_eq!(
            plan.normalized_proposal_title,
            "Stable @irq_handler surface alias"
        );
        assert_eq!(
            plan.normalized_compatibility_risk,
            "Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles."
        );
        assert_eq!(
            plan.normalized_migration_plan,
            "No migration required; the alias is now stable and remains interchangeable with @ctx(irq)."
        );
    }
}
