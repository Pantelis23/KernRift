use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::SigningKey;
use jsonschema::JSONSchema;
use predicates::str::contains;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CONTRACTS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v1.schema.json");
const CONTRACTS_SCHEMA_V2: &str =
    include_str!("../../../docs/schemas/kernrift_contracts_v2.schema.json");
const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn report_unknown_metric_exits_nonzero() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit")
        .arg("report")
        .arg("--metrics")
        .arg("max_lock_depth,unknown")
        .arg(fixture.as_os_str());
    cmd.assert()
        .failure()
        .stderr(contains("unsupported report metric"));
}

#[test]
fn check_unresolved_callee_exits_nonzero_with_hir_error() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("unresolved_callee.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert()
        .failure()
        .stderr(contains("undefined symbol 'no_such_symbol'"));
}

#[test]
fn check_yield_hidden_two_levels_exits_nonzero_with_lockgraph_message() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("yield_hidden_two_levels.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lockgraph_lines = stderr
        .lines()
        .filter(|line| line.starts_with("lockgraph: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lockgraph_lines,
        vec!["lockgraph: function 'outer' calls yielding callee 'mid' under lock(s): SchedLock"],
        "expected exactly one lockgraph line and no lockgraph noise, got:\n{}",
        stderr
    );
}

#[test]
fn emit_lockgraph_outputs_only_expected_top_level_keys() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit")
        .arg("lockgraph")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let json: Value = serde_json::from_str(&stdout).expect("lockgraph json");
    let keys = json
        .as_object()
        .expect("lockgraph object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from(["edges".to_string(), "max_lock_depth".to_string()])
    );
    let edges = json["edges"].as_array().expect("edges array");
    assert_eq!(edges.len(), 1, "callee_acquires_lock should emit one edge");
    for edge in edges {
        let obj = edge.as_object().expect("edge object");
        let edge_keys = obj.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(
            edge_keys,
            BTreeSet::from(["from".to_string(), "to".to_string()])
        );
        assert!(
            obj.get("from").and_then(|v| v.as_str()).is_some(),
            "edge.from must be a string"
        );
        assert!(
            obj.get("to").and_then(|v| v.as_str()).is_some(),
            "edge.to must be a string"
        );
    }
}

#[test]
fn emit_report_outputs_only_requested_keys() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit")
        .arg("report")
        .arg("--metrics")
        .arg("max_lock_depth,no_yield_spans")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let json: Value = serde_json::from_str(&stdout).expect("report json");
    let keys = json
        .as_object()
        .expect("report object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from(["max_lock_depth".to_string(), "no_yield_spans".to_string()])
    );
}

#[test]
fn emit_contracts_outputs_expected_schema_and_keys() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit")
        .arg("contracts")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let json: Value = serde_json::from_str(&stdout).expect("contracts json");

    let top_keys = json
        .as_object()
        .expect("contracts object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        top_keys,
        BTreeSet::from([
            "capabilities".to_string(),
            "facts".to_string(),
            "lockgraph".to_string(),
            "report".to_string(),
            "schema_version".to_string(),
        ])
    );
    assert_eq!(
        json["schema_version"],
        Value::String("kernrift_contracts_v1".to_string())
    );

    let fact_symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    assert!(
        !fact_symbols.is_empty(),
        "facts symbols should not be empty"
    );

    let first_symbol_keys = fact_symbols[0]
        .as_object()
        .expect("fact symbol object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        first_symbol_keys,
        BTreeSet::from([
            "attrs".to_string(),
            "caps_req".to_string(),
            "ctx_ok".to_string(),
            "eff_used".to_string(),
            "is_extern".to_string(),
            "name".to_string(),
        ])
    );
}

#[test]
fn check_with_contracts_out_writes_canonical_json() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-{}.json", ts));

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&out_path).expect("read contracts output");
    fs::remove_file(&out_path).ok();

    assert!(
        !text.contains('\n'),
        "contracts output file should be canonical (minified)"
    );
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema(&json);
    assert_eq!(
        json["schema_version"],
        Value::String("kernrift_contracts_v1".to_string())
    );
}

