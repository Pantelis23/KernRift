#[cfg(test)]
use std::collections::BTreeSet;

use super::{
    POLICY_RULE_CATALOG, PolicyEnablementProbe, PolicyFile, PolicyMaterializationAction,
    PolicyRule, normalize_policy, policy_rule_spec,
};
pub(super) fn kernel_profile_default_rules() -> Vec<PolicyRule> {
    let mut rules = POLICY_RULE_CATALOG
        .iter()
        .filter(|spec| spec.default_enabled_in_profile_kernel)
        .map(|spec| spec.rule)
        .collect::<Vec<_>>();
    rules.sort_by_key(|rule| policy_rule_spec(*rule).sort_rank);
    rules.dedup();
    rules
}

pub(crate) fn materialize_kernel_profile_policy() -> Result<PolicyFile, String> {
    let mut policy = PolicyFile::default();
    for rule in kernel_profile_default_rules() {
        materialize_kernel_profile_rule(&mut policy, rule);
    }
    normalize_policy(policy, "<materialized-kernel-profile>")
}

pub(super) fn materialize_kernel_profile_rule(policy: &mut PolicyFile, rule: PolicyRule) {
    for action in policy_rule_spec(rule).materialization_actions {
        apply_policy_materialization_action(policy, *action);
    }
}

pub(super) fn apply_policy_materialization_action(
    policy: &mut PolicyFile,
    action: PolicyMaterializationAction,
) {
    match action {
        PolicyMaterializationAction::SetAllowRawMmio(allow) => {
            policy.kernel.allow_raw_mmio = Some(allow);
        }
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

pub(super) fn policy_rule_is_enabled(policy: &PolicyFile, rule: PolicyRule) -> bool {
    let spec = policy_rule_spec(rule);
    spec.enablement_probes
        .iter()
        .any(|probe| policy_enablement_probe_enabled(policy, *probe))
}

fn policy_enablement_probe_enabled(policy: &PolicyFile, probe: PolicyEnablementProbe) -> bool {
    match probe {
        PolicyEnablementProbe::KernelRawMmioForbidConfigured => {
            policy.kernel.allow_raw_mmio.is_some()
        }
        PolicyEnablementProbe::KernelRawMmioSiteLimitConfigured => {
            policy.kernel.max_raw_mmio_sites.is_some()
        }
        PolicyEnablementProbe::KernelRawMmioSymbolAllowlistConfigured => {
            !policy.kernel.allow_raw_mmio_symbols.is_empty()
        }
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
        PolicyEnablementProbe::KernelV2RulesEnabled => kernel_v2_rules_enabled(policy),
    }
}

#[cfg(test)]
pub(super) fn enabled_policy_rules(policy: &PolicyFile) -> BTreeSet<PolicyRule> {
    POLICY_RULE_CATALOG
        .iter()
        .filter_map(|spec| policy_rule_is_enabled(policy, spec.rule).then_some(spec.rule))
        .collect::<BTreeSet<_>>()
}

pub(super) fn kernel_v2_rules_enabled(policy: &PolicyFile) -> bool {
    POLICY_RULE_CATALOG
        .iter()
        .filter(|spec| spec.requires_v2)
        .any(|spec| policy_rule_is_enabled(policy, spec.rule))
}
