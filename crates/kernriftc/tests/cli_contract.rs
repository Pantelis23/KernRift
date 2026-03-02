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

fn write_v2_contracts_for_fixture(root: &Path, fixture: &Path, label: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-inspect-{}-{}.json", label, ts));
    fs::remove_file(&contracts_path).ok();

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    check_cmd.assert().success();

    contracts_path
}

fn inject_single_critical_violation(
    contracts_json: &mut Value,
    function: &str,
    effect: &str,
    direct: bool,
    via_callee: &[&str],
    via_extern: &[&str],
) {
    contracts_json["report"]["critical"]["violations"] = json!([{
        "function": function,
        "effect": effect,
        "provenance": {
            "direct": direct,
            "via_callee": via_callee,
            "via_extern": via_extern
        }
    }]);
}

fn write_verify_report_fixture(label: &str, report_json: &Value) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let report_path =
        std::env::temp_dir().join(format!("kernrift-inspect-report-{}-{}.json", label, ts));
    fs::remove_file(&report_path).ok();
    fs::write(
        &report_path,
        serde_json::to_string_pretty(report_json).expect("report json"),
    )
    .expect("write report fixture");
    report_path
}

fn write_promotion_repo_fixture(feature_id: &str) -> PathBuf {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let repo_dir = std::env::temp_dir().join(format!("kernrift-promotion-{}-{}", feature_id, ts));
    let hir_dir = repo_dir.join("crates").join("hir").join("src");
    let proposal_dir = repo_dir.join("docs").join("design").join("examples");
    fs::create_dir_all(&hir_dir).expect("create hir dir");
    fs::create_dir_all(&proposal_dir).expect("create proposal dir");
    fs::copy(
        root.join("crates").join("hir").join("src").join("lib.rs"),
        hir_dir.join("lib.rs"),
    )
    .expect("copy hir lib");
    for entry in
        fs::read_dir(root.join("docs").join("design").join("examples")).expect("read proposal dir")
    {
        let entry = entry.expect("proposal dir entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            fs::copy(
                &path,
                proposal_dir.join(path.file_name().expect("proposal file name")),
            )
            .expect("copy proposal example");
        }
    }
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.name", "KernRift Test"]);
    git(&["config", "user.email", "kernrift@example.test"]);
    git(&["add", "."]);
    git(&["commit", "-m", "baseline"]);
    repo_dir
}

fn git_commit_all(repo_dir: &Path, message: &str) {
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(repo_dir)
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    };
    git(&["add", "."]);
    git(&["commit", "-m", message]);
}

fn replace_once_in_file(path: &Path, from: &str, to: &str) {
    let src = fs::read_to_string(path).expect("read file");
    assert!(src.contains(from), "missing pattern '{}'", from);
    let updated = src.replacen(from, to, 1);
    fs::write(path, updated).expect("write file");
}