#[test]
fn check_with_contracts_out_must_fail_does_not_write_file() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("yield_hidden_two_levels.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-fail-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().failure();

    assert!(
        !out_path.exists(),
        "contracts output should not be created for failing input"
    );
}

#[test]
fn contracts_v2_contains_effect_and_critical_report_fields() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_yield.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-v2-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&out_path).expect("read contracts output");
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema_v2(&json);
    assert_eq!(
        json["schema_version"],
        Value::String("kernrift_contracts_v2".to_string())
    );
    let report = json["report"].as_object().expect("report object");
    let report_keys = report.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        report_keys,
        BTreeSet::from([
            "critical".to_string(),
            "effects".to_string(),
            "max_lock_depth".to_string(),
            "no_yield_spans".to_string(),
        ])
    );
    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let critical_symbol = symbols
        .iter()
        .find(|sym| sym["name"] == "critical_entry")
        .expect("critical_entry symbol");
    assert_eq!(critical_symbol["attrs"]["critical"], Value::Bool(true));
    assert!(
        json["report"]["effects"]["yield_sites_count"]
            .as_u64()
            .expect("yield count")
            >= 1
    );
    assert!(json["report"]["effects"]["alloc_sites_count"].is_u64());
    assert!(json["report"]["effects"]["block_sites_count"].is_u64());
    assert!(json["report"]["critical"]["depth_max"].is_u64());
    assert!(json["report"]["critical"]["violations"].is_array());

    fs::remove_file(&out_path).ok();
}

#[test]
fn policy_uses_ctx_reachable_from_contract_facts() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let policy_path = root.join("policies").join("kernel.toml");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-ctx-src-{}.contracts.json", ts));
    let mutated_fail_path = std::env::temp_dir().join(format!(
        "kernrift-policy-ctx-mut-fail-{}.contracts.json",
        ts
    ));
    let mutated_pass_path = std::env::temp_dir().join(format!(
        "kernrift-policy-ctx-mut-pass-{}.contracts.json",
        ts
    ));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_fail_path).ok();
    fs::remove_file(&mutated_pass_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    let helper = symbols
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["eff_transitive"] = json!(["alloc"]);
    helper["eff_provenance"] = json!([{
        "effect":"alloc",
        "provenance":{
            "direct": false,
            "via_callee": [],
            "via_extern": []
        }
    }]);
    fs::write(
        &mutated_fail_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated fail contracts");

    let mut policy_fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_fail_path.as_os_str());
    let fail_assert = policy_fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    assert!(
        fail_stderr.contains("policy: KERNEL_IRQ_ALLOC: function 'helper'"),
        "expected helper to fail when ctx_reachable includes irq, got:\n{}",
        fail_stderr
    );

    let mut contracts_json_pass = contracts_json.clone();
    let symbols_pass = contracts_json_pass["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    let helper_pass = symbols_pass
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper_pass["ctx_reachable"] = json!([]);
    fs::write(
        &mutated_pass_path,
        serde_json::to_string(&contracts_json_pass).expect("contracts json text"),
    )
    .expect("write mutated pass contracts");

    let mut policy_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_pass_path.as_os_str());
    policy_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_fail_path).ok();
    fs::remove_file(&mutated_pass_path).ok();
}

