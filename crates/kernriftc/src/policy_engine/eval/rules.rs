use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    CONTRACTS_SCHEMA_VERSION_V2, ContractsBundle, ContractsFactSymbol, ContractsNoYieldSpan,
    ContractsProvenance, PolicyConditionDescriptor, PolicyFamily, PolicyFile, PolicyRule,
    PolicyViolation, policy_family_specs, policy_rule_conditions, policy_rule_effect_condition,
    policy_rule_forbid_raw_mmio_in_irq, policy_rule_forbidden_lock_edges,
    policy_rule_irq_capability_lists, policy_rule_irq_raw_mmio_site_limit, policy_rule_is_enabled,
    policy_rule_max_lock_depth, policy_rule_max_no_yield_span, policy_rule_module_cap_allowlist,
    policy_rule_raw_mmio_allow_global, policy_rule_raw_mmio_site_limit,
    policy_rule_raw_mmio_symbol_allowlist, policy_rule_spec,
};
use super::violation::{
    bind_capability_rule_violation, bind_context_rule_violation, bind_effect_rule_violation,
    bind_lock_rule_violation, bind_region_rule_violation,
};

pub(super) struct PolicyEvalView<'a> {
    symbol_by_name: BTreeMap<&'a str, &'a ContractsFactSymbol>,
    irq_symbol_names: Vec<&'a str>,
    raw_mmio_symbol_names: Vec<&'a str>,
}

impl<'a> PolicyEvalView<'a> {
    pub(super) fn build(contracts: &'a ContractsBundle) -> Self {
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

        let mut raw_mmio_symbol_names = contracts
            .facts
            .symbols
            .iter()
            .filter(|symbol| symbol.has_raw_mmio_usage())
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();
        raw_mmio_symbol_names.sort();
        raw_mmio_symbol_names.dedup();

        Self {
            symbol_by_name,
            irq_symbol_names,
            raw_mmio_symbol_names,
        }
    }
}