fn replace_json_string_field(path: &Path, field: &str, value: &str) {
    let src = fs::read_to_string(path).expect("read json file");
    let mut json: serde_json::Value = serde_json::from_str(&src).expect("parse json");
    let obj = json.as_object_mut().expect("json object");
    obj.insert(
        field.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    let mut text = serde_json::to_string_pretty(&json).expect("serialize json");
    text.push('\n');
    fs::write(path, text).expect("write json file");
}

fn hir_entry_slice(src: &str, section_marker: &str, entry_id: &str) -> String {
    let section_start = src.find(section_marker).expect("section marker");
    let id_marker = format!("        id: \"{}\",", entry_id);
    let entry_start = section_start + src[section_start..].find(&id_marker).expect("entry");
    let entry_end = entry_start + src[entry_start..].find("    },").expect("entry end");
    src[entry_start..entry_end].to_string()
}

fn replace_in_hir_entry(path: &Path, section_marker: &str, entry_id: &str, from: &str, to: &str) {
    let src = fs::read_to_string(path).expect("read hir file");
    let section_start = src.find(section_marker).expect("section marker");
    let id_marker = format!("        id: \"{}\",", entry_id);
    let relative_entry_start = src[section_start..].find(&id_marker).expect("entry");
    let entry_start = section_start + relative_entry_start;
    let relative_entry_end = src[entry_start..].find("    },").expect("entry end");
    let entry_end = entry_start + relative_entry_end;
    let entry = &src[entry_start..entry_end];
    assert!(entry.contains(from), "missing pattern '{}'", from);
    let replaced = entry.replacen(from, to, 1);
    let mut out = String::with_capacity(src.len() - entry.len() + replaced.len());
    out.push_str(&src[..entry_start]);
    out.push_str(&replaced);
    out.push_str(&src[entry_end..]);
    fs::write(path, out).expect("write hir file");
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
fn inspect_rejects_malformed_contracts_deterministically() {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let bad_path = std::env::temp_dir().join(format!("kernrift-inspect-malformed-{}.json", ts));
    fs::write(&bad_path, "{}").expect("write malformed contracts");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect")
        .arg("--contracts")
        .arg(bad_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![format!(
            "failed to decode contracts bundle '{}': missing string field 'schema_version'",
            bad_path.display()
        )]
    );

    fs::remove_file(&bad_path).ok();
}

#[test]
fn inspect_contracts_v2_summary_is_stable_and_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "summary");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect")
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "schema: kernrift_contracts_v2",
            "symbols: total=1",
            "contexts:",
            "irq_reachable: 1 [entry]",
            "critical_functions: 0 []",
            "effects:",
            "alloc: 1 [entry]",
            "block: 0 []",
            "yield: 0 []",
            "capabilities:",
            "symbols_with_caps: 1 [entry]",
            "critical_report:",
            "violations: 0",
        ]
    );

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn inspect_contracts_output_is_repeatable() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "repeatable");

    let run_inspect = || {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root)
            .arg("inspect")
            .arg("--contracts")
            .arg(contracts_path.as_os_str());
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    let first = run_inspect();
    let second = run_inspect();
    assert_eq!(first, second, "inspect output must be byte-stable");

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn inspect_report_rejects_malformed_report_deterministically() {
    let root = repo_root();
    let report_path = write_verify_report_fixture("malformed", &json!({}));

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![format!(
            "failed to decode verify report '{}': missing string field 'schema_version'",
            report_path.display()
        )]
    );

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_summary_is_stable_and_exact() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "summary",
        &json!({
            "schema_version": "kernrift_verify_report_v1",
            "result": "deny",
            "inputs": {
                "contracts": "contracts.json",
                "hash": "contracts.sha256",
                "sig": Value::Null,
                "pubkey": Value::Null
            },
            "hash": {
                "expected_sha256": "0000",
                "computed_sha256": "1111",
                "matched": false
            },
            "contracts": {
                "utf8_valid": true,
                "schema_valid": true,
                "schema_version": "kernrift_contracts_v2"
            },
            "signature": {
                "checked": false,
                "valid": Value::Null
            },
            "diagnostics": [
                "verify: HASH_MISMATCH: expected 0000, got 1111"
            ]
        }),
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "schema: kernrift_verify_report_v1",
            "result: deny",
            "inputs:",
            "contracts: contracts.json",
            "hash: contracts.sha256",
            "sig: <none>",
            "pubkey: <none>",
            "hash_status:",
            "matched: false",
            "expected_sha256: 0000",
            "computed_sha256: 1111",
            "contracts_status:",
            "utf8_valid: true",
            "schema_valid: true",
            "schema_version: kernrift_contracts_v2",
            "signature_status:",
            "checked: false",
            "valid: <none>",
            "diagnostics: 1",
            "diagnostic: verify: HASH_MISMATCH: expected 0000, got 1111",
        ]
    );

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_output_is_repeatable() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "repeatable",
        &json!({
            "schema_version": "kernrift_verify_report_v1",
            "result": "pass",
            "inputs": {
                "contracts": "contracts.json",
                "hash": "contracts.sha256",
                "sig": Value::Null,
                "pubkey": Value::Null
            },
            "hash": {
                "expected_sha256": Value::Null,
                "computed_sha256": "abcd",
                "matched": true
            },
            "contracts": {
                "utf8_valid": true,
                "schema_valid": true,
                "schema_version": "kernrift_contracts_v1"
            },
            "signature": {
                "checked": false,
                "valid": Value::Null
            },
            "diagnostics": []
        }),
    );

    let run_inspect = || {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root)
            .arg("inspect-report")
            .arg("--report")
            .arg(report_path.as_os_str());
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    let first = run_inspect();
    let second = run_inspect();
    assert_eq!(first, second, "inspect-report output must be byte-stable");

    fs::remove_file(&report_path).ok();
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
fn check_surface_stable_rejects_irq_handler_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_handler_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec!["surface feature '@irq_handler' requires --surface experimental for 'isr'"]
    );
}

#[test]
fn check_surface_experimental_accepts_irq_handler_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_handler_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_surface_default_matches_stable_for_irq_handler_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_handler_alias.kr");

    let run = |surface: Option<&str>| {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("check");
        if let Some(surface) = surface {
            cmd.arg("--surface").arg(surface);
        }
        cmd.arg(fixture.as_os_str());
        let assert = cmd.assert().failure().code(1);
        String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8")
    };

    let default_stderr = run(None);
    let stable_stderr = run(Some("stable"));
    assert_eq!(default_stderr, stable_stderr);
}

#[test]
fn check_surface_duplicate_flag_is_rejected_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_handler_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("stable")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid check mode: duplicate --surface")
    );
}

#[test]
fn check_surface_invalid_value_is_rejected_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_handler_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("beta")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "invalid check mode: invalid surface mode 'beta', expected 'stable' or 'experimental'"
        )
    );
}

#[test]
fn check_surface_stable_accepts_thread_entry_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("thread_entry_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_surface_experimental_accepts_thread_entry_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("thread_entry_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_surface_stable_rejects_may_block_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("may_block_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec!["surface feature '@may_block' requires --surface experimental for 'worker'"]
    );
}

