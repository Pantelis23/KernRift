use super::super::{
    CONTRACTS_SCHEMA_VERSION_V2, ContractsCriticalViolation, ContractsProvenance, PolicyFamily,
    PolicyRule, PolicyViolation, PolicyViolationBinderKind, policy_rule_binder_kind,
    policy_rule_spec,
};
use super::common::{
    canonicalize_provenance_fields, format_optional_provenance, format_provenance,
};
use super::rules::{
    CapabilityRuleObservation, EffectRuleObservation, IrqCapabilityObservation,
    IrqEffectObservation, IrqRawMmioForbiddenObservation, IrqRawMmioSiteLimitObservation,
    LockDepthObservation, LockRuleObservation, ModuleCapabilityObservation,
    NoYieldLimitObservation, NoYieldUnboundedObservation, RawMmioForbiddenObservation,
    RawMmioSiteLimitObservation, RawMmioSymbolObservation, SchemaMismatchObservation,
};

pub(super) fn bind_context_rule_violation(
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

pub(super) fn bind_lock_rule_violation(
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

pub(super) fn bind_effect_rule_violation(
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
        EffectRuleObservation::RawMmioForbidden(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::RawMmioForbidden);
            match binder_kind {
                PolicyViolationBinderKind::RawMmioForbidden => {
                    bind_raw_mmio_forbidden_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::RawMmioSiteLimit(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::RawMmioSiteLimit);
            match binder_kind {
                PolicyViolationBinderKind::RawMmioSiteLimit => {
                    bind_raw_mmio_site_limit_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::RawMmioSymbol(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::RawMmioSymbol);
            match binder_kind {
                PolicyViolationBinderKind::RawMmioSymbol => {
                    bind_raw_mmio_symbol_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::IrqRawMmioForbidden(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::IrqRawMmioForbidden);
            match binder_kind {
                PolicyViolationBinderKind::IrqRawMmioForbidden => {
                    bind_irq_raw_mmio_forbidden_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
        EffectRuleObservation::IrqRawMmioSiteLimit(observation) => {
            debug_assert_eq!(binder_kind, PolicyViolationBinderKind::IrqRawMmioSiteLimit);
            match binder_kind {
                PolicyViolationBinderKind::IrqRawMmioSiteLimit => {
                    bind_irq_raw_mmio_site_limit_observation(observation)
                }
                _ => unreachable!("unexpected binder kind for {:?}", rule),
            }
        }
    }
}

pub(super) fn bind_region_rule_violation(
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

pub(super) fn bind_capability_rule_violation(
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

pub(in crate::policy_engine) fn policy_violation(rule: PolicyRule, msg: String) -> PolicyViolation {
    policy_violation_with_evidence(rule, msg, Vec::new())
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

fn bind_schema_mismatch_observation(_observation: SchemaMismatchObservation) -> PolicyViolation {
    violation_kernel_policy_requires_v2()
}

fn bind_lock_depth_observation(observation: LockDepthObservation) -> PolicyViolation {
    violation_limit_max_lock_depth(observation.observed, observation.limit)
}

fn bind_forbidden_lock_edge_observation(
    observation: super::rules::ForbiddenLockEdgeObservation<'_>,
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

fn bind_raw_mmio_forbidden_observation(
    _observation: RawMmioForbiddenObservation,
) -> PolicyViolation {
    violation_kernel_raw_mmio_forbid()
}

fn bind_raw_mmio_site_limit_observation(
    observation: RawMmioSiteLimitObservation,
) -> PolicyViolation {
    violation_kernel_raw_mmio_site_limit(observation.observed, observation.limit)
}

fn bind_raw_mmio_symbol_observation(observation: RawMmioSymbolObservation<'_>) -> PolicyViolation {
    violation_kernel_raw_mmio_symbol_allowlist(observation.symbol_name)
}

fn bind_irq_raw_mmio_forbidden_observation(
    _observation: IrqRawMmioForbiddenObservation,
) -> PolicyViolation {
    violation_kernel_irq_raw_mmio_forbid()
}

fn bind_irq_raw_mmio_site_limit_observation(
    observation: IrqRawMmioSiteLimitObservation,
) -> PolicyViolation {
    violation_kernel_irq_raw_mmio_site_limit(observation.observed, observation.limit)
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

fn violation_kernel_raw_mmio_forbid() -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelRawMmioForbid,
        "raw_mmio is not allowed".to_string(),
    )
}

fn violation_kernel_raw_mmio_site_limit(observed: u64, limit: u64) -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelRawMmioSiteLimit,
        format!(
            "raw_mmio_sites_count {} exceeds allowed maximum {}",
            observed, limit
        ),
    )
}

fn violation_kernel_raw_mmio_symbol_allowlist(symbol_name: &str) -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelRawMmioSymbolAllowlist,
        format!("raw_mmio symbol '{}' is not allowed", symbol_name),
    )
}

fn violation_kernel_irq_raw_mmio_forbid() -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelIrqRawMmioForbid,
        "raw_mmio is not allowed in irq context".to_string(),
    )
}

fn violation_kernel_irq_raw_mmio_site_limit(observed: u64, limit: u64) -> PolicyViolation {
    policy_violation(
        PolicyRule::KernelIrqRawMmioSiteLimit,
        format!(
            "irq raw_mmio_sites_count {} exceeds allowed maximum {}",
            observed, limit
        ),
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
