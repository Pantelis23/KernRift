use std::fs;
use std::path::Path;

use jsonschema::JSONSchema;
use serde_json::Value;

use super::{
    CONTRACTS_SCHEMA_V1, CONTRACTS_SCHEMA_V2, CONTRACTS_SCHEMA_VERSION,
    CONTRACTS_SCHEMA_VERSION_V2, ContractsBundle, PolicyFile,
};
pub(crate) fn load_policy_file(policy_path: &str) -> Result<PolicyFile, String> {
    let policy_text = fs::read_to_string(Path::new(policy_path))
        .map_err(|e| format!("failed to read policy '{}': {}", policy_path, e))?;
    parse_policy_text(&policy_text, policy_path)
}

pub(crate) fn parse_policy_text(text: &str, source_name: &str) -> Result<PolicyFile, String> {
    let policy: PolicyFile = toml::from_str(text)
        .map_err(|e| format!("failed to parse policy '{}': {}", source_name, e))?;
    normalize_policy(policy, source_name)
}

pub(super) fn normalize_policy(
    mut policy: PolicyFile,
    source_name: &str,
) -> Result<PolicyFile, String> {
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
pub(crate) fn decode_contracts_bundle(
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

pub(crate) fn validate_json_against_schema_text(
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