#[test]
fn check_surface_experimental_accepts_may_block_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("may_block_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_surface_stable_rejects_deprecated_irq_legacy_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_legacy_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "surface feature '@irq_legacy' is deprecated and unavailable under --surface stable for 'legacy_isr'"
        ]
    );
}

#[test]
fn check_surface_experimental_rejects_deprecated_irq_legacy_alias() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("irq_legacy_alias.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "surface feature '@irq_legacy' is deprecated and unavailable under --surface experimental for 'legacy_isr'"
        ]
    );
}

#[test]
fn features_surface_stable_output_is_exact() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("features")
        .arg("--surface")
        .arg("stable");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "features: 1",
            "feature: thread_entry_alias",
            "status: stable",
            "surface_form: @thread_entry",
            "lowering_target: @ctx(thread)",
            "proposal_id: thread_entry_alias",
            "migration_safe: true",
            "canonical_replacement: @ctx(thread)",
            "rewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        ]
    );
}

#[test]
fn features_surface_experimental_output_is_exact() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("features")
        .arg("--surface")
        .arg("experimental");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "features: 3",
            "feature: irq_handler_alias",
            "status: experimental",
            "surface_form: @irq_handler",
            "lowering_target: @ctx(irq)",
            "proposal_id: irq_handler_alias",
            "migration_safe: true",
            "canonical_replacement: @ctx(irq)",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "feature: may_block_alias",
            "status: experimental",
            "surface_form: @may_block",
            "lowering_target: @eff(block)",
            "proposal_id: may_block_alias",
            "migration_safe: true",
            "canonical_replacement: @eff(block)",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "feature: thread_entry_alias",
            "status: stable",
            "surface_form: @thread_entry",
            "lowering_target: @ctx(thread)",
            "proposal_id: thread_entry_alias",
            "migration_safe: true",
            "canonical_replacement: @ctx(thread)",
            "rewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        ]
    );
}

#[test]
fn features_surface_stable_and_experimental_differ_correctly() {
    let root = repo_root();
    let run = |surface: &str| {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root)
            .arg("features")
            .arg("--surface")
            .arg(surface);
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    let stable = run("stable");
    let experimental = run("experimental");
    assert_ne!(stable, experimental);
    assert_eq!(stable.lines().nth(1), Some("features: 1"));
    assert_eq!(experimental.lines().nth(1), Some("features: 3"));
}

#[test]
fn features_surface_duplicate_flag_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("features")
        .arg("--surface")
        .arg("stable")
        .arg("--surface")
        .arg("experimental");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid features mode: duplicate --surface")
    );
}

#[test]
fn features_surface_invalid_value_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("features")
        .arg("--surface")
        .arg("beta");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "invalid features mode: invalid surface mode 'beta', expected 'stable' or 'experimental'"
        )
    );
}

#[test]
fn features_surface_unexpected_arg_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("features")
        .arg("--surface")
        .arg("stable")
        .arg("extra");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid features mode: unexpected argument 'extra'")
    );
}

#[test]
fn proposals_output_is_exact() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("proposals");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "proposals: 4",
            "features: 4",
            "feature: irq_handler_alias",
            "proposal_id: irq_handler_alias",
            "status: experimental",
            "surface_form: @irq_handler",
            "lowering_target: @ctx(irq)",
            "canonical_replacement: @ctx(irq)",
            "feature: irq_legacy_alias",
            "proposal_id: irq_legacy_alias",
            "status: deprecated",
            "surface_form: @irq_legacy",
            "lowering_target: @ctx(irq)",
            "canonical_replacement: @ctx(irq)",
            "feature: may_block_alias",
            "proposal_id: may_block_alias",
            "status: experimental",
            "surface_form: @may_block",
            "lowering_target: @eff(block)",
            "canonical_replacement: @eff(block)",
            "feature: thread_entry_alias",
            "proposal_id: thread_entry_alias",
            "status: stable",
            "surface_form: @thread_entry",
            "lowering_target: @ctx(thread)",
            "canonical_replacement: @ctx(thread)",
        ]
    );
}

#[test]
fn proposals_validate_output_is_exact() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("proposals").arg("--validate");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["proposal-validation: OK"]
    );
}

#[test]
fn proposals_promotion_readiness_output_is_exact() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--promotion-readiness");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "promotion-readiness: 4",
            "feature: irq_handler_alias",
            "current_status: experimental",
            "promotable_to_stable: true",
            "reason: proposal status aligns, migration metadata is complete, proposal linked exactly once",
            "feature: irq_legacy_alias",
            "current_status: deprecated",
            "promotable_to_stable: false",
            "reason: deprecated features are not promotable",
            "feature: may_block_alias",
            "current_status: experimental",
            "promotable_to_stable: true",
            "reason: proposal status aligns, migration metadata is complete, proposal linked exactly once",
            "feature: thread_entry_alias",
            "current_status: stable",
            "promotable_to_stable: false",
            "reason: already stable",
        ]
    );
}

#[test]
fn proposals_duplicate_validate_flag_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--validate")
        .arg("--validate");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: duplicate --validate")
    );
}