#[test]
fn policy_uses_critical_report_violations_without_reconstruction() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_yield.kr");
    let policy_path = root.join("policies").join("kernel.toml");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!(
        "kernrift-policy-critical-src-{}.contracts.json",
        ts
    ));
    let mutated_path = std::env::temp_dir().join(format!(
        "kernrift-policy-critical-mut-{}.contracts.json",
        ts
    ));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut policy_fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let fail_assert = policy_fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    assert!(
        fail_stderr.contains("policy: KERNEL_CRITICAL_REGION_YIELD:"),
        "expected critical region deny from emitted report facts, got:\n{}",
        fail_stderr
    );

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    contracts_json["report"]["critical"]["violations"] = json!([]);
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated critical contracts");

    let mut policy_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    policy_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn contracts_v2_critical_report_includes_transitive_violation_details() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_yield.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-critical-report-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);
    let violations = json["report"]["critical"]["violations"]
        .as_array()
        .expect("critical violations");
    assert!(
        violations.iter().any(|v| v["function"] == "entry"
            && v["effect"] == "yield"
            && v["provenance"]["direct"] == Value::Bool(false)
            && v["provenance"]["via_callee"] == json!(["helper"])
            && v["provenance"]["via_extern"] == json!([])),
        "expected transitive critical-region yield violation in report, got {:?}",
        violations
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_attrs_include_critical_flag() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_attr.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-v2-critical-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&out_path).expect("read contracts output");
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let critical_symbol = symbols
        .iter()
        .find(|sym| sym["name"] == "critical_entry")
        .expect("critical_entry symbol must exist");
    assert_eq!(critical_symbol["attrs"]["critical"], Value::Bool(true));

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_effect_counters_track_alloc_and_block_sites() {
    let root = repo_root();
    let alloc_fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_site.kr");
    let block_fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_block_site.kr");

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let alloc_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-alloc-{}.json", ts));
    let block_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-block-{}.json", ts));
    fs::remove_file(&alloc_out).ok();
    fs::remove_file(&block_out).ok();

    let mut alloc_cmd: Command = cargo_bin_cmd!("kernriftc");
    alloc_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(alloc_out.as_os_str())
        .arg(alloc_fixture.as_os_str());
    alloc_cmd.assert().success();

    let mut block_cmd: Command = cargo_bin_cmd!("kernriftc");
    block_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(block_out.as_os_str())
        .arg(block_fixture.as_os_str());
    block_cmd.assert().success();

    let alloc_json: Value =
        serde_json::from_str(&fs::read_to_string(&alloc_out).expect("alloc contracts"))
            .expect("alloc json");
    let block_json: Value =
        serde_json::from_str(&fs::read_to_string(&block_out).expect("block contracts"))
            .expect("block json");
    validate_contracts_schema_v2(&alloc_json);
    validate_contracts_schema_v2(&block_json);

    assert!(
        alloc_json["report"]["effects"]["alloc_sites_count"]
            .as_u64()
            .expect("alloc count")
            >= 1
    );
    assert!(
        block_json["report"]["effects"]["block_sites_count"]
            .as_u64()
            .expect("block count")
            >= 1
    );

    fs::remove_file(&alloc_out).ok();
    fs::remove_file(&block_out).ok();
}

