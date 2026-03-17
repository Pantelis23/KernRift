use super::{ContractsBundle, PolicyFile, PolicyViolation};

mod common;
mod rules;
mod summary;
mod violation;

pub(crate) use summary::{contracts_bundle_schema_version, format_contracts_inspect_summary};
#[cfg(test)]
pub(in crate::policy_engine) use violation::policy_violation;
pub(crate) use violation::{
    emit_policy_violations_json, format_policy_violation, print_policy_violations,
};

pub(crate) fn evaluate_policy(
    policy: &PolicyFile,
    contracts: &ContractsBundle,
) -> Vec<PolicyViolation> {
    let view = rules::PolicyEvalView::build(contracts);
    let mut violations = Vec::<PolicyViolation>::new();

    let (context_violations, kernel_v2_allowed) = rules::evaluate_context_rules(policy, contracts);
    violations.extend(context_violations);
    violations.extend(rules::evaluate_lock_rules(policy, contracts));
    violations.extend(rules::evaluate_effect_rules(
        policy,
        contracts,
        &view,
        kernel_v2_allowed,
    ));
    violations.extend(rules::evaluate_region_rules(
        policy,
        contracts,
        kernel_v2_allowed,
    ));
    violations.extend(rules::evaluate_capability_rules(
        policy,
        contracts,
        &view,
        kernel_v2_allowed,
    ));

    violations.sort();
    violations.dedup();
    violations
}