#[test]
fn proposals_duplicate_promotion_readiness_flag_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--promotion-readiness")
        .arg("--promotion-readiness");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: duplicate --promotion-readiness")
    );
}

#[test]
fn proposals_promote_irq_handler_alias_updates_statuses_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["proposal-promotion: promoted feature 'irq_handler_alias' to stable"]
    );

    let hir_src = fs::read_to_string(
        repo_dir
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
    )
    .expect("read updated hir");
    let feature_entry = hir_entry_slice(
        &hir_src,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
    );
    assert!(feature_entry.contains("status: AdaptiveFeatureStatus::Stable,"));
    assert!(!feature_entry.contains("status: AdaptiveFeatureStatus::Experimental,"));

    let proposal_entry = hir_entry_slice(
        &hir_src,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
    );
    assert!(proposal_entry.contains("status: AdaptiveFeatureStatus::Stable,"));
    assert!(proposal_entry.contains("title: \"Stable @irq_handler surface alias\","));
    assert!(proposal_entry.contains("compatibility_risk: \"Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.\","));
    assert!(proposal_entry.contains("migration_plan: \"No migration required; the alias is now stable and remains interchangeable with @ctx(irq).\","));
    assert!(!proposal_entry.contains("status: AdaptiveFeatureStatus::Experimental,"));

    let proposal_json = fs::read_to_string(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("read updated proposal");
    assert!(proposal_json.contains("  \"status\": \"stable\""));
    assert!(proposal_json.contains("  \"title\": \"Stable @irq_handler surface alias\""));
    assert!(proposal_json.contains("  \"compatibility_risk\": \"Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.\""));
    assert!(proposal_json.contains("  \"migration_plan\": \"No migration required; the alias is now stable and remains interchangeable with @ctx(irq).\""));
    assert!(!proposal_json.contains("  \"status\": \"experimental\""));
}

#[test]
fn proposals_promote_handles_escaped_hir_proposal_text_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    let json_path = repo_dir
        .join("docs")
        .join("design")
        .join("examples")
        .join("irq_handler_alias.proposal.json");

    let title = "Experimental @irq_handler \\\"quoted\\\" alias";
    let compatibility =
        "Low; alias path C:\\\\irq remains experimental and lowers to \\\"@ctx(irq)\\\".";
    let migration = "Keep experimental for now; rewrite to @ctx(irq) if needed.\\nStill pinned.";

    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
        "title: \"Experimental @irq_handler surface alias\",",
        "title: \"Experimental @irq_handler \\\\\\\"quoted\\\\\\\" alias\",",
    );
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
        "compatibility_risk: \"Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.\",",
        "compatibility_risk: \"Low; alias path C:\\\\\\\\irq remains experimental and lowers to \\\\\\\"@ctx(irq)\\\\\\\".\",",
    );
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
        "migration_plan: \"Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.\",",
        "migration_plan: \"Keep experimental for now; rewrite to @ctx(irq) if needed.\\\\nStill pinned.\",",
    );
    replace_json_string_field(&json_path, "title", title);
    replace_json_string_field(&json_path, "compatibility_risk", compatibility);
    replace_json_string_field(&json_path, "migration_plan", migration);
    git_commit_all(&repo_dir, "escaped proposal text");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["proposal-promotion: promoted feature 'irq_handler_alias' to stable"]
    );

    let hir_src = fs::read_to_string(&hir_path).expect("read updated escaped hir");
    let proposal_entry = hir_entry_slice(
        &hir_src,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
    );
    assert!(proposal_entry.contains("status: AdaptiveFeatureStatus::Stable,"));
    assert!(proposal_entry.contains("title: \"Stable @irq_handler surface alias\","));
    assert!(proposal_entry.contains("compatibility_risk: \"Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.\","));
    assert!(proposal_entry.contains("migration_plan: \"No migration required; the alias is now stable and remains interchangeable with @ctx(irq).\","));

    let proposal_json = fs::read_to_string(&json_path).expect("read updated escaped json");
    assert!(proposal_json.contains("\"status\": \"stable\""));
    assert!(proposal_json.contains("\"title\": \"Stable @irq_handler surface alias\""));
    assert!(proposal_json.contains("\"compatibility_risk\": \"Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.\""));
    assert!(proposal_json.contains("\"migration_plan\": \"No migration required; the alias is now stable and remains interchangeable with @ctx(irq).\""));
}

#[test]
fn proposals_promote_dry_run_is_deterministic_and_non_mutating() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let before_hir = fs::read_to_string(
        repo_dir
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
    )
    .expect("read baseline hir");
    let before_proposal = fs::read_to_string(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("read baseline proposal");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--dry-run");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["proposal-promotion: dry-run promotion for feature 'irq_handler_alias' is valid"]
    );

    let after_hir = fs::read_to_string(
        repo_dir
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
    )
    .expect("read dry-run hir");
    let after_proposal = fs::read_to_string(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("read dry-run proposal");
    assert_eq!(after_hir, before_hir);
    assert_eq!(after_proposal, before_proposal);
}