pub(super) fn evaluate_context_rules(
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

pub(super) fn evaluate_lock_rules(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
) -> Vec<PolicyViolation> {
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

pub(super) fn evaluate_effect_rules(
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
                PolicyConditionDescriptor::RawMmioObserved if kernel_v2_allowed => {
                    for observation in
                        policy_rule_raw_mmio_forbid_violations(policy, view, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::RawMmioForbidden(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::RawMmioSitesCountAboveConfiguredLimit
                    if kernel_v2_allowed =>
                {
                    if let Some(observation) =
                        policy_rule_raw_mmio_site_limit_violation(policy, contracts, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::RawMmioSiteLimit(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::RawMmioSymbolNotAllowed if kernel_v2_allowed => {
                    for observation in
                        policy_rule_raw_mmio_symbol_violations(policy, view, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::RawMmioSymbol(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::IrqRawMmioObserved if kernel_v2_allowed => {
                    for observation in
                        policy_rule_irq_raw_mmio_forbid_violations(policy, view, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::IrqRawMmioForbidden(observation),
                        ));
                    }
                }
                PolicyConditionDescriptor::IrqRawMmioSitesCountAboveConfiguredLimit
                    if kernel_v2_allowed =>
                {
                    if let Some(observation) =
                        policy_rule_irq_raw_mmio_site_limit_violation(policy, view, spec.rule)
                    {
                        violations.push(bind_effect_rule_violation(
                            spec.rule,
                            EffectRuleObservation::IrqRawMmioSiteLimit(observation),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    violations
}

pub(super) fn evaluate_region_rules(
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

pub(super) fn evaluate_capability_rules(
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

pub(super) enum NoYieldLimitObservation<'a> {
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

pub(super) enum LockRuleObservation<'a> {
    Depth(LockDepthObservation),
    ForbiddenEdge(ForbiddenLockEdgeObservation<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SchemaMismatchObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LockDepthObservation {
    pub(super) observed: u64,
    pub(super) limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ForbiddenLockEdgeObservation<'a> {
    pub(super) from: &'a str,
    pub(super) to: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NoYieldUnboundedObservation<'a> {
    pub(super) symbol: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IrqEffectObservation<'a> {
    pub(super) symbol_name: &'a str,
    pub(super) provenance: Option<&'a ContractsProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawMmioForbiddenObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IrqRawMmioForbiddenObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawMmioSiteLimitObservation {
    pub(super) observed: u64,
    pub(super) limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IrqRawMmioSiteLimitObservation {
    pub(super) observed: u64,
    pub(super) limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RawMmioSymbolObservation<'a> {
    pub(super) symbol_name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CriticalRegionRuleObservation {
    effect: &'static str,
    rule: PolicyRule,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ModuleCapabilityObservation {
    pub(super) capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IrqCapabilityObservation<'a> {
    pub(super) symbol_name: &'a str,
    pub(super) capability: String,
    pub(super) provenance: Option<&'a ContractsProvenance>,
}

pub(super) enum EffectRuleObservation<'a> {
    NoYieldLimit(NoYieldLimitObservation<'a>),
    NoYieldUnbounded(NoYieldUnboundedObservation<'a>),
    IrqEffect {
        effect: &'a str,
        observation: IrqEffectObservation<'a>,
    },
    RawMmioForbidden(RawMmioForbiddenObservation),
    RawMmioSiteLimit(RawMmioSiteLimitObservation),
    RawMmioSymbol(RawMmioSymbolObservation<'a>),
    IrqRawMmioForbidden(IrqRawMmioForbiddenObservation),
    IrqRawMmioSiteLimit(IrqRawMmioSiteLimitObservation),
}

pub(super) enum CapabilityRuleObservation<'a> {
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

fn policy_rule_raw_mmio_forbid_violations<'a>(
    policy: &PolicyFile,
    view: &'a PolicyEvalView<'a>,
    rule: PolicyRule,
) -> Vec<RawMmioForbiddenObservation> {
    if policy_rule_raw_mmio_allow_global(policy, rule) {
        return Vec::new();
    }
    if !policy.kernel.allow_raw_mmio_symbols.is_empty() {
        return Vec::new();
    }
    (!view.raw_mmio_symbol_names.is_empty())
        .then_some(RawMmioForbiddenObservation)
        .into_iter()
        .collect()
}

fn policy_rule_raw_mmio_site_limit_violation(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
    rule: PolicyRule,
) -> Option<RawMmioSiteLimitObservation> {
    policy_rule_raw_mmio_site_limit(policy, rule)
        .filter(|limit| contracts.report.effects.raw_mmio_sites_count > *limit)
        .map(|limit| RawMmioSiteLimitObservation {
            observed: contracts.report.effects.raw_mmio_sites_count,
            limit,
        })
}

fn policy_rule_raw_mmio_symbol_violations<'a>(
    policy: &PolicyFile,
    view: &'a PolicyEvalView<'a>,
    rule: PolicyRule,
) -> Vec<RawMmioSymbolObservation<'a>> {
    if policy_rule_raw_mmio_allow_global(policy, rule) {
        return Vec::new();
    }
    let Some(allowlist) = policy_rule_raw_mmio_symbol_allowlist(policy, rule) else {
        return Vec::new();
    };
    if allowlist.is_empty() {
        return Vec::new();
    }

    let allowed = allowlist
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<BTreeSet<_>>();
    view.raw_mmio_symbol_names
        .iter()
        .copied()
        .filter(|symbol_name| !allowed.contains(symbol_name))
        .map(|symbol_name| RawMmioSymbolObservation { symbol_name })
        .collect()
}

fn policy_rule_irq_raw_mmio_forbid_violations(
    policy: &PolicyFile,
    view: &PolicyEvalView<'_>,
    rule: PolicyRule,
) -> Vec<IrqRawMmioForbiddenObservation> {
    if !policy_rule_forbid_raw_mmio_in_irq(policy, rule) {
        return Vec::new();
    }

    view.irq_symbol_names
        .iter()
        .any(|symbol_name| {
            view.symbol_by_name
                .get(*symbol_name)
                .is_some_and(|symbol| symbol.has_raw_mmio_usage())
        })
        .then_some(IrqRawMmioForbiddenObservation)
        .into_iter()
        .collect()
}

fn policy_rule_irq_raw_mmio_site_limit_violation(
    policy: &PolicyFile,
    view: &PolicyEvalView<'_>,
    rule: PolicyRule,
) -> Option<IrqRawMmioSiteLimitObservation> {
    let observed = view
        .irq_symbol_names
        .iter()
        .filter_map(|symbol_name| view.symbol_by_name.get(*symbol_name))
        .map(|symbol| symbol.raw_mmio_sites_count)
        .sum::<u64>();

    policy_rule_irq_raw_mmio_site_limit(policy, rule)
        .filter(|limit| observed > *limit)
        .map(|limit| IrqRawMmioSiteLimitObservation { observed, limit })
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
