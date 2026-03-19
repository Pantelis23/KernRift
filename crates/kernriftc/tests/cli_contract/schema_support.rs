use super::*;

pub(super) fn validate_contracts_schema(instance: &Value) {
    let schema_json: Value = serde_json::from_str(CONTRACTS_SCHEMA_V1).expect("schema json");
    let compiled = JSONSchema::compile(&schema_json).expect("compile schema");
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "contracts JSON must validate against contracts_v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_contracts_schema_v2(instance: &Value) {
    let schema_json: Value = serde_json::from_str(CONTRACTS_SCHEMA_V2).expect("schema json");
    let compiled = JSONSchema::compile(&schema_json).expect("compile schema");
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "contracts JSON must validate against contracts_v2 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn compile_verify_report_schema() -> JSONSchema {
    let schema_json: Value = serde_json::from_str(VERIFY_REPORT_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_inspect_report_schema() -> JSONSchema {
    let schema_json: Value = serde_json::from_str(INSPECT_REPORT_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_inspect_artifact_schema() -> JSONSchema {
    let schema_json: Value = serde_json::from_str(INSPECT_ARTIFACT_SCHEMA_V2).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_verify_artifact_meta_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(VERIFY_ARTIFACT_META_SCHEMA_V2).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_policy_violations_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(POLICY_VIOLATIONS_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_canonical_findings_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FINDINGS_SCHEMA_V2).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_canonical_edit_plan_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_EDIT_PLAN_SCHEMA_V2).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_canonical_fix_result_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FIX_RESULT_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn compile_canonical_fix_preview_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FIX_PREVIEW_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

pub(super) fn validate_policy_violations_schema(instance: &Value) {
    let compiled = compile_policy_violations_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "policy JSON must validate against policy violations v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_inspect_artifact_schema(instance: &Value) {
    let compiled = compile_inspect_artifact_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "inspect-artifact JSON must validate against inspect-artifact v2 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_inspect_report_schema(instance: &Value) {
    let compiled = compile_inspect_report_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "inspect-report JSON must validate against inspect-report v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_verify_artifact_meta_schema(instance: &Value) {
    let compiled = compile_verify_artifact_meta_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "verify-artifact-meta JSON must validate against verify-artifact-meta v2 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_canonical_findings_schema(instance: &Value) {
    let compiled = compile_canonical_findings_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical findings JSON must validate against canonical findings v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_canonical_edit_plan_schema(instance: &Value) {
    let compiled = compile_canonical_edit_plan_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical edit plan JSON must validate against canonical edit plan v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_canonical_fix_result_schema(instance: &Value) {
    let compiled = compile_canonical_fix_result_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical fix result JSON must validate against canonical fix result v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn validate_canonical_fix_preview_schema(instance: &Value) {
    let compiled = compile_canonical_fix_preview_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical fix preview JSON must validate against canonical fix preview v1 schema: {}",
            details.join(" | ")
        );
    }
}

pub(super) fn assert_schema_rejects(compiled: &JSONSchema, instance: &Value, needle: &str) {
    let errors = compiled
        .validate(instance)
        .expect_err("instance should fail verify report schema");
    let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
    assert!(
        details.iter().any(|line| line.contains(needle)),
        "expected schema error containing '{needle}', got: {}",
        details.join(" | ")
    );
}

pub(super) fn write_test_keypair(secret_path: &Path, pubkey_path: &Path) {
    let secret = std::array::from_fn::<u8, 32, _>(|i| (i as u8).wrapping_add(1));
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    fs::write(secret_path, format!("{}\n", hex_encode(&secret))).expect("write secret key");
    fs::write(pubkey_path, format!("{}\n", hex_encode(&pubkey))).expect("write pubkey");
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

pub(super) fn nibble_to_hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}
