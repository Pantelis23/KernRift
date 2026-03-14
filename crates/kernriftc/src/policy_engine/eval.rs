use std::collections::{BTreeMap, BTreeSet};

use super::{
    CONTRACTS_SCHEMA_VERSION_V2, ContractsBundle, ContractsCriticalViolation, ContractsFactSymbol,
    ContractsNoYieldSpan, ContractsProvenance, PolicyConditionDescriptor, PolicyFamily, PolicyFile,
    PolicyRule, PolicyViolation, PolicyViolationBinderKind, policy_family_specs,
    policy_rule_binder_kind, policy_rule_conditions, policy_rule_effect_condition,
    policy_rule_forbidden_lock_edges, policy_rule_irq_capability_lists, policy_rule_is_enabled,
    policy_rule_max_lock_depth, policy_rule_max_no_yield_span, policy_rule_module_cap_allowlist,
    policy_rule_spec,
};
pub(crate) fn evaluate_policy(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
) -> Vec<PolicyViolation> {
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

pub(crate) fn contracts_bundle_schema_version(contracts: &ContractsBundle) -> &str {
    &contracts.schema_version
}

pub(crate) fn format_contracts_inspect_summary(contracts: &ContractsBundle) -> String {
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
            format_bracketed_list(&irq_reachable)
        ),
        format!(
            "critical_functions: {} {}",
            critical_functions.len(),
            format_bracketed_list(&critical_functions)
        ),
        "effects:".to_string(),
        format!(
            "alloc: {} {}",
            alloc_symbols.len(),
            format_bracketed_list(&alloc_symbols)
        ),
        format!(
            "block: {} {}",
            block_symbols.len(),
            format_bracketed_list(&block_symbols)
        ),
        format!(
            "yield: {} {}",
            yield_symbols.len(),
            format_bracketed_list(&yield_symbols)
        ),
        "capabilities:".to_string(),
        format!(
            "symbols_with_caps: {} {}",
            cap_symbols.len(),
            format_bracketed_list(&cap_symbols)
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
            format_bracketed_list(&via_callee),
            format_bracketed_list(&via_extern)
        ));
    }

    lines.join("\n")
}

fn collect_sorted_symbol_names_by<F>(symbols: &[ContractsFactSymbol], predicate: F) -> Vec<String>
where
    F: Fn(&ContractsFactSymbol) -> bool,
{
    let mut out = symbols
        .iter()
        .filter(|symbol| predicate(symbol))
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn format_bracketed_list(items: &[String]) -> String {
    format!("[{}]", items.join(","))
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

pub(super) fn policy_violation(rule: PolicyRule, msg: String) -> PolicyViolation {
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

pub(crate) fn format_policy_violation(violation: &PolicyViolation) -> String {
    match violation.family {
        PolicyFamily::Context => format_context_violation(violation),
        PolicyFamily::Lock => format_lock_violation(violation),
        PolicyFamily::Effect => format_effect_violation(violation),
        PolicyFamily::Region => format_region_violation(violation),
        PolicyFamily::Capability => format_capability_violation(violation),
        PolicyFamily::Limit => format_limit_violation(violation),
    }
}

pub(crate) fn print_policy_violations(violations: &[PolicyViolation], evidence: bool) {
    for violation in violations {
        eprintln!("{}", format_policy_violation(violation));
        if evidence {
            for line in &violation.evidence {
                eprintln!("{}", line);
            }
        }
    }
}