#[test]
fn proposals_promote_diff_output_is_exact() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--diff");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "promotion-diff: 9",
            "feature: irq_handler_alias",
            "proposal_id: irq_handler_alias",
            "file: crates/hir/src/lib.rs",
            "field: feature.status",
            "before: experimental",
            "after: stable",
            "file: crates/hir/src/lib.rs",
            "field: proposal.compatibility_risk",
            "before: Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
            "after: Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
            "file: crates/hir/src/lib.rs",
            "field: proposal.migration_plan",
            "before: Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
            "after: No migration required; the alias is now stable and remains interchangeable with @ctx(irq).",
            "file: crates/hir/src/lib.rs",
            "field: proposal.status",
            "before: experimental",
            "after: stable",
            "file: crates/hir/src/lib.rs",
            "field: proposal.title",
            "before: Experimental @irq_handler surface alias",
            "after: Stable @irq_handler surface alias",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.compatibility_risk",
            "before: Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
            "after: Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.migration_plan",
            "before: Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
            "after: No migration required; the alias is now stable and remains interchangeable with @ctx(irq).",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.status",
            "before: experimental",
            "after: stable",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.title",
            "before: Experimental @irq_handler surface alias",
            "after: Stable @irq_handler surface alias",
            "proposal-promotion: promoted feature 'irq_handler_alias' to stable",
        ]
    );
}

#[test]
fn proposals_promote_dry_run_diff_output_is_exact_and_non_mutating() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let before_hir = fs::read_to_string(
        repo_dir
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
    )
    .expect("read baseline hir");
    let before_proposal = fs::read_to_string(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("read baseline proposal");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--dry-run")
        .arg("--diff");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "promotion-diff: 9",
            "feature: irq_handler_alias",
            "proposal_id: irq_handler_alias",
            "file: crates/hir/src/lib.rs",
            "field: feature.status",
            "before: experimental",
            "after: stable",
            "file: crates/hir/src/lib.rs",
            "field: proposal.compatibility_risk",
            "before: Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
            "after: Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
            "file: crates/hir/src/lib.rs",
            "field: proposal.migration_plan",
            "before: Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
            "after: No migration required; the alias is now stable and remains interchangeable with @ctx(irq).",
            "file: crates/hir/src/lib.rs",
            "field: proposal.status",
            "before: experimental",
            "after: stable",
            "file: crates/hir/src/lib.rs",
            "field: proposal.title",
            "before: Experimental @irq_handler surface alias",
            "after: Stable @irq_handler surface alias",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.compatibility_risk",
            "before: Low; stable mode rejects the alias and experimental mode lowers to existing canonical semantics.",
            "after: Low; the alias is stable and lowers to existing canonical semantics in all supported surface profiles.",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.migration_plan",
            "before: Keep the alias experimental until usage and diagnostics stabilize; projects can stay pinned to stable to avoid it.",
            "after: No migration required; the alias is now stable and remains interchangeable with @ctx(irq).",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.status",
            "before: experimental",
            "after: stable",
            "file: docs/design/examples/irq_handler_alias.proposal.json",
            "field: proposal.title",
            "before: Experimental @irq_handler surface alias",
            "after: Stable @irq_handler surface alias",
            "proposal-promotion: dry-run promotion for feature 'irq_handler_alias' is valid",
        ]
    );

    let after_hir = fs::read_to_string(
        repo_dir
            .join("crates")
            .join("hir")
            .join("src")
            .join("lib.rs"),
    )
    .expect("read dry-run diff hir");
    let after_proposal = fs::read_to_string(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("read dry-run diff proposal");
    assert_eq!(after_hir, before_hir);
    assert_eq!(after_proposal, before_proposal);
}

#[test]
fn proposals_promote_duplicate_flag_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--promote")
        .arg("may_block_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: duplicate --promote")
    );
}

#[test]
fn proposals_promote_duplicate_diff_flag_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--diff")
        .arg("--diff");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: duplicate --diff")
    );
}

#[test]
fn proposals_diff_without_promote_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("proposals").arg("--diff");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: --diff requires --promote <feature-id>")
    );
}

#[test]
fn proposals_promote_stable_feature_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("thread_entry_alias");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("thread_entry_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: feature 'thread_entry_alias' is not promotable: already stable")
    );
}

#[test]
fn proposals_promote_unknown_feature_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("unknown_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: unknown feature 'unknown_alias'")
    );
}

#[test]
fn proposals_promote_duplicate_dry_run_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--dry-run")
        .arg("--dry-run");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: duplicate --dry-run")
    );
}

#[test]
fn proposals_promote_dirty_worktree_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    fs::write(repo_dir.join("DIRTY.txt"), "dirty\n").expect("write dirty file");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: repository worktree is not clean")
    );
}

#[test]
fn proposals_promote_invalid_repo_root_is_rejected_deterministically() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let repo_dir = std::env::temp_dir().join(format!("kernrift-invalid-promotion-root-{}", ts));
    fs::create_dir_all(&repo_dir).expect("create temp dir");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: current directory is not a KernRift repo root")
    );
}

