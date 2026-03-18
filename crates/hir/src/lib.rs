use std::collections::{BTreeMap, BTreeSet};

use krir::{
    CallEdge, Ctx, Eff, ExecutableBlock, ExecutableExternDecl, ExecutableFacts, ExecutableFunction,
    ExecutableKrirModule, ExecutableOp as KrExecutableOp, ExecutableSignature,
    ExecutableTerminator, ExecutableValue, ExecutableValueType, Function, FunctionAttrs,
    KrirModule, KrirOp, MmioAddrExpr as KrirMmioAddrExpr, MmioBaseDecl as KrirMmioBaseDecl,
    MmioRegAccess as KrirMmioRegAccess, MmioRegisterDecl as KrirMmioRegisterDecl,
    MmioScalarType as KrirMmioScalarType, MmioValueExpr as KrirMmioValueExpr,
};
use parser::{
    FnAst, MmioAddrExpr as ParserMmioAddrExpr, MmioBaseDecl as ParserMmioBaseDecl,
    MmioRegAccess as ParserMmioRegAccess, MmioRegisterDecl as ParserMmioRegisterDecl,
    MmioScalarType as ParserMmioScalarType, MmioValueExpr as ParserMmioValueExpr, ModuleAst,
    RawAttr, Stmt, format_source_diagnostic, int_literal_numeric_value, split_csv,
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
            "surface feature '@{}' requires --surface experimental for '{}'",
            feature.surface_form, function_name
        ),
        AdaptiveFeatureStatus::Deprecated => format!(
            "surface feature '@{}' is deprecated and unavailable under --surface {} for '{}'",
            feature.surface_form,
            match surface_profile {
                SurfaceProfile::Stable => "stable",
                SurfaceProfile::Experimental => "experimental",
            },
            function_name
        ),
        AdaptiveFeatureStatus::Stable => format!(
            "surface feature '@{}' is unavailable under --surface {} for '{}'",
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

fn canonical_spelling_help(spelling: &str) -> String {
    format!("did you mean the canonical spelling {}?", spelling)
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

    for item in &ast.items {
        validate_mmio_address_bases_in_stmts(
            &item.body,
            &mmio_base_names,
            &mmio_register_rules,
            &mmio_absolute_register_rules,
            module_declares_mmio_structure,
            module_allows_raw_mmio_literals,
            &mut errors,
        );
    }

    for item in &ast.items {
        match lower_function(item, surface_profile) {
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
            if let KrirOp::Call { callee } = op {
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
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut module = KrirModule {
        module_caps: ast.module_caps.clone(),
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

fn lower_function(item: &FnAst, surface_profile: SurfaceProfile) -> Result<Function, Vec<String>> {
    let facts = normalize_function_facts(item, surface_profile)?;

    let mut ops = Vec::new();
    let mut eff_used = facts.eff_used;
    for stmt in &item.body {
        lower_stmt(stmt, &mut ops, &mut eff_used);
    }

    Ok(Function {
        name: item.name.clone(),
        is_extern: item.is_extern,
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

fn validate_mmio_address_bases_in_stmts(
    stmts: &[Stmt],
    declared_bases: &BTreeSet<String>,
    declared_registers: &MmioRegisterRules,
    declared_absolute_registers: &MmioAbsoluteRegisterRules,
    module_declares_mmio_structure: bool,
    module_allows_raw_mmio_literals: bool,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Critical(inner) => validate_mmio_address_bases_in_stmts(
                inner,
                declared_bases,
                declared_registers,
                declared_absolute_registers,
                module_declares_mmio_structure,
                module_allows_raw_mmio_literals,
                errors,
            ),
            Stmt::MmioRead { ty, addr } => {
                if let Some(base) = mmio_addr_base_name(addr)
                    && !declared_bases.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_mmio_read_invocation(*ty, addr)
                    ));
                } else {
                    match addr {
                        ParserMmioAddrExpr::Ident(base) => validate_mmio_register_read(
                            base,
                            MMIO_SYMBOLIC_BASE_ZERO_OFFSET,
                            *ty,
                            addr,
                            declared_registers,
                            errors,
                        ),
                        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => {
                            validate_mmio_register_read(
                                base,
                                offset,
                                *ty,
                                addr,
                                declared_registers,
                                errors,
                            );
                        }
                        ParserMmioAddrExpr::IntLiteral(literal) => {
                            let matched = validate_mmio_register_read_absolute(
                                literal,
                                *ty,
                                addr,
                                declared_absolute_registers,
                                errors,
                            );
                            if !matched
                                && module_declares_mmio_structure
                                && !module_allows_raw_mmio_literals
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
            Stmt::RawMmioRead { ty, addr } => {
                if !module_allows_raw_mmio_literals {
                    errors.push(format!(
                        "{} requires @module_caps({})",
                        format_raw_mmio_read_invocation(*ty, addr),
                        MMIO_RAW_MODULE_CAP
                    ));
                    continue;
                }
                if let Some(base) = mmio_addr_base_name(addr)
                    && !declared_bases.contains(base)
                {
                    errors.push(format!(
                        "undeclared mmio base '{}' used in {}",
                        base,
                        format_raw_mmio_read_invocation(*ty, addr)
                    ));
                }
            }
            Stmt::MmioWrite { ty, addr, value } => {
                if let Some(base) = mmio_addr_base_name(addr)
                    && !declared_bases.contains(base)
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
                            declared_registers,
                            errors,
                        ),
                        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => {
                            validate_mmio_register_write(
                                base,
                                offset,
                                *ty,
                                addr,
                                value,
                                declared_registers,
                                errors,
                            );
                        }
                        ParserMmioAddrExpr::IntLiteral(literal) => {
                            let matched = validate_mmio_register_write_absolute(
                                literal,
                                *ty,
                                addr,
                                value,
                                declared_absolute_registers,
                                errors,
                            );
                            if !matched
                                && module_declares_mmio_structure
                                && !module_allows_raw_mmio_literals
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
                if !module_allows_raw_mmio_literals {
                    errors.push(format!(
                        "{} requires @module_caps({})",
                        format_raw_mmio_write_invocation(*ty, addr, value),
                        MMIO_RAW_MODULE_CAP
                    ));
                    continue;
                }
                if let Some(base) = mmio_addr_base_name(addr)
                    && !declared_bases.contains(base)
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
    declared_registers: &MmioRegisterRules,
    errors: &mut Vec<String>,
) {
    let invocation = format_mmio_read_invocation(ty, addr);
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
    declared_absolute_registers: &MmioAbsoluteRegisterRules,
    errors: &mut Vec<String>,
) -> bool {
    let Some(addr_numeric) = int_literal_numeric_value(literal) else {
        return false;
    };
    let Some(rule) = declared_absolute_registers.get(&addr_numeric) else {
        return false;
    };
    let invocation = format_mmio_read_invocation(ty, addr);
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
                    caps_req.extend(split_csv(raw));
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
            "module_caps" => {}
            other => {
                if let Some(feature) = adaptive_surface_feature(other) {
                    if !surface_profile_enables_feature(surface_profile, feature) {
                        errors.push(format_attr_diagnostic(
                            item,
                            attr,
                            &feature_unavailability_error(surface_profile, feature, &item.name),
                            Some(&canonical_spelling_help(feature.canonical_replacement)),
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
                    errors.push(format_attr_diagnostic(
                        item,
                        attr,
                        &format!("unknown attribute '@{}' on function '{}'", other, item.name),
                        None,
                    ));
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
            Stmt::Critical(_) => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported critical region",
                function_name
            )),
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
            Stmt::MmioRead { ty, addr } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_mmio_read_invocation(*ty, addr)
            )),
            Stmt::MmioWrite { ty, addr, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_mmio_write_invocation(*ty, addr, value)
            )),
            Stmt::RawMmioRead { ty, addr } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_raw_mmio_read_invocation(*ty, addr)
            )),
            Stmt::RawMmioWrite { ty, addr, value } => errors.push(format!(
                "canonical-exec: function '{}' contains unsupported {}",
                function_name,
                format_raw_mmio_write_invocation(*ty, addr, value)
            )),
        }
    }
}

fn lower_stmt(stmt: &Stmt, ops: &mut Vec<KrirOp>, eff_used: &mut BTreeSet<Eff>) {
    match stmt {
        Stmt::Call(callee) => ops.push(KrirOp::Call {
            callee: callee.clone(),
        }),
        Stmt::Critical(inner) => {
            ops.push(KrirOp::CriticalEnter);
            for stmt in inner {
                lower_stmt(stmt, ops, eff_used);
            }
            ops.push(KrirOp::CriticalExit);
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
        Stmt::MmioRead { ty, addr } => {
            ops.push(KrirOp::MmioRead {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::MmioWrite { ty, addr, value } => {
            ops.push(KrirOp::MmioWrite {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr),
                value: lower_mmio_value_expr(value),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::RawMmioRead { ty, addr } => {
            ops.push(KrirOp::RawMmioRead {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr),
            });
            eff_used.insert(Eff::Mmio);
        }
        Stmt::RawMmioWrite { ty, addr, value } => {
            ops.push(KrirOp::RawMmioWrite {
                ty: lower_mmio_scalar_type(*ty),
                addr: lower_mmio_addr_expr(addr),
                value: lower_mmio_value_expr(value),
            });
            eff_used.insert(Eff::Mmio);
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

fn lower_mmio_reg_access(access: ParserMmioRegAccess) -> KrirMmioRegAccess {
    match access {
        ParserMmioRegAccess::Ro => KrirMmioRegAccess::Ro,
        ParserMmioRegAccess::Wo => KrirMmioRegAccess::Wo,
        ParserMmioRegAccess::Rw => KrirMmioRegAccess::Rw,
    }
}

fn lower_mmio_addr_expr(expr: &ParserMmioAddrExpr) -> KrirMmioAddrExpr {
    match expr {
        ParserMmioAddrExpr::Ident(name) => KrirMmioAddrExpr::Ident { name: name.clone() },
        ParserMmioAddrExpr::IntLiteral(value) => KrirMmioAddrExpr::IntLiteral {
            value: value.clone(),
        },
        ParserMmioAddrExpr::IdentPlusOffset { base, offset } => KrirMmioAddrExpr::IdentPlusOffset {
            base: base.clone(),
            offset: offset.clone(),
        },
    }
}

fn lower_mmio_value_expr(expr: &ParserMmioValueExpr) -> KrirMmioValueExpr {
    match expr {
        ParserMmioValueExpr::Ident(name) => KrirMmioValueExpr::Ident { name: name.clone() },
        ParserMmioValueExpr::IntLiteral(value) => KrirMmioValueExpr::IntLiteral {
            value: value.clone(),
        },
    }
}

fn format_mmio_read_invocation(ty: ParserMmioScalarType, addr: &ParserMmioAddrExpr) -> String {
    format!("mmio_read<{}>({})", ty.as_str(), addr.as_source())
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

fn format_raw_mmio_read_invocation(ty: ParserMmioScalarType, addr: &ParserMmioAddrExpr) -> String {
    format!("raw_mmio_read<{}>({})", ty.as_str(), addr.as_source())
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

fn parse_ctx_attr(attr: &RawAttr) -> Result<Vec<Ctx>, String> {
    let Some(args) = attr.args.as_deref() else {
        return Err("@ctx(...) requires a context list".to_string());
    };

    let mut out = Vec::new();
    for token in split_csv(args) {
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
    for token in split_csv(args) {
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
                "surface feature '@irq_handler' requires --surface experimental for 'isr' at 1:1\n  1 | @irq_handler fn isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)?"
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
            "surface feature '@may_block' requires --surface experimental for 'worker' at 1:1\n  1 | @may_block fn worker() { }\n  = help: did you mean the canonical spelling @eff(block)?",
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
                "surface feature '@irq_legacy' is deprecated and unavailable under --surface stable for 'legacy_isr' at 1:1\n  1 | @irq_legacy fn legacy_isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)?"
            ]
        );
        assert_eq!(
            experimental_errs,
            vec![
                "surface feature '@irq_legacy' is deprecated and unavailable under --surface experimental for 'legacy_isr' at 1:1\n  1 | @irq_legacy fn legacy_isr() { }\n  = help: did you mean the canonical spelling @ctx(irq)?"
            ]
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