#[test]
fn contracts_v2_facts_include_transitive_effects() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-eff-transitive-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(
        helper["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        helper["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": false,
                "via_callee": ["helper"],
                "via_extern": []
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_include_transitive_capabilities() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_caps.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-cap-transitive-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let isr = symbols
        .iter()
        .find(|sym| sym["name"] == "isr")
        .expect("isr symbol");

    assert_eq!(
        helper["caps_transitive"],
        Value::Array(vec![Value::String("PhysMap".to_string())])
    );
    assert_eq!(
        helper["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        isr["caps_transitive"],
        Value::Array(vec![Value::String("PhysMap".to_string())])
    );
    assert_eq!(
        isr["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": false,
                "via_callee": ["helper"],
                "via_extern": []
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_capabilities_include_extern_stubs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_caps_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-cap-extern-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let map_io = symbols
        .iter()
        .find(|sym| sym["name"] == "map_io")
        .expect("map_io symbol");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let isr = symbols
        .iter()
        .find(|sym| sym["name"] == "isr")
        .expect("isr symbol");

    assert_eq!(map_io["is_extern"], Value::Bool(true));
    assert_eq!(
        map_io["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        helper["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": false,
                "via_callee": [],
                "via_extern": ["map_io"]
            }
        }])
    );
    assert_eq!(
        isr["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": false,
                "via_callee": ["helper"],
                "via_extern": ["map_io"]
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_effects_include_eff_attr_annotations() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc_attr.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-eff-attr-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(
        helper["eff_used"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        helper["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_transitive_effects_include_extern_eff_stubs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("transitive_alloc_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!(
        "kernrift-contracts-v2-eff-extern-transitive-{}.json",
        ts
    ));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let kmalloc = symbols
        .iter()
        .find(|sym| sym["name"] == "kmalloc")
        .expect("kmalloc symbol");
    let entry = symbols
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");

    assert_eq!(kmalloc["is_extern"], Value::Bool(true));
    assert_eq!(
        kmalloc["eff_used"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        kmalloc["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        kmalloc["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        entry["eff_transitive"],
        Value::Array(vec![Value::String("alloc".to_string())])
    );
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": false,
                "via_callee": [],
                "via_extern": ["kmalloc"]
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_facts_include_ctx_reachable_transitive_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-ctx-reach-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);
    let symbols = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols array");
    let helper = symbols
        .iter()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    assert!(
        helper["ctx_reachable"]
            .as_array()
            .expect("ctx_reachable array")
            .iter()
            .any(|ctx| ctx == "irq"),
        "expected helper ctx_reachable to include irq"
    );

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_ctx_reachable_transitive_irq_drives_policy() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_ALLOC: function 'helper'"),
        "expected transitive KERNEL_IRQ_ALLOC for helper, got:\n{}",
        stderr
    );
}

#[test]
fn contracts_v2_semantic_fields_coexist_and_validate_schema() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out_path = std::env::temp_dir().join(format!("kernrift-contracts-v2-abi-{}.json", ts));
    fs::remove_file(&out_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(out_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let json: Value = serde_json::from_str(&fs::read_to_string(&out_path).expect("contracts text"))
        .expect("contracts json");
    validate_contracts_schema_v2(&json);

    let entry = json["facts"]["symbols"]
        .as_array()
        .expect("facts symbols")
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("entry symbol");
    assert_eq!(entry["ctx_reachable"], json!(["irq"]));
    assert_eq!(entry["eff_transitive"], json!(["alloc"]));
    assert_eq!(
        entry["eff_provenance"],
        json!([{
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(entry["caps_transitive"], json!(["PhysMap"]));
    assert_eq!(
        entry["caps_provenance"],
        json!([{
            "capability": "PhysMap",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );
    assert_eq!(
        json["report"]["critical"]["violations"],
        json!([{
            "function": "entry",
            "effect": "alloc",
            "provenance": {
                "direct": true,
                "via_callee": [],
                "via_extern": []
            }
        }])
    );

    fs::remove_file(&out_path).ok();
}

fn validate_contracts_schema(instance: &Value) {
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

fn validate_contracts_schema_v2(instance: &Value) {
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

fn compile_verify_report_schema() -> JSONSchema {
    let schema_json: Value = serde_json::from_str(VERIFY_REPORT_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn assert_schema_rejects(compiled: &JSONSchema, instance: &Value, needle: &str) {
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

fn write_test_keypair(secret_path: &Path, pubkey_path: &Path) {
    let secret = std::array::from_fn::<u8, 32, _>(|i| (i as u8).wrapping_add(1));
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    fs::write(secret_path, format!("{}\n", hex_encode(&secret))).expect("write secret key");
    fs::write(pubkey_path, format!("{}\n", hex_encode(&pubkey))).expect("write pubkey");
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

fn nibble_to_hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}

#[test]
fn policy_passes_for_compliant_contracts() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-pass-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-pass-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]

[caps]
allow_module = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_fails_with_deterministic_ordered_errors() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-fail-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-fail-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_caps_allow_module_rejects_disallowed_caps() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-caps-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-caps-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[caps]
allow_module = ["IoPort"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains(
            "policy: CAP_MODULE_ALLOWLIST: module capability 'PhysMap' is not in allow_module"
        ),
        "expected caps allowlist violation, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_forbid_caps_in_irq_is_artifact_driven_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-irq-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-cap-irq-{}.toml", ts));
    let mutated_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-irq-mut-{}.json", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let fail_assert = fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    let lines = fail_stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=false, via_callee=[helper], via_extern=[])",
        ],
        "expected deterministic capability violations, got:\n{}",
        fail_stderr
    );

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    for symbol in symbols {
        symbol["caps_transitive"] = json!([]);
        symbol["caps_provenance"] = json!([]);
    }
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn policy_kernel_allow_caps_in_irq_overrides_forbid() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-allow-override-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-allow-override-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_non_listed_irq_capability_is_allowed() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_unlisted.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-unlisted-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-unlisted-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_forbid_caps_in_irq_via_extern_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-cap-extern-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-cap-extern-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=false, via_callee=[], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=false, via_callee=[helper], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'map_io' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
        ],
        "expected deterministic extern capability propagation violations, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_kernel_capability_rule_requires_contracts_v2() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-v1-cap-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-v1-cap-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = ["IoPort"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_POLICY_REQUIRES_V2: kernel policy rules require contracts schema 'kernrift_contracts_v2'"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_outputs_cross_family_violations_in_deterministic_order() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-families-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-families-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]

[kernel]
forbid_alloc_in_irq = true
forbid_effects_in_critical = ["alloc"]
forbid_caps_in_irq = ["IoPort", "PhysMap"]
allow_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ],
        "cross-family ordering must be deterministic, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_catalog_rank_order_is_deterministic_across_families() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-rank-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-rank-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]

[caps]
allow_module = ["IoPort"]

[kernel]
forbid_alloc_in_irq = true
forbid_effects_in_critical = ["alloc"]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = []
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: CAP_MODULE_ALLOWLIST: module capability 'PhysMap' is not in allow_module",
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'entry' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ],
        "catalog rank ordering must be deterministic, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_bad_parse_exits_with_invalid_input_code() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-policy-bad-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-bad-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(&policy_path, "[limits\nmax_lock_depth = 1\n").expect("write bad policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    policy_cmd.assert().failure().code(2);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allow_caps_in_irq_empty_entry_exits_with_invalid_input_code() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-allow-cap-empty-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-allow-cap-empty-{}.toml", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &policy_path,
        r#"
[kernel]
allow_caps_in_irq = [""]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("allow_caps_in_irq entries must be non-empty strings"),
        "expected allow_caps_in_irq parse validation error, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_pass_writes_contracts_out() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-check-policy-pass-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-pass-{}.toml", ts));

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2

[locks]
forbid_edges = [["RunQueueLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&contracts_path).expect("read contracts output");
    assert!(
        !text.is_empty(),
        "contracts output should be written on pass"
    );
    let json: Value = serde_json::from_str(&text).expect("contracts json");
    validate_contracts_schema(&json);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_fail_does_not_write_file() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-check-policy-fail-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-fail-{}.toml", ts));
    fs::remove_file(&contracts_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().failure().code(1);

    assert!(
        !contracts_path.exists(),
        "contracts output should not be written when policy denies"
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_fail_has_deterministic_lines() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-check-policy-lines-{}.toml", ts));

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1

[locks]
forbid_edges = [["ConsoleLock", "SchedLock"]]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ]
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_pass_writes_contracts_hash_sig_and_verifies() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-p84-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);
    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let contracts_bytes = fs::read(&contracts_path).expect("read contracts");
    let expected_hash = sha256_hex(&contracts_bytes);
    let got_hash = fs::read_to_string(&hash_path).expect("read hash");
    assert_eq!(
        got_hash.trim(),
        expected_hash,
        "hash file must match contracts bytes"
    );
    let sig_text = fs::read_to_string(&sig_path).expect("read sig");
    assert!(!sig_text.trim().is_empty(), "sig file should not be empty");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str());
    verify_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_policy_deny_writes_nothing_even_if_hash_sig_flags_present() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-deny-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-deny-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-deny-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-deny-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-deny-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-deny-pubkey-{}.hex", ts));
    write_test_keypair(&secret_path, &pubkey_path);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 1
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().failure().code(1);

    assert!(
        !contracts_path.exists(),
        "contracts file must not be written on deny"
    );
    assert!(!hash_path.exists(), "hash file must not be written on deny");
    assert!(!sig_path.exists(), "sig file must not be written on deny");

    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_invalid_key_exits_2_and_writes_nothing() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-badkey-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-badkey-policy-{}.toml", ts));
    let bad_secret_path =
        std::env::temp_dir().join(format!("kernrift-p84-badkey-secret-{}.hex", ts));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();

    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");
    fs::write(&bad_secret_path, "zz\n").expect("write bad key");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(bad_secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().failure().code(2);

    assert!(
        !contracts_path.exists(),
        "contracts file must not be written on invalid key"
    );
    assert!(
        !hash_path.exists(),
        "hash file must not be written on invalid key"
    );
    assert!(
        !sig_path.exists(),
        "sig file must not be written on invalid key"
    );

    fs::remove_file(&policy_path).ok();
    fs::remove_file(&bad_secret_path).ok();
}

#[test]
fn verify_rejects_mismatched_hash_or_sig() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-vrf-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-sig-{}.sig", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-policy-{}.toml", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-p84-vrf-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);
    fs::write(
        &policy_path,
        r#"
[limits]
max_lock_depth = 2
"#,
    )
    .expect("write policy");

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg("--sign-ed25519")
        .arg(secret_path.as_os_str())
        .arg("--sig-out")
        .arg(sig_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write tampered hash");
    let mut verify_hash_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_hash_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    verify_hash_cmd.assert().failure().code(1);

    let contracts_bytes = fs::read(&contracts_path).expect("read contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(&contracts_bytes))).expect("restore hash");
    let bad_sig = BASE64_STANDARD.encode([0_u8; 64]);
    fs::write(&sig_path, format!("{bad_sig}\n")).expect("tamper sig");
    let mut verify_sig_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_sig_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--sig")
        .arg(sig_path.as_os_str())
        .arg("--pubkey")
        .arg(pubkey_path.as_os_str());
    verify_sig_cmd.assert().failure().code(1);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn check_refuses_overwriting_existing_outputs() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-p84-overwrite-{}.json", ts));

    fs::write(&contracts_path, "sentinel\n").expect("write sentinel");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().failure().code(2);

    let after = fs::read_to_string(&contracts_path).expect("read sentinel");
    assert_eq!(after, "sentinel\n", "existing output must remain untouched");

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn verify_rejects_schema_invalid_even_with_matching_hash() {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-schema-invalid-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-p84-schema-invalid-{}.sha256", ts));

    let garbage = b"{\"not_contracts\":true}";
    fs::write(&contracts_path, garbage).expect("write garbage contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(garbage))).expect("write matching hash");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    cmd.assert().failure().code(2);

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
}

#[test]
fn verify_rejects_contracts_with_unknown_top_level_key() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-p84-unknown-contract-key-{}.json", ts));
    let hash_path =
        std::env::temp_dir().join(format!("kernrift-p84-unknown-contract-key-{}.sha256", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("read contracts"))
            .expect("contracts json");
    contracts
        .as_object_mut()
        .expect("contracts object")
        .insert("unexpected".to_string(), Value::Bool(true));
    let tampered = serde_json::to_string(&contracts).expect("serialize tampered contracts");
    fs::write(&contracts_path, tampered.as_bytes()).expect("write tampered contracts");
    fs::write(&hash_path, format!("{}\n", sha256_hex(tampered.as_bytes())))
        .expect("write matching tampered hash");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str());
    verify_cmd
        .assert()
        .failure()
        .code(2)
        .stderr(contains("contracts schema validation failed"));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
}

#[test]
fn verify_report_schema_rejects_unknown_keys_and_invalid_result() {
    let compiled = compile_verify_report_schema();
    let mut valid = json!({
        "schema_version": "kernrift_verify_report_v1",
        "result": "pass",
        "inputs": {
            "contracts": "contracts.json",
            "hash": "contracts.sha256",
            "sig": null,
            "pubkey": null
        },
        "hash": {
            "expected_sha256": "0".repeat(64),
            "computed_sha256": "0".repeat(64),
            "matched": true
        },
        "contracts": {
            "utf8_valid": true,
            "schema_valid": true,
            "schema_version": "kernrift_contracts_v1"
        },
        "signature": {
            "checked": false,
            "valid": null
        },
        "diagnostics": []
    });
    if let Err(errors) = compiled.validate(&valid) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "expected valid verify report schema instance, got: {}",
            details.join(" | ")
        );
    }

    valid
        .as_object_mut()
        .expect("report object")
        .insert("unexpected".to_string(), Value::Bool(true));
    assert_schema_rejects(&compiled, &valid, "Additional properties are not allowed");

    let mut invalid_nested = json!({
        "schema_version": "kernrift_verify_report_v1",
        "result": "pass",
        "inputs": {
            "contracts": "contracts.json",
            "hash": "contracts.sha256",
            "sig": null,
            "pubkey": null,
            "extra": "nope"
        },
        "hash": {
            "expected_sha256": "0".repeat(64),
            "computed_sha256": "0".repeat(64),
            "matched": true
        },
        "contracts": {
            "utf8_valid": true,
            "schema_valid": true,
            "schema_version": "kernrift_contracts_v1"
        },
        "signature": {
            "checked": false,
            "valid": null
        },
        "diagnostics": []
    });
    assert_schema_rejects(
        &compiled,
        &invalid_nested,
        "Additional properties are not allowed",
    );

    invalid_nested["inputs"] = json!({
        "contracts": "contracts.json",
        "hash": "contracts.sha256",
        "sig": null,
        "pubkey": null
    });
    invalid_nested["result"] = Value::String("maybe".to_string());
    assert_schema_rejects(&compiled, &invalid_nested, "\"maybe\" is not one of");
}

#[test]
fn policy_rejects_unbounded_no_yield_spans() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("thread_no_yield_unbounded.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-no-yield-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[limits]
forbid_unbounded_no_yield = true
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'helper' is unbounded",
            "policy: NO_YIELD_UNBOUNDED: no_yield_spans 'worker' is unbounded",
        ]
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn verify_report_success_is_deterministic_and_path_stripped() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("locks_ok.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-report-{}.report.json", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    let keys = report_json
        .as_object()
        .expect("verify report object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "contracts".to_string(),
            "diagnostics".to_string(),
            "hash".to_string(),
            "inputs".to_string(),
            "result".to_string(),
            "schema_version".to_string(),
            "signature".to_string(),
        ])
    );
    assert_eq!(
        report_json["schema_version"],
        Value::String("kernrift_verify_report_v1".to_string())
    );
    assert_eq!(report_json["result"], Value::String("pass".to_string()));
    assert_eq!(report_json["hash"]["matched"], Value::Bool(true));
    assert_eq!(
        report_json["diagnostics"],
        Value::Array(vec![]),
        "verify report diagnostics should be empty on success"
    );

    let contracts_name = contracts_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("contracts basename");
    let hash_name = hash_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("hash basename");
    let report_contracts_path = report_json["inputs"]["contracts"]
        .as_str()
        .expect("report contracts path");
    let report_hash_path = report_json["inputs"]["hash"]
        .as_str()
        .expect("report hash path");
    assert_eq!(report_contracts_path, contracts_name);
    assert_eq!(report_hash_path, hash_name);
    assert!(
        !report_contracts_path.contains('/'),
        "verify report should strip absolute paths"
    );
    assert!(
        !report_hash_path.contains('/'),
        "verify report should strip absolute paths"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}

#[test]
fn verify_report_records_hash_mismatch_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-deny-{}.report.json", ts));

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg("--hash-out")
        .arg(hash_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("tamper hash");

    let mut verify_cmd: Command = cargo_bin_cmd!("kernriftc");
    verify_cmd
        .current_dir(&root)
        .arg("verify")
        .arg("--contracts")
        .arg(contracts_path.as_os_str())
        .arg("--hash")
        .arg(hash_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().failure().code(1);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(report_json["result"], Value::String("deny".to_string()));
    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with("verify: HASH_MISMATCH:"),
        "unexpected diagnostics: {:?}",
        diagnostics
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}