#[test]
fn proposals_promote_missing_feature_entry_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "id: \"irq_handler_alias\",",
        "id: \"irq_handler_alias_missing\",",
    );
    git_commit_all(&repo_dir, "remove feature entry identity");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: target repo missing feature 'irq_handler_alias'")
    );
}

#[test]
fn proposals_promote_missing_hir_proposal_entry_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "proposal_id: \"irq_handler_alias\",",
        "proposal_id: \"missing_alias\",",
    );
    git_commit_all(&repo_dir, "remove linked proposal entry");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: target repo missing proposal 'missing_alias'")
    );
}

#[test]
fn proposals_promote_missing_json_proposal_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    fs::remove_file(
        repo_dir
            .join("docs")
            .join("design")
            .join("examples")
            .join("irq_handler_alias.proposal.json"),
    )
    .expect("remove proposal json");
    git_commit_all(&repo_dir, "remove json proposal");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("proposal-promotion: target repo missing proposal 'irq_handler_alias'")
    );
}

#[test]
fn proposals_promote_proposal_linkage_drift_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "proposal_id: \"irq_handler_alias\",",
        "proposal_id: \"may_block_alias\",",
    );
    git_commit_all(&repo_dir, "drift proposal linkage");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo feature 'irq_handler_alias' proposal linkage mismatch"
        )
    );
}

#[test]
fn proposals_promote_canonical_replacement_drift_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "canonical_replacement: \"@ctx(irq)\",",
        "canonical_replacement: \"@ctx(thread)\",",
    );
    git_commit_all(&repo_dir, "drift canonical replacement");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo feature 'irq_handler_alias' canonical replacement mismatch"
        )
    );
}

#[test]
fn proposals_promote_feature_status_drift_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "status: AdaptiveFeatureStatus::Experimental,",
        "status: AdaptiveFeatureStatus::Stable,",
    );
    git_commit_all(&repo_dir, "drift feature status");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: binary/repo disagreement for feature 'irq_handler_alias' current status"
        )
    );
}

#[test]
fn proposals_promote_hir_proposal_status_drift_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_FEATURE_PROPOSALS:",
        "irq_handler_alias",
        "status: AdaptiveFeatureStatus::Experimental,",
        "status: AdaptiveFeatureStatus::Stable,",
    );
    git_commit_all(&repo_dir, "drift proposal status in hir");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo proposal 'irq_handler_alias' status mismatch between HIR and JSON"
        )
    );
}

#[test]
fn proposals_promote_json_proposal_status_drift_is_rejected_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let json_path = repo_dir
        .join("docs")
        .join("design")
        .join("examples")
        .join("irq_handler_alias.proposal.json");
    replace_once_in_file(
        &json_path,
        "\"status\": \"experimental\"",
        "\"status\": \"stable\"",
    );
    git_commit_all(&repo_dir, "drift proposal status in json");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo proposal 'irq_handler_alias' status mismatch between HIR and JSON"
        )
    );
}

#[test]
fn proposals_promote_dry_run_fails_on_repo_drift_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "canonical_replacement: \"@ctx(irq)\",",
        "canonical_replacement: \"@ctx(thread)\",",
    );
    git_commit_all(&repo_dir, "drift canonical replacement for dry run");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--dry-run");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo feature 'irq_handler_alias' canonical replacement mismatch"
        )
    );
}

#[test]
fn proposals_promote_diff_fails_on_repo_drift_before_preview_deterministically() {
    let repo_dir = write_promotion_repo_fixture("irq_handler_alias");
    let hir_path = repo_dir
        .join("crates")
        .join("hir")
        .join("src")
        .join("lib.rs");
    replace_in_hir_entry(
        &hir_path,
        "const ADAPTIVE_SURFACE_FEATURES:",
        "irq_handler_alias",
        "canonical_replacement: \"@ctx(irq)\",",
        "canonical_replacement: \"@ctx(thread)\",",
    );
    git_commit_all(&repo_dir, "drift canonical replacement for diff");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&repo_dir)
        .arg("proposals")
        .arg("--promote")
        .arg("irq_handler_alias")
        .arg("--diff");
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stdout.trim().is_empty());
    assert_eq!(
        stderr.lines().next(),
        Some(
            "proposal-promotion: target repo feature 'irq_handler_alias' canonical replacement mismatch"
        )
    );
}

#[test]
fn proposals_unexpected_arg_is_rejected_deterministically() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("proposals").arg("extra");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid proposals mode: unexpected argument 'extra'")
    );
}

#[test]
fn migrate_preview_surface_stable_output_is_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "migrations: 4",
            "function: blocker",
            "surface_form: @may_block",
            "feature: may_block_alias",
            "status: experimental",
            "enabled_under_surface: false",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "function: isr",
            "surface_form: @irq_handler",
            "feature: irq_handler_alias",
            "status: experimental",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "function: legacy_isr",
            "surface_form: @irq_legacy",
            "feature: irq_legacy_alias",
            "status: deprecated",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
            "function: worker",
            "surface_form: @thread_entry",
            "feature: thread_entry_alias",
            "status: stable",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(thread)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        ]
    );
}

#[test]
fn migrate_preview_surface_experimental_output_is_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "migrations: 4",
            "function: blocker",
            "surface_form: @may_block",
            "feature: may_block_alias",
            "status: experimental",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "function: isr",
            "surface_form: @irq_handler",
            "feature: irq_handler_alias",
            "status: experimental",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "function: legacy_isr",
            "surface_form: @irq_legacy",
            "feature: irq_legacy_alias",
            "status: deprecated",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
            "function: worker",
            "surface_form: @thread_entry",
            "feature: thread_entry_alias",
            "status: stable",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(thread)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        ]
    );
}

#[test]
fn migrate_preview_surface_duplicate_flag_is_rejected_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--surface")
        .arg("stable")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: duplicate --surface")
    );
}

#[test]
fn migrate_preview_surface_invalid_value_is_rejected_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--surface")
        .arg("beta")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "invalid migrate-preview mode: invalid surface mode 'beta', expected 'stable' or 'experimental'"
        )
    );
}

#[test]
fn migrate_preview_surface_unexpected_arg_is_rejected_deterministically() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str())
        .arg("extra");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: unexpected argument 'extra'")
    );
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
fn check_rejects_irq_block_effect_boundary_direct() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_block_site.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "ctx-check: CTX_IRQ_BLOCK_BOUNDARY: function 'isr_block' is @ctx(irq) and uses block effect (direct=true, via_callee=[], via_extern=[])"
        ]
    );
}

#[test]
fn check_rejects_irq_block_effect_boundary_transitive() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("irq_block_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "ctx-check: CTX_IRQ_BLOCK_BOUNDARY: function 'helper' is @ctx(irq) and uses block effect (direct=true, via_callee=[], via_extern=[])",
            "ctx-check: CTX_IRQ_BLOCK_BOUNDARY: function 'isr' is @ctx(irq) and uses block effect (direct=false, via_callee=[helper], via_extern=[])"
        ]
    );
}

#[test]
fn check_rejects_critical_alloc_boundary_direct() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("critical_alloc_direct.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "critical-region: CRITICAL_ALLOC_BOUNDARY: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])"
        ]
    );
}

#[test]
fn check_rejects_critical_alloc_boundary_transitive() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("critical_alloc_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "critical-region: CRITICAL_ALLOC_BOUNDARY: function 'entry' uses alloc effect in critical region (direct=false, via_callee=[helper], via_extern=[])"
        ]
    );
}

#[test]
fn check_allows_alloc_outside_critical() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("alloc_outside_critical.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_rejects_critical_block_boundary_direct() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("critical_block_direct.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "critical-region: CRITICAL_BLOCK_BOUNDARY: function 'entry' uses block effect in critical region (direct=true, via_callee=[], via_extern=[])"
        ]
    );
}

#[test]
fn check_rejects_critical_block_boundary_transitive() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("critical_block_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "critical-region: CRITICAL_BLOCK_BOUNDARY: function 'entry' uses block effect in critical region (direct=false, via_callee=[helper], via_extern=[])"
        ]
    );
}

#[test]
fn check_allows_block_outside_critical() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("block_outside_critical.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_rejects_capability_boundary_direct() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("capability_boundary_direct.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "cap-check: CAPABILITY_BOUNDARY: function 'entry' reaches capability 'PhysMap' without declaring @caps(PhysMap) (direct=false, via_callee=[helper], via_extern=[])"
        ]
    );
}

#[test]
fn check_rejects_capability_boundary_transitive() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("capability_boundary_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "cap-check: CAPABILITY_BOUNDARY: function 'entry' reaches capability 'PhysMap' without declaring @caps(PhysMap) (direct=false, via_callee=[helper, mid], via_extern=[])",
            "cap-check: CAPABILITY_BOUNDARY: function 'mid' reaches capability 'PhysMap' without declaring @caps(PhysMap) (direct=false, via_callee=[helper], via_extern=[])",
        ]
    );
}

#[test]
fn check_allows_capability_boundary_when_declared() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("capability_boundary_declared.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_missing_module_cap_behavior_is_unchanged() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_fail").join("missing_cap.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "cap-check: call 'caller -> mapit' violates caps_avail=module_caps, missing: PhysMap",
            "cap-check: function 'mapit' requires unavailable caps: PhysMap",
        ]
    );
}

#[test]
fn check_allows_block_effect_outside_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("blockpoint_thread.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_alloc_in_irq_behavior_is_unchanged() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_fail").join("alloc_in_irq.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec!["effect-check: call 'isr -> allocy' in ctx 'irq' uses forbidden effects: alloc"]
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
        .join("must_pass")
        .join("blockpoint_thread.kr");

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
                "direct": true,
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
                "direct": true,
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
                "direct": true,
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
fn policy_kernel_forbid_yield_in_irq_is_artifact_driven_and_deterministic() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-yield-irq-src-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-yield-irq-{}.toml", ts));
    let mutated_path =
        std::env::temp_dir().join(format!("kernrift-policy-yield-irq-mut-{}.json", ts));
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
forbid_yield_in_irq = true
"#,
    )
    .expect("write policy");

    let mut pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    pass_cmd.assert().success();

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    let symbols = contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array");
    assert!(
        symbols.len() >= 2,
        "expected at least two symbols in contracts"
    );
    symbols[0]["name"] = json!("helper");
    symbols[0]["ctx_reachable"] = json!(["irq"]);
    symbols[0]["eff_transitive"] = json!(["yield"]);
    symbols[0]["eff_provenance"] = json!([{
        "effect": "yield",
        "provenance": {
            "direct": true,
            "via_callee": [],
            "via_extern": []
        }
    }]);
    symbols[1]["name"] = json!("isr");
    symbols[1]["ctx_reachable"] = json!(["irq"]);
    symbols[1]["eff_transitive"] = json!(["yield"]);
    symbols[1]["eff_provenance"] = json!([{
        "effect": "yield",
        "provenance": {
            "direct": false,
            "via_callee": ["helper"],
            "via_extern": []
        }
    }]);
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut fail_cmd: Command = cargo_bin_cmd!("kernriftc");
    fail_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    let fail_assert = fail_cmd.assert().failure().code(1);
    let fail_stderr = String::from_utf8(fail_assert.get_output().stderr.clone()).expect("stderr");
    let lines = fail_stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_YIELD: function 'helper' is irq-reachable and uses yield effect (direct=true, via_callee=[], via_extern=[])",
            "policy: KERNEL_IRQ_YIELD: function 'isr' is irq-reachable and uses yield effect (direct=false, via_callee=[helper], via_extern=[])",
        ],
        "expected deterministic irq yield violations, got:\n{}",
        fail_stderr
    );

    for symbol in contracts_json["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols array")
    {
        symbol["eff_transitive"] = json!([]);
        symbol["eff_provenance"] = json!([]);
    }
    fs::write(
        &mutated_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    let mut clear_pass_cmd: Command = cargo_bin_cmd!("kernriftc");
    clear_pass_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(mutated_path.as_os_str());
    clear_pass_cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
    fs::remove_file(&mutated_path).ok();
}

#[test]
fn contracts_v2_semantic_fields_coexist_and_validate_schema() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
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
    assert_eq!(json["report"]["critical"]["violations"], json!([]));

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
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[])",
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
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[map_io])",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'map_io' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
        ],
        "expected deterministic extern capability propagation violations, got:\n{}",
        stderr
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_without_evidence_keeps_output_exactly_unchanged() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-no-evidence-{}.json", ts));
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-no-evidence-{}.toml", ts));
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
forbid_alloc_in_irq = true
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
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_effect_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-effect-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-effect-{}.toml", ts));
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
forbid_alloc_in_irq = true
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_capability_is_deterministic() {
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
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-cap-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-irq-cap-{}.toml", ts));
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
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_CAP_FORBID: function 'helper' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=helper",
            "evidence: capability=PhysMap",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: KERNEL_IRQ_CAP_FORBID: function 'isr' is irq-reachable and uses forbidden capability 'PhysMap' (direct=true, via_callee=[helper], via_extern=[])",
            "evidence: symbol=isr",
            "evidence: capability=PhysMap",
            "evidence: direct=true",
            "evidence: via_callee=[helper]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_critical_region_is_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-critical-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-critical-{}.toml", ts));
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

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_effects_in_critical = ["alloc"]
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "evidence: function=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_blocks_follow_deterministic_violation_order() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("policy_families_order_no_critical_alloc.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-order-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-evidence-order-{}.toml", ts));
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

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

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
"#,
    )
    .expect("write policy");

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = policy_cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_CRITICAL_REGION_ALLOC: function 'entry' uses alloc effect in critical region (direct=true, via_callee=[], via_extern=[])",
            "evidence: function=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: KERNEL_IRQ_ALLOC: function 'entry' is irq-reachable and uses alloc effect (direct=true, via_callee=[], via_extern=[])",
            "evidence: symbol=entry",
            "evidence: effect=alloc",
            "evidence: direct=true",
            "evidence: via_callee=[]",
            "evidence: via_extern=[]",
            "policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1",
            "policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present",
        ]
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
fn policy_kernel_irq_yield_rule_requires_contracts_v2() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-policy-v1-irq-yield-{}.json", ts));
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-policy-v1-irq-yield-{}.toml", ts));
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
forbid_yield_in_irq = true
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
        .join("policy_families_order_no_critical_alloc.kr");
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

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

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
        .join("policy_families_order_no_critical_alloc.kr");
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

    let mut contracts_json: Value =
        serde_json::from_str(&fs::read_to_string(&contracts_path).expect("contracts text"))
            .expect("contracts json");
    inject_single_critical_violation(&mut contracts_json, "entry", "alloc", true, &[], &[]);
    fs::write(
        &contracts_path,
        serde_json::to_string(&contracts_json).expect("contracts json text"),
    )
    .expect("write mutated contracts");

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
