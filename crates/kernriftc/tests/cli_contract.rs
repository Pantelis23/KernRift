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
const POLICY_VIOLATIONS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_policy_violations_v1.schema.json");
const CANONICAL_FINDINGS_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_findings_v1.schema.json");
const CANONICAL_EDIT_PLAN_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_edit_plan_v1.schema.json");
const CANONICAL_FIX_RESULT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_fix_result_v1.schema.json");
const CANONICAL_FIX_PREVIEW_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_canonical_fix_preview_v1.schema.json");
const VERIFY_REPORT_SCHEMA_V1: &str =
    include_str!("../../../docs/schemas/kernrift_verify_report_v1.schema.json");
const ADAPTIVE_OS_CONTEXT_TEXT: &str = include_str!("../../../docs/ADAPTIVE_OS_CONTEXT.md");
const ARCHITECTURE_DOC_TEXT: &str = include_str!("../../../docs/ARCHITECTURE.md");
const KERNEL_PROFILE_NOTES_TEXT: &str =
    include_str!("../../../docs/design/kernel_profile_pr1_notes.md");
const KR0_KR3_PLAN_TEXT: &str = include_str!("../../../docs/KR0_KR3_PLAN.md");
const KR0_AUTHORING_REFERENCE_TEXT: &str =
    include_str!("../../../docs/spec/kr0-canonical-authoring-reference.md");
const KRIR_SPEC_TEXT: &str = include_str!("../../../docs/spec/krir-v0.1.md");
const KERNEL_PROFILE_SPEC_TEXT: &str = include_str!("../../../docs/spec/kernel_profile.md");
const README_TEXT: &str = include_str!("../../../README.md");
const FULL_SERIAL_WRAPPER_TEXT: &str = include_str!("../../../tools/validation/full_serial.sh");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

// Contributor lock for future JSON-capable commands: reuse this helper from
// cli_contract coverage instead of creating command-specific transport rules.
fn assert_json_transport(stdout: &str, stderr: &str, schema_version: &str) {
    assert!(
        stderr.is_empty(),
        "json mode must not write stderr: {}",
        stderr
    );
    assert!(
        stdout.ends_with('\n'),
        "json mode output must end with trailing newline: {:?}",
        stdout
    );
    let json: Value = serde_json::from_str(stdout).expect("json stdout");
    assert_eq!(
        json["schema_version"],
        json!(schema_version),
        "json mode must include stable schema_version"
    );
}

#[test]
fn structured_output_conventions_spec_locks_future_json_command_transport_tests() {
    assert!(
        KRIR_SPEC_TEXT
            .contains("new JSON-capable commands must add `cli_contract` transport assertions"),
        "structured output conventions spec must require cli_contract transport assertions"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("prefer reusing `assert_json_transport`"),
        "structured output conventions spec must point contributors at assert_json_transport"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("stdout` only") || KRIR_SPEC_TEXT.contains("`stdout` only"),
        "structured output conventions spec must lock stdout-only JSON transport"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("empty `stderr`"),
        "structured output conventions spec must lock empty stderr in JSON mode"
    );
    assert!(
        KRIR_SPEC_TEXT.contains("trailing\n  newline termination")
            || KRIR_SPEC_TEXT.contains("trailing newline"),
        "structured output conventions spec must lock trailing-newline JSON termination"
    );
}

#[test]
fn structured_output_command_matrix_spec_lists_current_json_capable_commands() {
    for surface in [
        "kernriftc inspect-artifact <artifact> --format json",
        "kernriftc verify-artifact-meta --format json <artifact> <meta.json>",
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>",
        "kernriftc check --format json --policy <policy.toml> <file.kr>",
        "kernriftc check --canonical --format json <file.kr>",
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>",
        "kernriftc fix --canonical --write --format json <file.kr>",
        "kernriftc fix --canonical --dry-run --format json <file.kr>",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(surface),
            "structured output command matrix must list '{}'",
            surface
        );
    }
    for schema in [
        "kernrift_inspect_artifact_v1",
        "kernrift_verify_artifact_meta_v1",
        "kernrift_policy_violations_v1",
        "kernrift_canonical_findings_v1",
        "kernrift_canonical_edit_plan_v1",
        "kernrift_canonical_fix_result_v1",
        "kernrift_canonical_fix_preview_v1",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(schema),
            "structured output command matrix must mention schema '{}'",
            schema
        );
    }
}

#[test]
fn structured_output_test_coverage_matrix_spec_lists_current_json_capable_commands() {
    assert!(
        KRIR_SPEC_TEXT.contains("### Structured Output Test Coverage Matrix"),
        "structured output spec must include the test coverage matrix section"
    );
    for surface in [
        "kernriftc inspect-artifact <artifact> --format json",
        "kernriftc verify-artifact-meta --format json <artifact> <meta.json>",
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>",
        "kernriftc check --format json --policy <policy.toml> <file.kr>",
        "kernriftc check --canonical --format json <file.kr>",
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>",
        "kernriftc fix --canonical --write --format json <file.kr>",
        "kernriftc fix --canonical --dry-run --format json <file.kr>",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(surface),
            "structured output test coverage matrix must list '{}'",
            surface
        );
    }
}

#[test]
fn structured_output_new_json_command_checklist_is_present() {
    assert!(
        KRIR_SPEC_TEXT.contains("### New JSON Command Checklist"),
        "structured output spec must include the new JSON command checklist"
    );
    for line in [
        "document the command surface in the structured-output command matrix",
        "document its coverage in the structured-output test coverage matrix",
        "add `cli_contract` transport assertions",
        "reuse `assert_json_transport` where applicable",
        "add or reference a schema when the payload is versioned",
        "preserve `stdout`-only, empty-`stderr`, trailing-newline transport behavior",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(line),
            "structured output checklist must include '{}'",
            line
        );
    }
}

#[test]
fn readme_points_to_full_serial_low_memory_validation_wrapper() {
    assert!(
        README_TEXT.contains("bash tools/validation/full_serial.sh"),
        "README must point contributors at the repo-owned full serialized validation wrapper"
    );
    assert!(
        README_TEXT.contains("per-crate serialized test steps")
            && README_TEXT.contains("32 GB class machines"),
        "README must describe the full_serial wrapper as the safer local low-memory path"
    );
}

#[test]
fn full_serial_wrapper_avoids_workspace_test_aggregation() {
    assert!(
        !FULL_SERIAL_WRAPPER_TEXT.contains("cargo test --workspace -- --test-threads=1"),
        "full_serial wrapper must not use the workspace-wide test aggregation step locally"
    );
    for step in [
        "run_step cargo test -p parser -- --test-threads=1",
        "run_step cargo test -p hir -- --test-threads=1",
        "run_step cargo test -p krir -- --test-threads=1",
        "run_step cargo test -p kernriftc --test cli_contract -- --test-threads=1",
        "run_step cargo test -p kernriftc --test golden -- --test-threads=1",
        "run_step cargo test -p kernriftc -- --test-threads=1",
    ] {
        assert!(
            FULL_SERIAL_WRAPPER_TEXT.contains(step),
            "full_serial wrapper must include '{}'",
            step
        );
    }
}

#[test]
fn kr0_frontend_spec_declares_canonical_spellings_and_alias_policy() {
    assert!(
        KRIR_SPEC_TEXT.contains("### Canonical KR0 Frontend Spellings"),
        "kr0 frontend spec must declare a canonical spelling section"
    );
    for spelling in [
        "`@ctx(...)`",
        "`@eff(...)`",
        "`@caps(...)`",
        "`@module_caps(...)`",
        "`critical { ... }`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(spelling),
            "kr0 frontend spec must list canonical spelling '{}'",
            spelling
        );
    }
    for alias in [
        "`@thread_entry`",
        "`@irq_handler`",
        "`@may_block`",
        "`@irq_legacy`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(alias),
            "kr0 frontend spec must classify compatibility alias '{}'",
            alias
        );
    }
    assert!(
        KRIR_SPEC_TEXT.contains("Compatibility fixtures under `tests/living_compiler/*alias*.kr`"),
        "kr0 frontend spec must explain that alias fixtures are compatibility locks"
    );
    assert!(
        KRIR_SPEC_TEXT.contains(
            "classify non-canonical spellings as `compatibility aliases` or `deprecated aliases`"
        ),
        "kr0 frontend spec must describe alias classification guidance"
    );
    for legacy in [
        "`@irq` -> `@ctx(irq)`",
        "`@noirq` -> `@ctx(thread, boot)`",
        "`@alloc` -> `@eff(alloc)`",
        "`@block` -> `@eff(block)`",
        "`@preempt_off` -> `@eff(preempt_off)`",
        "`@yieldpoint` -> `yieldpoint()`",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(legacy),
            "kr0 frontend spec must document legacy-to-canonical mapping '{}'",
            legacy
        );
    }
}

#[test]
fn kr0_authoring_reference_covers_canonical_templates() {
    assert!(
        KRIR_SPEC_TEXT.contains("docs/spec/kr0-canonical-authoring-reference.md"),
        "krir spec must point readers at the canonical KR0 authoring reference"
    );
    for section in [
        "## Canonical Function Forms",
        "## Canonical Extern Form",
        "## Canonical Context, Effect, and Capability Facts",
        "## Canonical MMIO Declaration and Use",
        "## Canonical Critical and Yield Usage",
        "## Common Mistakes -> Canonical Replacement",
        "## Alias Fixture Note",
    ] {
        assert!(
            KR0_AUTHORING_REFERENCE_TEXT.contains(section),
            "kr0 authoring reference must contain '{}'",
            section
        );
    }
    for template in [
        "fn entry() { }",
        "extern @ctx(thread, boot) @eff(block) @caps() fn sleep();",
        "@module_caps(PhysMap);",
        "@ctx(thread, boot,)",
        "@module_caps(PhysMap,)",
        "kernriftc check --canonical <file.kr>",
        "mmio UART0 = 0x1000;",
        "mmio_reg UART0.DR = 0x00 : u32 rw;",
        "mmio_read<u32>(UART0 + 0x04);",
        "raw_mmio_write<u32>(0x1014, x);",
        "critical {",
        "yieldpoint();",
        "| `@irq` | `@ctx(irq)` |",
        "| `@noirq` | `@ctx(thread, boot)` |",
        "| `@alloc` | `@eff(alloc)` |",
        "| `@block` | `@eff(block)` |",
        "| `@preempt_off` | `@eff(preempt_off)` |",
    ] {
        assert!(
            KR0_AUTHORING_REFERENCE_TEXT.contains(template),
            "kr0 authoring reference must contain template '{}'",
            template
        );
    }
    assert!(
        KR0_AUTHORING_REFERENCE_TEXT.contains("tests/living_compiler/*alias*.kr"),
        "kr0 authoring reference must explain that alias fixtures are compatibility locks"
    );
}

#[test]
fn kr0_spec_and_authoring_reference_document_trailing_comma_fact_lists() {
    assert!(
        KRIR_SPEC_TEXT.contains("CsvIdent    := Ident { \",\" Ident } [ \",\" ]"),
        "krir spec must document optional trailing commas in canonical fact lists"
    );
    for example in [
        "@ctx(thread, boot,)",
        "@eff(block,)",
        "@caps(PhysMap,)",
        "@module_caps(PhysMap,)",
    ] {
        assert!(
            KRIR_SPEC_TEXT.contains(example) || KR0_AUTHORING_REFERENCE_TEXT.contains(example),
            "docs must mention trailing-comma example '{}'",
            example
        );
    }
}

#[test]
fn kr0_general_teaching_docs_prefer_canonical_frontend_surface() {
    let teaching_docs = [
        ARCHITECTURE_DOC_TEXT,
        KR0_KR3_PLAN_TEXT,
        ADAPTIVE_OS_CONTEXT_TEXT,
        KERNEL_PROFILE_SPEC_TEXT,
        KERNEL_PROFILE_NOTES_TEXT,
    ];

    for legacy in [
        "@irq",
        "@noirq",
        "@alloc",
        "@block",
        "@preempt_off",
        "@yieldpoint",
        "mmio<T>",
        "volatile_load",
        "volatile_store",
    ] {
        for doc in teaching_docs {
            assert!(
                !doc.contains(legacy),
                "general teaching docs must not drift back to legacy/non-canonical surface '{}'",
                legacy
            );
        }
    }

    assert!(
        ARCHITECTURE_DOC_TEXT.contains("@ctx(...)")
            && ARCHITECTURE_DOC_TEXT.contains("@eff(...)")
            && ARCHITECTURE_DOC_TEXT.contains("@caps(...)"),
        "architecture docs must teach canonical fact spellings"
    );
    assert!(
        KR0_KR3_PLAN_TEXT.contains("yieldpoint()"),
        "KR0/KR3 plan must teach the canonical yieldpoint() form"
    );
    assert!(
        ADAPTIVE_OS_CONTEXT_TEXT.contains("mmio_read<T>(addr)")
            && ADAPTIVE_OS_CONTEXT_TEXT.contains("mmio_write<T>(addr, value)"),
        "adaptive OS context doc must teach current typed MMIO call forms"
    );
}

fn object_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("json object")
        .keys()
        .cloned()
        .collect()
}

fn unique_temp_output_path(label: &str, ext: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("kernrift-{}-{}.{}", label, ts, ext))
}

fn copy_fixture_to_temp(label: &str, fixture: &Path) -> PathBuf {
    let temp_path = unique_temp_output_path(label, "kr");
    fs::copy(fixture, &temp_path).expect("copy fixture to temp");
    temp_path
}

fn write_temp_source_fixture(label: &str, src: &str) -> PathBuf {
    let temp_path = unique_temp_output_path(label, "kr");
    fs::write(&temp_path, src).expect("write temp source fixture");
    temp_path
}

fn emit_backend_artifact_with_sidecar(
    root: &Path,
    kind: &str,
    fixture: &Path,
    artifact_path: &Path,
    meta_path: &Path,
    explicit_stable: bool,
) {
    fs::remove_file(artifact_path).ok();
    fs::remove_file(meta_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root);
    if explicit_stable {
        cmd.arg("--surface").arg("stable");
    }
    cmd.arg(format!("--emit={kind}"))
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

fn emit_backend_artifact(
    root: &Path,
    kind: &str,
    fixture: &Path,
    artifact_path: &Path,
    explicit_stable: bool,
) {
    fs::remove_file(artifact_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root);
    if explicit_stable {
        cmd.arg("--surface").arg("stable");
    }
    cmd.arg(format!("--emit={kind}"))
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

fn inspect_artifact_output(root: &Path, artifact_path: &Path, format: Option<&str>) -> String {
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(root)
        .arg("inspect-artifact")
        .arg(artifact_path.as_os_str());
    if let Some(format) = format {
        cmd.arg("--format").arg(format);
    }
    let assert = cmd.assert().success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
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

fn write_temp_policy_file(label: &str, policy_text: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-policy-{}-{}.toml", label, ts));
    fs::remove_file(&policy_path).ok();
    fs::write(&policy_path, policy_text).expect("write policy");
    policy_path
}

fn write_temp_contracts_file(label: &str, contracts: &Value) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path =
        std::env::temp_dir().join(format!("kernrift-contracts-{}-{}.json", label, ts));
    fs::remove_file(&contracts_path).ok();
    fs::write(
        &contracts_path,
        serde_json::to_vec_pretty(contracts).expect("serialize contracts"),
    )
    .expect("write contracts");
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
fn usage_includes_artifact_json_consumer_commands() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root);
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");

    assert!(stderr.contains("kernriftc inspect-artifact <artifact-path> --format json"));
    assert!(stderr.contains("kernriftc verify-artifact-meta --format json <artifact> <meta.json>"));
    assert!(stderr.contains(
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>"
    ));
    assert!(stderr.contains("kernriftc check --format json --policy <policy.toml> <file.kr>"));
    assert!(stderr.contains("kernriftc check --canonical <file.kr>"));
    assert!(stderr.contains("kernriftc check --canonical --format json <file.kr>"));
    assert!(stderr.contains(
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>"
    ));
    assert!(stderr.contains("kernriftc fix --canonical --write <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --write --format json <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run --format json <file.kr>"));
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
fn emit_krbo_writes_valid_artifact() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let output_path = unique_temp_output_path("emit-krbo", "krbo");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let bytes = fs::read(&output_path).expect("read krbo output");
    assert!(bytes.len() >= 12, "krbo output too small");
    assert_eq!(&bytes[0..4], b"KRBO");
    assert_eq!(bytes[4], 0, "expected KRBO version major 0");
    assert_eq!(bytes[5], 1, "expected KRBO version minor 1");
    assert_eq!(bytes[9], 1, "expected x86_64-sysv target tag");

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_elfobj_writes_valid_relocatable_object() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let output_path = unique_temp_output_path("emit-elfobj", "o");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=elfobj")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let bytes = fs::read(&output_path).expect("read elf object output");
    assert!(bytes.len() >= 20, "elf object output too small");
    assert_eq!(&bytes[0..4], b"\x7fELF");
    assert_eq!(bytes[4], 2, "expected ELF64 class");
    assert_eq!(bytes[5], 1, "expected little-endian ELF");
    assert_eq!(
        u16::from_le_bytes([bytes[16], bytes[17]]),
        1,
        "expected ET_REL"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[18], bytes[19]]),
        62,
        "expected EM_X86_64"
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_asm_writes_expected_text_output() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let output_path = unique_temp_output_path("emit-asm", "s");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=asm")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let text = fs::read_to_string(&output_path).expect("read asm output");
    assert_eq!(
        text,
        ".text\n\n.globl bar\nbar:\n    ret\n\n.globl foo\nfoo:\n    call bar\n    ret\n"
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_krbo_supports_declared_extern_call_target_and_metadata_verifies() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let artifact_path = unique_temp_output_path("emit-krbo-extern-call", "krbo");
    let meta_path = unique_temp_output_path("emit-krbo-extern-call", "json");

    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_elfobj_supports_declared_extern_call_target_and_metadata_verifies() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let artifact_path = unique_temp_output_path("emit-elfobj-extern-call", "o");
    let meta_path = unique_temp_output_path("emit-elfobj-extern-call", "json");

    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_asm_supports_declared_extern_call_target_downstream() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let output_path = unique_temp_output_path("emit-asm-extern-call", "s");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=asm")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
    let text = fs::read_to_string(&output_path).expect("read asm output");
    assert_eq!(
        text,
        ".text\n\n.globl entry\nentry:\n    call ext\n    ret\n"
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_asm_supports_mixed_internal_and_declared_extern_targets_downstream() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_internal_chain.kr");
    let output_path = unique_temp_output_path("emit-asm-mixed-extern-chain", "s");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=asm")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();
    let text = fs::read_to_string(&output_path).expect("read asm output");
    assert_eq!(
        text,
        ".text\n\n.globl entry\nentry:\n    call helper\n    ret\n\n.globl helper\nhelper:\n    call ext\n    ret\n"
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_backend_artifacts_are_deterministic_for_supported_subset() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let krbo_a = unique_temp_output_path("emit-krbo-a", "krbo");
    let krbo_b = unique_temp_output_path("emit-krbo-b", "krbo");
    let elf_a = unique_temp_output_path("emit-elf-a", "o");
    let elf_b = unique_temp_output_path("emit-elf-b", "o");
    let asm_a = unique_temp_output_path("emit-asm-a", "s");
    let asm_b = unique_temp_output_path("emit-asm-b", "s");

    for (kind, first, second) in [
        ("krbo", &krbo_a, &krbo_b),
        ("elfobj", &elf_a, &elf_b),
        ("asm", &asm_a, &asm_b),
    ] {
        for path in [first, second] {
            fs::remove_file(path).ok();

            let mut cmd: Command = cargo_bin_cmd!("kernriftc");
            cmd.current_dir(&root)
                .arg(format!("--emit={kind}"))
                .arg("-o")
                .arg(path.as_os_str())
                .arg(fixture.as_os_str());
            cmd.assert().success();
        }

        let first_bytes = fs::read(first).expect("read first emitted artifact");
        let second_bytes = fs::read(second).expect("read second emitted artifact");
        assert_eq!(
            first_bytes, second_bytes,
            "emitted {kind} artifact must be byte-stable"
        );
    }

    for path in [&krbo_a, &krbo_b, &elf_a, &elf_b, &asm_a, &asm_b] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn inspect_artifact_text_summarizes_basic_asm_output() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-basic-asm", "s");
    emit_backend_artifact(&root, "asm", &fixture, &artifact_path, false);

    let output = inspect_artifact_output(&root, &artifact_path, None);
    assert!(output.contains("Artifact: asm_text\n"));
    assert!(output.contains("Machine: x86_64\n"));
    assert!(output.contains("Defined symbols:\n- bar\n- foo\n"));
    assert!(output.contains("ASM direct call targets:\n- bar\n"));
    assert!(output.contains("- has_entry_symbol: no\n"));

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_text_summarizes_extern_elf_object_output() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-extern-elf", "o");
    emit_backend_artifact(&root, "elfobj", &fixture, &artifact_path, false);

    let output = inspect_artifact_output(&root, &artifact_path, None);
    assert!(output.contains("Artifact: elf_relocatable\n"));
    assert!(output.contains("Machine: x86_64\n"));
    assert!(output.contains("Defined symbols:\n- entry\n"));
    assert!(output.contains("Undefined symbols:\n- ext\n"));
    assert!(output.contains("- .rela.text R_X86_64_PLT32 -> ext\n"));
    assert!(output.contains("- has_text_relocations: yes\n"));

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_text_summarizes_mixed_extern_asm_output() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_internal_chain.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-mixed-asm", "s");
    emit_backend_artifact(&root, "asm", &fixture, &artifact_path, false);

    let output = inspect_artifact_output(&root, &artifact_path, None);
    assert!(output.contains("Artifact: asm_text\n"));
    assert!(output.contains("Defined symbols:\n- entry\n- helper\n"));
    assert!(output.contains("Undefined symbols:\n- ext\n"));
    assert!(output.contains("ASM direct call targets:\n- ext\n- helper\n"));
    assert!(output.contains("- has_entry_symbol: yes\n"));
    assert!(output.contains("- has_undefined_symbols: yes\n"));

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_json_reports_krbo_header_and_symbols() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-krbo-json", "krbo");
    emit_backend_artifact(&root, "krbo", &fixture, &artifact_path, false);

    let output = inspect_artifact_output(&root, &artifact_path, Some("json"));
    let json: Value = serde_json::from_str(&output).expect("parse inspect-artifact JSON");
    assert_eq!(json["schema_version"], "kernrift_inspect_artifact_v1");
    assert_eq!(json["artifact_kind"], "krbo");
    assert_eq!(json["machine"], "x86_64");
    assert_eq!(json["pointer_bits"], 64);
    assert_eq!(json["defined_symbols"], json!(["bar", "foo"]));
    assert_eq!(json["undefined_symbols"], json!([]));
    assert_eq!(json["flags"]["has_text_relocations"], true);

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_json_reports_relocation_bearing_elf_object() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-elf-json", "o");
    emit_backend_artifact(&root, "elfobj", &fixture, &artifact_path, false);

    let output = inspect_artifact_output(&root, &artifact_path, Some("json"));
    let json: Value = serde_json::from_str(&output).expect("parse inspect-artifact JSON");
    assert_eq!(json["schema_version"], "kernrift_inspect_artifact_v1");
    assert_eq!(json["artifact_kind"], "elf_relocatable");
    assert_eq!(json["machine"], "x86_64");
    assert_eq!(json["undefined_symbols"], json!(["ext"]));
    assert_eq!(json["flags"]["has_text_relocations"], true);
    assert_eq!(json["relocations"][0]["section"], ".rela.text");
    assert_eq!(json["relocations"][0]["target"], "ext");

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_json_contract_shape_is_stable_across_krbo_elf_and_asm() {
    let root = repo_root();
    let basic_fixture = root.join("tests").join("must_pass").join("basic.kr");
    let extern_fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");

    let krbo_path = unique_temp_output_path("inspect-artifact-contract-krbo", "krbo");
    let elf_path = unique_temp_output_path("inspect-artifact-contract-elf", "o");
    let asm_path = unique_temp_output_path("inspect-artifact-contract-asm", "s");

    emit_backend_artifact(&root, "krbo", &basic_fixture, &krbo_path, false);
    emit_backend_artifact(&root, "elfobj", &extern_fixture, &elf_path, false);
    emit_backend_artifact(&root, "asm", &basic_fixture, &asm_path, false);

    let parse = |path: &Path| -> Value {
        serde_json::from_str(&inspect_artifact_output(&root, path, Some("json")))
            .expect("parse inspect-artifact json")
    };

    let krbo = parse(&krbo_path);
    let elf = parse(&elf_path);
    let asm = parse(&asm_path);

    for report in [&krbo, &elf, &asm] {
        assert_eq!(
            report["schema_version"],
            json!("kernrift_inspect_artifact_v1")
        );
        for key in [
            "schema_version",
            "artifact_kind",
            "file_size",
            "symbols",
            "defined_symbols",
            "undefined_symbols",
            "relocations",
            "flags",
        ] {
            assert!(
                report.get(key).is_some(),
                "missing required key '{}' in report: {}",
                key,
                report
            );
        }
        for key in [
            "has_entry_symbol",
            "has_undefined_symbols",
            "has_text_relocations",
        ] {
            assert!(
                report["flags"].get(key).is_some(),
                "missing required flag key '{}' in report: {}",
                key,
                report
            );
        }
        for symbol in report["symbols"].as_array().expect("symbols array") {
            assert!(
                symbol.get("name").is_some(),
                "symbol missing name: {}",
                symbol
            );
            assert!(
                symbol.get("category").is_some(),
                "symbol missing category: {}",
                symbol
            );
            assert!(
                symbol.get("definition").is_some(),
                "symbol missing definition: {}",
                symbol
            );
        }
        for relocation in report["relocations"].as_array().expect("relocations array") {
            assert!(
                relocation.get("section").is_some(),
                "relocation missing section: {}",
                relocation
            );
            assert!(
                relocation.get("type").is_some(),
                "relocation missing type: {}",
                relocation
            );
            assert!(
                relocation.get("target").is_some(),
                "relocation missing target: {}",
                relocation
            );
        }
    }

    assert!(krbo.get("pointer_bits").is_some());
    assert!(krbo.get("endianness").is_some());
    assert!(krbo.get("asm").is_none());
    assert!(elf.get("pointer_bits").is_some());
    assert!(elf.get("endianness").is_some());
    assert!(elf.get("asm").is_none());
    assert!(asm.get("pointer_bits").is_none());
    assert!(asm.get("endianness").is_none());
    assert!(asm.get("asm").is_some());

    for path in [&krbo_path, &elf_path, &asm_path] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn inspect_artifact_json_output_is_byte_stable() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-repeatable-json", "krbo");
    emit_backend_artifact(&root, "krbo", &fixture, &artifact_path, false);

    let first = inspect_artifact_output(&root, &artifact_path, Some("json"));
    let second = inspect_artifact_output(&root, &artifact_path, Some("json"));
    assert_eq!(first, second, "inspect-artifact JSON must be byte-stable");

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_json_transport_is_stdout_only_and_newline_terminated() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-json-transport", "krbo");
    emit_backend_artifact(&root, "krbo", &fixture, &artifact_path, false);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(artifact_path.as_os_str())
        .arg("--format")
        .arg("json");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_inspect_artifact_v1");

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_rejects_random_text_file() {
    let root = repo_root();
    let input_path = unique_temp_output_path("inspect-artifact-random-text", "txt");
    fs::write(&input_path, "hello artifact\n").expect("write random text");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(input_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("inspect-artifact: unsupported artifact bytes")
    );

    fs::remove_file(&input_path).ok();
}

#[test]
fn inspect_artifact_rejects_empty_file() {
    let root = repo_root();
    let input_path = unique_temp_output_path("inspect-artifact-empty", "bin");
    fs::write(&input_path, b"").expect("write empty file");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(input_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("inspect-artifact: unsupported artifact bytes")
    );

    fs::remove_file(&input_path).ok();
}

#[test]
fn inspect_artifact_rejects_malformed_known_magic_bytes() {
    let root = repo_root();
    let input_path = unique_temp_output_path("inspect-artifact-malformed-krbo", "krbo");
    fs::write(&input_path, b"KRBO").expect("write malformed KRBO bytes");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(input_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("inspect-artifact: failed to parse KRBO artifact: artifact too small")
    );

    fs::remove_file(&input_path).ok();
}

#[test]
fn inspect_artifact_rejects_elf_relocation_with_out_of_range_symbol_index() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let artifact_path = unique_temp_output_path("inspect-artifact-extern-reloc-idx", "o");
    let malformed_path =
        unique_temp_output_path("inspect-artifact-extern-reloc-idx-malformed", "o");

    emit_backend_artifact(&root, "elfobj", &fixture, &artifact_path, false);
    let mut bytes = fs::read(&artifact_path).expect("read emitted elf object");

    let shoff = u64::from_le_bytes(bytes[40..48].try_into().expect("u64")) as usize;
    let shentsize = u16::from_le_bytes(bytes[58..60].try_into().expect("u16")) as usize;
    let shnum = u16::from_le_bytes(bytes[60..62].try_into().expect("u16")) as usize;
    let rela_offset = (0..shnum)
        .find_map(|idx| {
            let base = shoff + idx * shentsize;
            let section_type = u32::from_le_bytes(bytes[base + 4..base + 8].try_into().ok()?);
            if section_type == 4 {
                Some(u64::from_le_bytes(bytes[base + 24..base + 32].try_into().ok()?) as usize)
            } else {
                None
            }
        })
        .expect("find SHT_RELA section");

    let malformed_r_info = (999u64 << 32) | 4;
    bytes[rela_offset + 8..rela_offset + 16].copy_from_slice(&malformed_r_info.to_le_bytes());
    fs::write(&malformed_path, bytes).expect("write malformed elf object");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(malformed_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "inspect-artifact: failed to parse ELF artifact: relocation section '.rela.text' entry 0 references out-of-range symbol index 999"
        )
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&malformed_path).ok();
}

#[test]
fn inspect_artifact_text_outputs_are_exact_for_fixture_matrix() {
    let root = repo_root();
    let basic_fixture = root.join("tests").join("must_pass").join("basic.kr");
    let extern_fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let mixed_fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_internal_chain.kr");

    let basic_krbo = unique_temp_output_path("inspect-exact-basic-krbo", "krbo");
    let basic_elf = unique_temp_output_path("inspect-exact-basic-elf", "o");
    let basic_asm = unique_temp_output_path("inspect-exact-basic-asm", "s");
    let extern_elf = unique_temp_output_path("inspect-exact-extern-elf", "o");
    let extern_asm = unique_temp_output_path("inspect-exact-extern-asm", "s");
    let mixed_elf = unique_temp_output_path("inspect-exact-mixed-elf", "o");
    let mixed_asm = unique_temp_output_path("inspect-exact-mixed-asm", "s");

    emit_backend_artifact(&root, "krbo", &basic_fixture, &basic_krbo, false);
    emit_backend_artifact(&root, "elfobj", &basic_fixture, &basic_elf, false);
    emit_backend_artifact(&root, "asm", &basic_fixture, &basic_asm, false);
    emit_backend_artifact(&root, "elfobj", &extern_fixture, &extern_elf, false);
    emit_backend_artifact(&root, "asm", &extern_fixture, &extern_asm, false);
    emit_backend_artifact(&root, "elfobj", &mixed_fixture, &mixed_elf, false);
    emit_backend_artifact(&root, "asm", &mixed_fixture, &mixed_asm, false);

    assert_eq!(
        inspect_artifact_output(&root, &basic_krbo, None),
        concat!(
            "Artifact: krbo\n",
            "File size: 136 bytes\n",
            "Machine: x86_64\n",
            "Pointer width: 64-bit\n",
            "Endianness: little\n",
            "Defined symbols:\n",
            "- bar\n",
            "- foo\n",
            "Undefined symbols:\n",
            "- <none>\n",
            "Relocations:\n",
            "- .text x86_64_call_rel32/w4 -> bar\n",
            "Flags:\n",
            "- has_entry_symbol: no\n",
            "- has_undefined_symbols: no\n",
            "- has_text_relocations: yes\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &basic_elf, None),
        concat!(
            "Artifact: elf_relocatable\n",
            "File size: 536 bytes\n",
            "Machine: x86_64\n",
            "Pointer width: 64-bit\n",
            "Endianness: little\n",
            "Defined symbols:\n",
            "- bar\n",
            "- foo\n",
            "Undefined symbols:\n",
            "- <none>\n",
            "Relocations:\n",
            "- <none>\n",
            "Flags:\n",
            "- has_entry_symbol: no\n",
            "- has_undefined_symbols: no\n",
            "- has_text_relocations: no\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &basic_asm, None),
        concat!(
            "Artifact: asm_text\n",
            "File size: 69 bytes\n",
            "Machine: x86_64\n",
            "Defined symbols:\n",
            "- bar\n",
            "- foo\n",
            "Undefined symbols:\n",
            "- <none>\n",
            "Relocations:\n",
            "- <none>\n",
            "ASM globals:\n",
            "- bar\n",
            "- foo\n",
            "ASM labels:\n",
            "- bar\n",
            "- foo\n",
            "ASM direct call targets:\n",
            "- bar\n",
            "ASM appears_x86_64_text_subset: yes\n",
            "Flags:\n",
            "- has_entry_symbol: no\n",
            "- has_undefined_symbols: no\n",
            "- has_text_relocations: no\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &extern_elf, None),
        concat!(
            "Artifact: elf_relocatable\n",
            "File size: 632 bytes\n",
            "Machine: x86_64\n",
            "Pointer width: 64-bit\n",
            "Endianness: little\n",
            "Defined symbols:\n",
            "- entry\n",
            "Undefined symbols:\n",
            "- ext\n",
            "Relocations:\n",
            "- .rela.text R_X86_64_PLT32 -> ext\n",
            "Flags:\n",
            "- has_entry_symbol: yes\n",
            "- has_undefined_symbols: yes\n",
            "- has_text_relocations: yes\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &extern_asm, None),
        concat!(
            "Artifact: asm_text\n",
            "File size: 48 bytes\n",
            "Machine: x86_64\n",
            "Defined symbols:\n",
            "- entry\n",
            "Undefined symbols:\n",
            "- ext\n",
            "Relocations:\n",
            "- <none>\n",
            "ASM globals:\n",
            "- entry\n",
            "ASM labels:\n",
            "- entry\n",
            "ASM direct call targets:\n",
            "- ext\n",
            "ASM appears_x86_64_text_subset: yes\n",
            "Flags:\n",
            "- has_entry_symbol: yes\n",
            "- has_undefined_symbols: yes\n",
            "- has_text_relocations: no\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &mixed_elf, None),
        concat!(
            "Artifact: elf_relocatable\n",
            "File size: 672 bytes\n",
            "Machine: x86_64\n",
            "Pointer width: 64-bit\n",
            "Endianness: little\n",
            "Defined symbols:\n",
            "- entry\n",
            "- helper\n",
            "Undefined symbols:\n",
            "- ext\n",
            "Relocations:\n",
            "- .rela.text R_X86_64_PLT32 -> ext\n",
            "Flags:\n",
            "- has_entry_symbol: yes\n",
            "- has_undefined_symbols: yes\n",
            "- has_text_relocations: yes\n"
        )
    );
    assert_eq!(
        inspect_artifact_output(&root, &mixed_asm, None),
        concat!(
            "Artifact: asm_text\n",
            "File size: 95 bytes\n",
            "Machine: x86_64\n",
            "Defined symbols:\n",
            "- entry\n",
            "- helper\n",
            "Undefined symbols:\n",
            "- ext\n",
            "Relocations:\n",
            "- <none>\n",
            "ASM globals:\n",
            "- entry\n",
            "- helper\n",
            "ASM labels:\n",
            "- entry\n",
            "- helper\n",
            "ASM direct call targets:\n",
            "- ext\n",
            "- helper\n",
            "ASM appears_x86_64_text_subset: yes\n",
            "Flags:\n",
            "- has_entry_symbol: yes\n",
            "- has_undefined_symbols: yes\n",
            "- has_text_relocations: no\n"
        )
    );

    for path in [
        &basic_krbo,
        &basic_elf,
        &basic_asm,
        &extern_elf,
        &extern_asm,
        &mixed_elf,
        &mixed_asm,
    ] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn inspect_artifact_json_outputs_are_exact_for_fixture_matrix() {
    let root = repo_root();
    let basic_fixture = root.join("tests").join("must_pass").join("basic.kr");
    let extern_fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_call_object.kr");
    let mixed_fixture = root
        .join("tests")
        .join("must_pass")
        .join("extern_internal_chain.kr");

    let basic_krbo = unique_temp_output_path("inspect-exact-json-basic-krbo", "krbo");
    let basic_elf = unique_temp_output_path("inspect-exact-json-basic-elf", "o");
    let basic_asm = unique_temp_output_path("inspect-exact-json-basic-asm", "s");
    let extern_elf = unique_temp_output_path("inspect-exact-json-extern-elf", "o");
    let extern_asm = unique_temp_output_path("inspect-exact-json-extern-asm", "s");
    let mixed_elf = unique_temp_output_path("inspect-exact-json-mixed-elf", "o");
    let mixed_asm = unique_temp_output_path("inspect-exact-json-mixed-asm", "s");

    emit_backend_artifact(&root, "krbo", &basic_fixture, &basic_krbo, false);
    emit_backend_artifact(&root, "elfobj", &basic_fixture, &basic_elf, false);
    emit_backend_artifact(&root, "asm", &basic_fixture, &basic_asm, false);
    emit_backend_artifact(&root, "elfobj", &extern_fixture, &extern_elf, false);
    emit_backend_artifact(&root, "asm", &extern_fixture, &extern_asm, false);
    emit_backend_artifact(&root, "elfobj", &mixed_fixture, &mixed_elf, false);
    emit_backend_artifact(&root, "asm", &mixed_fixture, &mixed_asm, false);

    let expected_json = |artifact_kind: &str,
                         file_size: usize,
                         machine: &str,
                         pointer_bits: Option<u64>,
                         endianness: Option<&str>,
                         symbols: Value,
                         defined_symbols: Value,
                         undefined_symbols: Value,
                         relocations: Value,
                         asm: Option<Value>,
                         flags: Value| {
        let mut obj = json!({
            "schema_version": "kernrift_inspect_artifact_v1",
            "artifact_kind": artifact_kind,
            "file_size": file_size,
            "machine": machine,
            "symbols": symbols,
            "defined_symbols": defined_symbols,
            "undefined_symbols": undefined_symbols,
            "relocations": relocations,
            "flags": flags
        });
        if let Some(pointer_bits) = pointer_bits {
            obj["pointer_bits"] = json!(pointer_bits);
        }
        if let Some(endianness) = endianness {
            obj["endianness"] = json!(endianness);
        }
        if let Some(asm) = asm {
            obj["asm"] = asm;
        }
        obj
    };

    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &basic_krbo, Some("json")))
            .expect("parse krbo inspect json"),
        expected_json(
            "krbo",
            136,
            "x86_64",
            Some(64),
            Some("little"),
            json!([
                {"name":"bar","category":"function","definition":"defined"},
                {"name":"foo","category":"function","definition":"defined"}
            ]),
            json!(["bar", "foo"]),
            json!([]),
            json!([{"section":".text","type":"x86_64_call_rel32/w4","target":"bar"}]),
            None,
            json!({"has_entry_symbol":false,"has_undefined_symbols":false,"has_text_relocations":true})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &basic_elf, Some("json")))
            .expect("parse basic elf inspect json"),
        expected_json(
            "elf_relocatable",
            536,
            "x86_64",
            Some(64),
            Some("little"),
            json!([
                {"name":"bar","category":"function","definition":"defined"},
                {"name":"foo","category":"function","definition":"defined"}
            ]),
            json!(["bar", "foo"]),
            json!([]),
            json!([]),
            None,
            json!({"has_entry_symbol":false,"has_undefined_symbols":false,"has_text_relocations":false})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &basic_asm, Some("json")))
            .expect("parse basic asm inspect json"),
        expected_json(
            "asm_text",
            69,
            "x86_64",
            None,
            None,
            json!([
                {"name":"bar","category":"function","definition":"defined"},
                {"name":"foo","category":"function","definition":"defined"}
            ]),
            json!(["bar", "foo"]),
            json!([]),
            json!([]),
            Some(json!({
                "globals":["bar","foo"],
                "labels":["bar","foo"],
                "direct_call_targets":["bar"],
                "appears_x86_64_text_subset": true
            })),
            json!({"has_entry_symbol":false,"has_undefined_symbols":false,"has_text_relocations":false})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &extern_elf, Some("json")))
            .expect("parse extern elf inspect json"),
        expected_json(
            "elf_relocatable",
            632,
            "x86_64",
            Some(64),
            Some("little"),
            json!([
                {"name":"entry","category":"function","definition":"defined"},
                {"name":"ext","category":"function","definition":"undefined"}
            ]),
            json!(["entry"]),
            json!(["ext"]),
            json!([{"section":".rela.text","type":"R_X86_64_PLT32","target":"ext"}]),
            None,
            json!({"has_entry_symbol":true,"has_undefined_symbols":true,"has_text_relocations":true})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &extern_asm, Some("json")))
            .expect("parse extern asm inspect json"),
        expected_json(
            "asm_text",
            48,
            "x86_64",
            None,
            None,
            json!([
                {"name":"entry","category":"function","definition":"defined"},
                {"name":"ext","category":"function","definition":"undefined"}
            ]),
            json!(["entry"]),
            json!(["ext"]),
            json!([]),
            Some(json!({
                "globals":["entry"],
                "labels":["entry"],
                "direct_call_targets":["ext"],
                "appears_x86_64_text_subset": true
            })),
            json!({"has_entry_symbol":true,"has_undefined_symbols":true,"has_text_relocations":false})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &mixed_elf, Some("json")))
            .expect("parse mixed elf inspect json"),
        expected_json(
            "elf_relocatable",
            672,
            "x86_64",
            Some(64),
            Some("little"),
            json!([
                {"name":"entry","category":"function","definition":"defined"},
                {"name":"ext","category":"function","definition":"undefined"},
                {"name":"helper","category":"function","definition":"defined"}
            ]),
            json!(["entry", "helper"]),
            json!(["ext"]),
            json!([{"section":".rela.text","type":"R_X86_64_PLT32","target":"ext"}]),
            None,
            json!({"has_entry_symbol":true,"has_undefined_symbols":true,"has_text_relocations":true})
        )
    );
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_artifact_output(&root, &mixed_asm, Some("json")))
            .expect("parse mixed asm inspect json"),
        expected_json(
            "asm_text",
            95,
            "x86_64",
            None,
            None,
            json!([
                {"name":"entry","category":"function","definition":"defined"},
                {"name":"ext","category":"function","definition":"undefined"},
                {"name":"helper","category":"function","definition":"defined"}
            ]),
            json!(["entry", "helper"]),
            json!(["ext"]),
            json!([]),
            Some(json!({
                "globals":["entry","helper"],
                "labels":["entry","helper"],
                "direct_call_targets":["ext","helper"],
                "appears_x86_64_text_subset": true
            })),
            json!({"has_entry_symbol":true,"has_undefined_symbols":true,"has_text_relocations":false})
        )
    );

    for path in [
        &basic_krbo,
        &basic_elf,
        &basic_asm,
        &extern_elf,
        &extern_asm,
        &mixed_elf,
        &mixed_asm,
    ] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn emit_krbo_sidecar_is_written_and_contains_expected_metadata() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("emit-krbo-sidecar", "krbo");
    let meta_path = unique_temp_output_path("emit-krbo-sidecar", "json");
    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let artifact_bytes = fs::read(&artifact_path).expect("read krbo output");
    let metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read krbo metadata"))
            .expect("parse krbo metadata");

    assert_eq!(
        metadata,
        json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "krbo",
            "surface": "stable",
            "byte_len": artifact_bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&artifact_bytes)),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "krbo": {
                "magic": "KRBO",
                "version_major": 0,
                "version_minor": 1,
                "format_revision": 2,
                "target_tag": 1,
                "target_name": "x86_64-sysv"
            }
        })
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_elfobj_sidecar_is_written_and_contains_expected_metadata() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("emit-elf-sidecar", "o");
    let meta_path = unique_temp_output_path("emit-elf-sidecar", "json");
    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=elfobj")
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    let artifact_bytes = fs::read(&artifact_path).expect("read elf output");
    let metadata: Value = serde_json::from_slice(&fs::read(&meta_path).expect("read elf metadata"))
        .expect("parse elf metadata");

    assert_eq!(
        metadata,
        json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "elfobj",
            "surface": "stable",
            "byte_len": artifact_bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&artifact_bytes)),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "elfobj": {
                "magic": "7f454c46",
                "class": "elf64",
                "endianness": "little",
                "elf_type": "relocatable",
                "machine": "x86_64"
            }
        })
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_backend_artifacts_with_explicit_stable_surface_match_default() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let krbo_default = unique_temp_output_path("emit-krbo-default", "krbo");
    let krbo_stable = unique_temp_output_path("emit-krbo-stable", "krbo");
    let elf_default = unique_temp_output_path("emit-elf-default", "o");
    let elf_stable = unique_temp_output_path("emit-elf-stable", "o");
    let krbo_default_meta = unique_temp_output_path("emit-krbo-default", "json");
    let krbo_stable_meta = unique_temp_output_path("emit-krbo-stable", "json");
    let elf_default_meta = unique_temp_output_path("emit-elf-default", "json");
    let elf_stable_meta = unique_temp_output_path("emit-elf-stable", "json");
    let asm_default = unique_temp_output_path("emit-asm-default", "s");
    let asm_stable = unique_temp_output_path("emit-asm-stable", "s");

    for (kind, default_path, stable_path, default_meta_path, stable_meta_path) in [
        (
            "krbo",
            &krbo_default,
            &krbo_stable,
            &krbo_default_meta,
            &krbo_stable_meta,
        ),
        (
            "elfobj",
            &elf_default,
            &elf_stable,
            &elf_default_meta,
            &elf_stable_meta,
        ),
    ] {
        fs::remove_file(default_path).ok();
        fs::remove_file(stable_path).ok();
        fs::remove_file(default_meta_path).ok();
        fs::remove_file(stable_meta_path).ok();

        let mut default_cmd: Command = cargo_bin_cmd!("kernriftc");
        default_cmd
            .current_dir(&root)
            .arg(format!("--emit={kind}"))
            .arg("-o")
            .arg(default_path.as_os_str())
            .arg("--meta-out")
            .arg(default_meta_path.as_os_str())
            .arg(fixture.as_os_str());
        default_cmd.assert().success();

        let mut stable_cmd: Command = cargo_bin_cmd!("kernriftc");
        stable_cmd
            .current_dir(&root)
            .arg("--surface")
            .arg("stable")
            .arg(format!("--emit={kind}"))
            .arg("-o")
            .arg(stable_path.as_os_str())
            .arg("--meta-out")
            .arg(stable_meta_path.as_os_str())
            .arg(fixture.as_os_str());
        stable_cmd.assert().success();

        let default_bytes = fs::read(default_path).expect("read default emitted artifact");
        let stable_bytes = fs::read(stable_path).expect("read stable emitted artifact");
        assert_eq!(
            default_bytes, stable_bytes,
            "explicit stable surface must match default {kind} output"
        );

        let default_meta = fs::read(default_meta_path).expect("read default metadata");
        let stable_meta = fs::read(stable_meta_path).expect("read stable metadata");
        assert_eq!(
            default_meta, stable_meta,
            "explicit stable surface must match default {kind} metadata"
        );
    }

    emit_backend_artifact(&root, "asm", &fixture, &asm_default, false);
    emit_backend_artifact(&root, "asm", &fixture, &asm_stable, true);
    let default_asm = fs::read(&asm_default).expect("read default asm");
    let stable_asm = fs::read(&asm_stable).expect("read stable asm");
    assert_eq!(
        default_asm, stable_asm,
        "explicit stable surface must match default asm output"
    );

    for path in [
        &krbo_default,
        &krbo_stable,
        &elf_default,
        &elf_stable,
        &krbo_default_meta,
        &krbo_stable_meta,
        &elf_default_meta,
        &elf_stable_meta,
        &asm_default,
        &asm_stable,
    ] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn emit_backend_artifact_requires_output_path() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=krbo")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid emit mode: missing -o <output-path>")
    );
}

#[test]
fn emit_backend_artifact_rejects_invalid_surface_value() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let output_path = unique_temp_output_path("emit-invalid-surface", "krbo");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--surface")
        .arg("beta")
        .arg("--emit=krbo")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid emit mode: invalid surface mode 'beta', expected 'stable' or 'experimental'")
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_backend_artifact_meta_out_requires_output_path() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("emit-meta-missing-path", "krbo");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg(fixture.as_os_str())
        .arg("--meta-out");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid emit mode: --meta-out requires a file path")
    );

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn emit_asm_rejects_meta_out() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let output_path = unique_temp_output_path("emit-asm-meta-out", "s");
    let meta_path = unique_temp_output_path("emit-asm-meta-out", "json");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=asm")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid emit mode: --meta-out is unsupported for 'asm'")
    );

    fs::remove_file(&output_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_backend_artifact_sidecar_normalizes_repo_relative_input_path() {
    let root = repo_root();
    let fixture_rel = Path::new("tests").join("must_pass").join("basic.kr");
    let artifact_rel = unique_temp_output_path("emit-relative-input", "krbo");
    let meta_rel = unique_temp_output_path("emit-relative-input", "json");

    for path in [&artifact_rel, &meta_rel] {
        fs::remove_file(path).ok();
    }

    let mut rel_cmd: Command = cargo_bin_cmd!("kernriftc");
    rel_cmd
        .current_dir(&root)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(artifact_rel.as_os_str())
        .arg("--meta-out")
        .arg(meta_rel.as_os_str())
        .arg(fixture_rel.as_os_str());
    rel_cmd.assert().success();

    let rel_json: Value =
        serde_json::from_slice(&fs::read(&meta_rel).expect("read relative metadata"))
            .expect("parse normalized metadata");
    assert_eq!(rel_json["input_path"], "tests/must_pass/basic.kr");
    assert_eq!(rel_json["input_path_kind"], "repo-relative");

    for path in [&artifact_rel, &meta_rel] {
        fs::remove_file(path).ok();
    }
}

#[test]
fn emit_backend_artifact_sidecar_normalizes_absolute_repo_input_outside_repo_cwd() {
    let root = repo_root();
    let fixture_abs = root.join("tests").join("must_pass").join("basic.kr");
    let outside_cwd = unique_temp_output_path("emit-outside-cwd", "dir");
    let artifact_path = unique_temp_output_path("emit-outside-cwd", "krbo");
    let meta_path = unique_temp_output_path("emit-outside-cwd", "json");
    fs::create_dir_all(&outside_cwd).expect("create outside cwd");

    for path in [&artifact_path, &meta_path] {
        fs::remove_file(path).ok();
    }

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&outside_cwd)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(fixture_abs.as_os_str());
    cmd.assert().success();

    let metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read outside-cwd metadata"))
            .expect("parse outside-cwd metadata");
    assert_eq!(metadata["input_path"], "tests/must_pass/basic.kr");
    assert_eq!(metadata["input_path_kind"], "repo-relative");

    for path in [&artifact_path, &meta_path] {
        fs::remove_file(path).ok();
    }
    fs::remove_dir_all(&outside_cwd).ok();
}

#[test]
fn emit_backend_artifact_sidecar_falls_back_to_raw_input_path_for_non_git_repo_file() {
    let root = repo_root();
    let external_root = unique_temp_output_path("emit-external-input", "dir");
    let external_fixture = external_root
        .join("tests")
        .join("must_pass")
        .join("basic.kr");
    let artifact_path = unique_temp_output_path("emit-external-input", "krbo");
    let meta_path = unique_temp_output_path("emit-external-input", "json");
    fs::create_dir_all(external_fixture.parent().expect("external fixture parent"))
        .expect("create external fixture tree");
    fs::create_dir_all(external_root.join(".git")).expect("create fake git dir");
    fs::create_dir_all(external_root.join("crates")).expect("create fake crates dir");
    fs::create_dir_all(external_root.join("docs")).expect("create fake docs dir");
    fs::write(&external_fixture, "fn entry() {\n}\n").expect("write external fixture");

    for path in [&artifact_path, &meta_path] {
        fs::remove_file(path).ok();
    }

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=krbo")
        .arg("-o")
        .arg(artifact_path.as_os_str())
        .arg("--meta-out")
        .arg(meta_path.as_os_str())
        .arg(external_fixture.as_os_str());
    cmd.assert().success();

    let metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read raw-path metadata"))
            .expect("parse raw-path metadata");
    assert_eq!(
        metadata["input_path"],
        external_fixture.to_string_lossy().to_string()
    );
    assert_eq!(metadata["input_path_kind"], "raw");

    for path in [&artifact_path, &meta_path] {
        fs::remove_file(path).ok();
    }
    fs::remove_dir_all(&external_root).ok();
}

#[test]
fn verify_artifact_meta_accepts_matching_krbo_artifact() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-krbo", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-krbo", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_accepts_matching_elf_object_artifact() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-elf", "o");
    let meta_path = unique_temp_output_path("verify-meta-elf", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_json_reports_success_with_schema_marker() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-json-success", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-json-success", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("json")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let json: Value = serde_json::from_str(&stdout).expect("parse verify-artifact-meta JSON");
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_verify_artifact_meta_v1",
            "result": "pass",
            "exit_code": 0,
            "message": "verify-artifact-meta: PASS"
        })
    );
    assert!(stderr.is_empty(), "expected empty stderr, got: {stderr}");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_accepts_krbo_artifact_with_extra_elf_block() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-krbo-extra-elf", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-krbo-extra-elf", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["elfobj"] = json!({
        "magic": "7f454c46",
        "class": "elf64",
        "endianness": "little",
        "elf_type": "relocatable",
        "machine": "arm64"
    });
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_accepts_elf_object_with_extra_krbo_block() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-elf-extra-krbo", "o");
    let meta_path = unique_temp_output_path("verify-meta-elf-extra-krbo", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["krbo"] = json!({
        "magic": "KRBO",
        "version_major": 0,
        "version_minor": 1,
        "format_revision": 999,
        "target_tag": 1,
        "target_name": "other-target"
    });
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_json_reports_mismatch_with_schema_marker() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-json-mismatch", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-json-mismatch", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);
    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    let artifact_sha256 = format!("{:x}", Sha256::digest(&artifact_bytes));

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["sha256"] = json!("00");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("json")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let json: Value = serde_json::from_str(&stdout).expect("parse verify-artifact-meta JSON");
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_verify_artifact_meta_v1",
            "result": "mismatch",
            "exit_code": 1,
            "message": format!("verify-artifact-meta: sha256 mismatch: metadata 00, artifact {}", artifact_sha256)
        })
    );
    assert!(stderr.is_empty(), "expected empty stderr, got: {stderr}");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_json_reports_invalid_input_with_schema_marker() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-json-invalid-input", "bin");
    let meta_path = unique_temp_output_path("verify-meta-json-invalid-input", "json");
    fs::write(&artifact_path, b"not-an-artifact").expect("write unsupported artifact");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "krbo",
            "surface": "stable",
            "byte_len": 15,
            "sha256": format!("{:x}", Sha256::digest(b"not-an-artifact")),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "krbo": {
                "magic": "KRBO",
                "version_major": 0,
                "version_minor": 1,
                "format_revision": 2,
                "target_tag": 1,
                "target_name": "x86_64-sysv"
            }
        }))
        .expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("json")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let json: Value = serde_json::from_str(&stdout).expect("parse verify-artifact-meta JSON");
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_verify_artifact_meta_v1",
            "result": "invalid_input",
            "exit_code": 2,
            "message": "verify-artifact-meta: unsupported artifact bytes"
        })
    );
    assert!(stderr.is_empty(), "expected empty stderr, got: {stderr}");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_json_transport_is_stdout_only_and_newline_terminated() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-json-transport", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-json-transport", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("json")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_verify_artifact_meta_v1");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_tampered_sha256() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-sha", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-sha", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);
    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    let artifact_sha256 = format!("{:x}", Sha256::digest(&artifact_bytes));

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["sha256"] = json!("00");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let expected = format!(
        "verify-artifact-meta: sha256 mismatch: metadata 00, artifact {}",
        artifact_sha256
    );
    assert_eq!(stderr.lines().next(), Some(expected.as_str()));

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_tampered_byte_len() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-len", "o");
    let meta_path = unique_temp_output_path("verify-meta-bad-len", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let artifact_bytes = fs::read(&artifact_path).expect("read artifact");
    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["byte_len"] = json!(artifact_bytes.len() + 1);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            format!(
                "verify-artifact-meta: byte_len mismatch: metadata {}, artifact {}",
                artifact_bytes.len() + 1,
                artifact_bytes.len()
            )
            .as_str()
        )
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_mismatched_emit_kind() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-kind", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-kind", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["emit_kind"] = json!("elfobj");
    metadata["krbo"] = Value::Null;
    metadata["elfobj"] = json!({
        "magic": "7f454c46",
        "class": "elf64",
        "endianness": "little",
        "elf_type": "relocatable",
        "machine": "x86_64"
    });
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: emit_kind mismatch: metadata 'elfobj', artifact 'krbo'")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_invalid_json() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-invalid-json", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-invalid-json", "json");
    fs::write(&artifact_path, b"KRBO\x00\x01\x02\x00\x00\x01\x00\x00").expect("write artifact");
    fs::write(&meta_path, b"{").expect("write invalid json");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}':",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_unsupported_schema_version() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-schema", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-schema", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["schema_version"] = json!("kernrift_artifact_meta_v999");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "unsupported artifact metadata schema_version 'kernrift_artifact_meta_v999', expected 'kernrift_artifact_meta_v1'"
        )
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_krbo_header_mismatch() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-header", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-header", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["krbo"]["format_revision"] = json!(999);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: krbo.format_revision mismatch: metadata 999, artifact 2")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_elf_header_mismatch() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-elf-header", "o");
    let meta_path = unique_temp_output_path("verify-meta-bad-elf-header", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["elfobj"]["machine"] = json!("arm64");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: elfobj.machine mismatch: metadata 'arm64', artifact 'x86_64'")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_krbo_block() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-krbo", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-krbo", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["krbo"] = Value::Null;
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: metadata missing krbo block")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_elf_block() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-elf", "o");
    let meta_path = unique_temp_output_path("verify-meta-missing-elf", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["elfobj"] = Value::Null;
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: metadata missing elfobj block")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_unsupported_artifact_bytes() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-unsupported-artifact", "bin");
    let meta_path = unique_temp_output_path("verify-meta-unsupported-artifact", "json");
    fs::write(&artifact_path, b"NOTANARTIFACT").expect("write bad artifact");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "krbo",
            "surface": "stable",
            "byte_len": 13,
            "sha256": format!("{:x}", Sha256::digest(b"NOTANARTIFACT")),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "krbo": {
                "magic": "KRBO",
                "version_major": 0,
                "version_minor": 1,
                "format_revision": 2,
                "target_tag": 1,
                "target_name": "x86_64-sysv"
            }
        }))
        .expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: unsupported artifact bytes")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_unreadable_artifact_path() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-missing-artifact", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-artifact", "json");
    fs::remove_file(&artifact_path).ok();
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "krbo",
            "surface": "stable",
            "byte_len": 0,
            "sha256": "",
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "krbo": {
                "magic": "KRBO",
                "version_major": 0,
                "version_minor": 1,
                "format_revision": 2,
                "target_tag": 1,
                "target_name": "x86_64-sysv"
            }
        }))
        .expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to read artifact '{}':",
                artifact_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_unreadable_metadata_path() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-meta", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-meta", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);
    fs::remove_file(&meta_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to read artifact metadata '{}':",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_wrong_json_field_types() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-types", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-types", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["byte_len"] = json!("123");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': invalid type:",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_krbo_target_name_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-krbo-target-name", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-krbo-target-name", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["krbo"]
        .as_object_mut()
        .expect("krbo metadata object")
        .remove("target_name");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `target_name`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_wrong_krbo_target_name_type() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-krbo-target-name-type", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-krbo-target-name-type", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["krbo"]["target_name"] = json!(123);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': invalid type: integer `123`, expected a string",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_elf_machine_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-elf-machine", "o");
    let meta_path = unique_temp_output_path("verify-meta-missing-elf-machine", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["elfobj"]
        .as_object_mut()
        .expect("elfobj metadata object")
        .remove("machine");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `machine`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_wrong_elf_machine_type() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-elf-machine-type", "o");
    let meta_path = unique_temp_output_path("verify-meta-bad-elf-machine-type", "json");
    emit_backend_artifact_with_sidecar(
        &root,
        "elfobj",
        &fixture,
        &artifact_path,
        &meta_path,
        false,
    );

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["elfobj"]["machine"] = json!(123);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': invalid type: integer `123`, expected a string",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_args() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("verify-artifact-meta");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: expected <artifact> <meta.json>")
    );
}

#[test]
fn verify_artifact_meta_rejects_only_one_positional() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("artifact-only.krbo");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: expected <artifact> <meta.json>")
    );
}

#[test]
fn verify_artifact_meta_rejects_extra_positional() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("artifact.krbo")
        .arg("meta.json")
        .arg("extra");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: expected <artifact> <meta.json>")
    );
}

#[test]
fn verify_artifact_meta_rejects_unexpected_flag() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--flag");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: unexpected argument '--flag'")
    );
}

#[test]
fn verify_artifact_meta_rejects_format_missing_value() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: --format requires 'text' or 'json'")
    );
}

#[test]
fn verify_artifact_meta_rejects_duplicate_format_flag() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("json")
        .arg("--format")
        .arg("text")
        .arg("artifact.krbo")
        .arg("meta.json");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid verify-artifact-meta mode: duplicate --format")
    );
}

#[test]
fn verify_artifact_meta_rejects_unsupported_format_value() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg("--format")
        .arg("yaml")
        .arg("artifact.krbo")
        .arg("meta.json");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "invalid verify-artifact-meta mode: unsupported --format 'yaml' (expected 'text' or 'json')"
        )
    );
}

#[test]
fn verify_artifact_meta_rejects_missing_schema_version_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-schema-version", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-schema-version", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("schema_version");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            format!(
                "failed to decode artifact metadata '{}': missing string field 'schema_version'",
                meta_path.display()
            )
            .as_str()
        )
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_emit_kind_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-emit-kind", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-emit-kind", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("emit_kind");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `emit_kind`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_wrong_emit_kind_type() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-emit-kind-type", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-emit-kind-type", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["emit_kind"] = json!(123);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': invalid type: integer `123`, expected a string",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_accepts_provenance_field_mismatches() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-provenance-ignored", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-provenance-ignored", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["surface"] = json!("experimental");
    metadata["input_path"] = json!("/tmp/not-the-original-source.kr");
    metadata["input_path_kind"] = json!("raw");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert_eq!(stdout, "verify-artifact-meta: PASS\n");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_surface_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-surface", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-surface", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("surface");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `surface`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_input_path_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-input-path", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-input-path", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("input_path");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `input_path`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_missing_input_path_kind_field() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-missing-input-path-kind", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-missing-input-path-kind", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata
        .as_object_mut()
        .expect("metadata object")
        .remove("input_path_kind");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': missing field `input_path_kind`",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_wrong_provenance_field_types() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-bad-provenance-types", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-bad-provenance-types", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);

    let mut metadata: Value =
        serde_json::from_slice(&fs::read(&meta_path).expect("read metadata")).expect("parse json");
    metadata["surface"] = json!(123);
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr
            .lines()
            .next()
            .expect("stderr line")
            .starts_with(&format!(
                "failed to decode artifact metadata '{}': invalid type: integer `123`, expected a string",
                meta_path.display()
            )),
        "unexpected stderr: {stderr}"
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_recognizable_but_too_small_krbo_bytes() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-short-krbo", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-short-krbo", "json");
    fs::write(&artifact_path, b"KRBO\x00\x01").expect("write short krbo artifact");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "krbo",
            "surface": "stable",
            "byte_len": 6,
            "sha256": format!("{:x}", Sha256::digest(b"KRBO\x00\x01")),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "krbo": {
                "magic": "KRBO",
                "version_major": 0,
                "version_minor": 1,
                "format_revision": 2,
                "target_tag": 1,
                "target_name": "x86_64-sysv"
            }
        }))
        .expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: failed to derive krbo metadata: artifact too small")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_rejects_recognizable_but_too_small_elf_bytes() {
    let root = repo_root();
    let artifact_path = unique_temp_output_path("verify-meta-short-elf", "o");
    let meta_path = unique_temp_output_path("verify-meta-short-elf", "json");
    fs::write(&artifact_path, b"\x7fELF\x02\x01").expect("write short elf artifact");
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "kernrift_artifact_meta_v1",
            "emit_kind": "elfobj",
            "surface": "stable",
            "byte_len": 6,
            "sha256": format!("{:x}", Sha256::digest(b"\x7fELF\x02\x01")),
            "input_path": "tests/must_pass/basic.kr",
            "input_path_kind": "repo-relative",
            "elfobj": {
                "magic": "7f454c46",
                "class": "elf64",
                "endianness": "little",
                "elf_type": "relocatable",
                "machine": "x86_64"
            }
        }))
        .expect("serialize metadata"),
    )
    .expect("write metadata");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("verify-artifact-meta")
        .arg(artifact_path.as_os_str())
        .arg(meta_path.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: failed to derive elfobj metadata: artifact too small")
    );

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn emit_backend_artifact_rejects_unsupported_current_subset() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("alloc_outside_critical.kr");
    let output_path = unique_temp_output_path("emit-unsupported", "o");
    fs::remove_file(&output_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("--emit=elfobj")
        .arg("-o")
        .arg(output_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec!["canonical-exec: function 'entry' contains unsupported allocpoint()"]
    );

    fs::remove_file(&output_path).ok();
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
            "raw_mmio_symbols: 0 []",
            "raw_mmio_sites_count: 0",
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
fn check_extern_missing_eff_includes_location_and_valid_template() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("extern_missing_eff.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "extern 'sleep' must declare @eff(...) facts explicitly at 2:1",
            "  2 | extern @ctx(thread) @caps() fn sleep();",
            "  = help: use the canonical extern skeleton: extern @ctx(...) @eff(...) @caps() fn sleep();",
        ]
    );
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
        vec![
            "surface feature '@irq_handler' is a compatibility alias and requires --surface experimental for 'isr' at 1:1",
            "  1 | @irq_handler",
            "  = help: did you mean the canonical spelling @ctx(irq)? this compatibility alias is kept only for migration guidance.",
        ]
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
        vec![
            "surface feature '@may_block' is a compatibility alias and requires --surface experimental for 'worker' at 1:1",
            "  1 | @may_block",
            "  = help: did you mean the canonical spelling @eff(block)? this compatibility alias is kept only for migration guidance.",
        ]
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
            "surface feature '@irq_legacy' is a deprecated alias and is unavailable under --surface stable for 'legacy_isr' at 1:1",
            "  1 | @irq_legacy",
            "  = help: did you mean the canonical spelling @ctx(irq)? this deprecated alias is kept only for migration guidance.",
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
            "surface feature '@irq_legacy' is a deprecated alias and is unavailable under --surface experimental for 'legacy_isr' at 1:1",
            "  1 | @irq_legacy",
            "  = help: did you mean the canonical spelling @ctx(irq)? this deprecated alias is kept only for migration guidance.",
        ]
    );
}

#[test]
fn check_rejects_legacy_yieldpoint_attribute_with_canonical_guidance() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("legacy_yieldpoint_attr.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "legacy spelling '@yieldpoint' is non-canonical and is not accepted on function 'pump' at 1:1",
            "  1 | @yieldpoint",
            "  = help: did you mean the canonical spelling yieldpoint()? control-point markers use statement form, not attributes.",
        ]
    );
}

#[test]
fn check_canonical_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "canonical check must report via stdout only"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "canonical_findings: 5",
            "function: alloc_worker",
            "classification: compatibility_alias",
            "surface_form: @alloc",
            "canonical_replacement: @eff(alloc)",
            "migration_safe: true",
            "function: block_worker",
            "classification: compatibility_alias",
            "surface_form: @block",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "function: irq_entry",
            "classification: compatibility_alias",
            "surface_form: @irq",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "function: noirq_worker",
            "classification: compatibility_alias",
            "surface_form: @noirq",
            "canonical_replacement: @ctx(thread, boot)",
            "migration_safe: true",
            "function: preempt_guarded",
            "classification: compatibility_alias",
            "surface_form: @preempt_off",
            "canonical_replacement: @eff(preempt_off)",
            "migration_safe: true",
        ]
    );
}

#[test]
fn check_canonical_json_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--format")
        .arg("json")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"stable\",\n  \"canonical_findings\": 5,\n  \"findings\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn check_canonical_reports_accepted_aliases_under_experimental_surface() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "canonical check must report via stdout only"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "canonical_findings: 3",
            "function: blocker",
            "classification: compatibility_alias",
            "surface_form: @may_block",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "function: isr",
            "classification: compatibility_alias",
            "surface_form: @irq_handler",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "function: worker",
            "classification: compatibility_alias",
            "surface_form: @thread_entry",
            "canonical_replacement: @ctx(thread)",
            "migration_safe: true",
        ]
    );
}

#[test]
fn check_canonical_json_reports_accepted_aliases_under_experimental_surface() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"experimental\",\n  \"canonical_findings\": 3,\n  \"findings\": [\n    {\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn check_canonical_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "canonical check success must keep stderr empty"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["surface: stable", "canonical_findings: 0"]
    );
}

#[test]
fn check_canonical_json_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--format")
        .arg("json")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"stable\",\n  \"canonical_findings\": 0,\n  \"findings\": []\n}\n"
    );
}

#[test]
fn check_canonical_rejects_policy_and_output_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--policy")
        .arg("policies/kernel.toml")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "invalid check mode: --canonical cannot be combined with --profile, --contracts-schema, --contracts-out, --policy, --hash-out, --sign-ed25519, or --sig-out"
        )
    );
}

#[test]
fn fix_canonical_rewrites_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-legacy-unary", &fixture);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_applied: 5",
            &format!("file: {}", temp_fixture.display()),
            "function: alloc_worker",
            "surface_form: @alloc",
            "canonical_replacement: @eff(alloc)",
            "function: block_worker",
            "surface_form: @block",
            "canonical_replacement: @eff(block)",
            "function: irq_entry",
            "surface_form: @irq",
            "canonical_replacement: @ctx(irq)",
            "function: noirq_worker",
            "surface_form: @noirq",
            "canonical_replacement: @ctx(thread, boot)",
            "function: preempt_guarded",
            "surface_form: @preempt_off",
            "canonical_replacement: @eff(preempt_off)",
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read rewritten fixture"),
        "@eff(alloc)\nfn alloc_worker() { }\n\n@eff(block)\nfn block_worker() { }\n\n@ctx(irq)\nfn irq_entry() { }\n\n@ctx(thread, boot)\nfn noirq_worker() { }\n\n@eff(preempt_off)\nfn preempt_guarded() { }\n"
    );
}

#[test]
fn fix_canonical_json_rewrites_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-json-legacy-unary", &fixture);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--format")
        .arg("json")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_result_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_result_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_result_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"{}\",\n  \"rewrites_applied\": 5,\n  \"changed\": true,\n  \"rewrites\": [\n    {{\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true\n    }}\n  ]\n}}\n",
            temp_fixture.display()
        )
    );
}

#[test]
fn fix_canonical_rewrites_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-aliases", &fixture);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "rewrites_applied: 3",
            &format!("file: {}", temp_fixture.display()),
            "function: blocker",
            "surface_form: @may_block",
            "canonical_replacement: @eff(block)",
            "function: isr",
            "surface_form: @irq_handler",
            "canonical_replacement: @ctx(irq)",
            "function: worker",
            "surface_form: @thread_entry",
            "canonical_replacement: @ctx(thread)",
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read rewritten fixture"),
        "@eff(block)\nfn blocker() { }\n\n@ctx(irq)\nfn isr() { }\n\n@ctx(thread)\nfn worker() { }\n"
    );
}

#[test]
fn fix_canonical_json_rewrites_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-json-aliases", &fixture);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_result_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_result_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_result_v1\",\n  \"surface\": \"experimental\",\n  \"file\": \"{}\",\n  \"rewrites_applied\": 3,\n  \"changed\": true,\n  \"rewrites\": [\n    {{\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true\n    }}\n  ]\n}}\n",
            temp_fixture.display()
        )
    );
}

#[test]
fn fix_canonical_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_applied: 0",
            &format!("file: {}", temp_fixture.display()),
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_json_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-json-noop", &fixture);

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--format")
        .arg("json")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_result_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_result_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_result_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"{}\",\n  \"rewrites_applied\": 0,\n  \"changed\": false,\n  \"rewrites\": []\n}}\n",
            temp_fixture.display()
        )
    );
}

#[test]
fn fix_canonical_dry_run_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix dry-run mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_planned: 5",
            &format!("file: {}", temp_fixture.display()),
            "function: alloc_worker",
            "surface_form: @alloc",
            "canonical_replacement: @eff(alloc)",
            "function: block_worker",
            "surface_form: @block",
            "canonical_replacement: @eff(block)",
            "function: irq_entry",
            "surface_form: @irq",
            "canonical_replacement: @ctx(irq)",
            "function: noirq_worker",
            "surface_form: @noirq",
            "canonical_replacement: @ctx(thread, boot)",
            "function: preempt_guarded",
            "surface_form: @preempt_off",
            "canonical_replacement: @eff(preempt_off)",
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_dry_run_json_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_preview_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_preview_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_preview_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"{}\",\n  \"rewrites_planned\": 5,\n  \"would_change\": true,\n  \"rewrites\": [\n    {{\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true\n    }}\n  ]\n}}\n",
            temp_fixture.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_dry_run_reports_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix dry-run mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "rewrites_planned: 3",
            &format!("file: {}", temp_fixture.display()),
            "function: blocker",
            "surface_form: @may_block",
            "canonical_replacement: @eff(block)",
            "function: isr",
            "surface_form: @irq_handler",
            "canonical_replacement: @ctx(irq)",
            "function: worker",
            "surface_form: @thread_entry",
            "canonical_replacement: @ctx(thread)",
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_dry_run_json_reports_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_preview_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_preview_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_preview_v1\",\n  \"surface\": \"experimental\",\n  \"file\": \"{}\",\n  \"rewrites_planned\": 3,\n  \"would_change\": true,\n  \"rewrites\": [\n    {{\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    }},\n    {{\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true\n    }}\n  ]\n}}\n",
            temp_fixture.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_dry_run_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix dry-run mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_planned: 0",
            &format!("file: {}", temp_fixture.display()),
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_dry_run_json_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_preview_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_preview_schema(&json);
    assert_eq!(
        stdout,
        format!(
            "{{\n  \"schema_version\": \"kernrift_canonical_fix_preview_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"{}\",\n  \"rewrites_planned\": 0,\n  \"would_change\": false,\n  \"rewrites\": []\n}}\n",
            temp_fixture.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_rejects_conflicting_write_and_dry_run_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--dry-run")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("invalid fix mode: exactly one of --write or --dry-run must be specified")
    );
}

#[test]
fn fix_canonical_rejects_duplicate_dry_run_flag() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--dry-run")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: duplicate --dry-run"));
}

#[test]
fn fix_canonical_is_idempotent() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-idempotent", &fixture);

    let mut first: Command = cargo_bin_cmd!("kernriftc");
    first
        .current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg(temp_fixture.as_os_str());
    first.assert().success();
    let rewritten_once = fs::read_to_string(&temp_fixture).expect("read once-rewritten fixture");

    let mut second: Command = cargo_bin_cmd!("kernriftc");
    second
        .current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg(temp_fixture.as_os_str());
    let assert = second.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_applied: 0",
            &format!("file: {}", temp_fixture.display()),
        ]
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read twice-rewritten fixture"),
        rewritten_once
    );
}

#[test]
fn fix_canonical_failure_keeps_original_file_unchanged() {
    let root = repo_root();
    let temp_fixture =
        write_temp_source_fixture("fix-canonical-parse-failure", "@ctx(\nfn broken() { }\n");
    let original = fs::read_to_string(&temp_fixture).expect("read invalid fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--format")
        .arg("json")
        .arg(temp_fixture.as_os_str());
    cmd.assert().failure().code(1);

    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged invalid fixture"),
        original
    );
}

#[test]
fn canonical_findings_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_findings_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_findings_v1",
            "surface": "stable",
            "canonical_findings": 0,
            "findings": []
        }),
        json!({
            "schema_version": "kernrift_canonical_findings_v1",
            "surface": "experimental",
            "canonical_findings": 1,
            "findings": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical findings JSON must validate against canonical findings v1 schema: {}",
                details.join(" | ")
            );
        }
    }
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
            "features: 6",
            "feature: legacy_alloc_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @alloc",
            "lowering_target: @eff(alloc)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(alloc)",
            "rewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.",
            "feature: legacy_block_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @block",
            "lowering_target: @eff(block)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(block)",
            "rewrite_intent: Replace the attribute token `@block` with `@eff(block)`.",
            "feature: legacy_irq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @irq",
            "lowering_target: @ctx(irq)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @ctx(irq)",
            "rewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.",
            "feature: legacy_noirq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @noirq",
            "lowering_target: @ctx(thread, boot)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @ctx(thread, boot)",
            "rewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
            "feature: legacy_preempt_off_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @preempt_off",
            "lowering_target: @eff(preempt_off)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(preempt_off)",
            "rewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
            "feature: thread_entry_alias",
            "status: stable",
            "classification: compatibility_alias",
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
            "features: 8",
            "feature: irq_handler_alias",
            "status: experimental",
            "classification: compatibility_alias",
            "surface_form: @irq_handler",
            "lowering_target: @ctx(irq)",
            "proposal_id: irq_handler_alias",
            "migration_safe: true",
            "canonical_replacement: @ctx(irq)",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "feature: legacy_alloc_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @alloc",
            "lowering_target: @eff(alloc)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(alloc)",
            "rewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.",
            "feature: legacy_block_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @block",
            "lowering_target: @eff(block)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(block)",
            "rewrite_intent: Replace the attribute token `@block` with `@eff(block)`.",
            "feature: legacy_irq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @irq",
            "lowering_target: @ctx(irq)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @ctx(irq)",
            "rewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.",
            "feature: legacy_noirq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @noirq",
            "lowering_target: @ctx(thread, boot)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @ctx(thread, boot)",
            "rewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
            "feature: legacy_preempt_off_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "surface_form: @preempt_off",
            "lowering_target: @eff(preempt_off)",
            "proposal_id: -",
            "migration_safe: true",
            "canonical_replacement: @eff(preempt_off)",
            "rewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
            "feature: may_block_alias",
            "status: experimental",
            "classification: compatibility_alias",
            "surface_form: @may_block",
            "lowering_target: @eff(block)",
            "proposal_id: may_block_alias",
            "migration_safe: true",
            "canonical_replacement: @eff(block)",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "feature: thread_entry_alias",
            "status: stable",
            "classification: compatibility_alias",
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
    assert_eq!(stable.lines().nth(1), Some("features: 6"));
    assert_eq!(experimental.lines().nth(1), Some("features: 8"));
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
            "classification: compatibility_alias",
            "enabled_under_surface: false",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "function: isr",
            "surface_form: @irq_handler",
            "feature: irq_handler_alias",
            "status: experimental",
            "classification: compatibility_alias",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "function: legacy_isr",
            "surface_form: @irq_legacy",
            "feature: irq_legacy_alias",
            "status: deprecated",
            "classification: deprecated_alias",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
            "function: worker",
            "surface_form: @thread_entry",
            "feature: thread_entry_alias",
            "status: stable",
            "classification: compatibility_alias",
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
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.",
            "function: isr",
            "surface_form: @irq_handler",
            "feature: irq_handler_alias",
            "status: experimental",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.",
            "function: legacy_isr",
            "surface_form: @irq_legacy",
            "feature: irq_legacy_alias",
            "status: deprecated",
            "classification: deprecated_alias",
            "enabled_under_surface: false",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq_legacy` with `@ctx(irq)`.",
            "function: worker",
            "surface_form: @thread_entry",
            "feature: thread_entry_alias",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(thread)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.",
        ]
    );
}

#[test]
fn migrate_preview_legacy_unary_stable_output_is_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
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
            "migrations: 5",
            "function: alloc_worker",
            "surface_form: @alloc",
            "feature: legacy_alloc_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(alloc)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.",
            "function: block_worker",
            "surface_form: @block",
            "feature: legacy_block_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@block` with `@eff(block)`.",
            "function: irq_entry",
            "surface_form: @irq",
            "feature: legacy_irq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.",
            "function: noirq_worker",
            "surface_form: @noirq",
            "feature: legacy_noirq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(thread, boot)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
            "function: preempt_guarded",
            "surface_form: @preempt_off",
            "feature: legacy_preempt_off_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(preempt_off)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
        ]
    );
}

#[test]
fn migrate_preview_legacy_unary_experimental_output_is_exact() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
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
            "migrations: 5",
            "function: alloc_worker",
            "surface_form: @alloc",
            "feature: legacy_alloc_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(alloc)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.",
            "function: block_worker",
            "surface_form: @block",
            "feature: legacy_block_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(block)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@block` with `@eff(block)`.",
            "function: irq_entry",
            "surface_form: @irq",
            "feature: legacy_irq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(irq)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.",
            "function: noirq_worker",
            "surface_form: @noirq",
            "feature: legacy_noirq_context_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @ctx(thread, boot)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.",
            "function: preempt_guarded",
            "surface_form: @preempt_off",
            "feature: legacy_preempt_off_effect_shorthand",
            "status: stable",
            "classification: compatibility_alias",
            "enabled_under_surface: true",
            "canonical_replacement: @eff(preempt_off)",
            "migration_safe: true",
            "rewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.",
        ]
    );
}

#[test]
fn migrate_preview_canonical_edits_json_reports_legacy_unary_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 5,\n  \"edits\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@alloc` with `@eff(alloc)`.\"\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@noirq` with `@ctx(thread, boot)`.\"\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_reports_experimental_aliases_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("experimental")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"experimental\",\n  \"edits_count\": 3,\n  \"edits\": [\n    {\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@may_block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq_handler` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@thread_entry` with `@ctx(thread)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 0,\n  \"edits\": []\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_requires_json_format() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: --canonical-edits requires --format json")
    );
}

#[test]
fn migrate_preview_rejects_format_without_canonical_edits() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: --format is only supported with --canonical-edits")
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
fn canonical_edit_plan_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_edit_plan_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_edit_plan_v1",
            "surface": "stable",
            "edits_count": 0,
            "edits": []
        }),
        json!({
            "schema_version": "kernrift_canonical_edit_plan_v1",
            "surface": "experimental",
            "edits_count": 1,
            "edits": [{
                "function": "worker",
                "classification": "compatibility_alias",
                "surface_form": "@thread_entry",
                "canonical_replacement": "@ctx(thread)",
                "migration_safe": true,
                "rewrite_intent": "Replace the attribute token `@thread_entry` with `@ctx(thread)`."
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical edit plan JSON must validate against canonical edit plan v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}

#[test]
fn canonical_fix_result_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_fix_result_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_fix_result_v1",
            "surface": "stable",
            "file": "basic.kr",
            "rewrites_applied": 0,
            "changed": false,
            "rewrites": []
        }),
        json!({
            "schema_version": "kernrift_canonical_fix_result_v1",
            "surface": "experimental",
            "file": "aliases.kr",
            "rewrites_applied": 1,
            "changed": true,
            "rewrites": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical fix result JSON must validate against canonical fix result v1 schema: {}",
                details.join(" | ")
            );
        }
    }
}

#[test]
fn canonical_fix_preview_json_schema_accepts_empty_and_nonempty_reports() {
    let compiled = compile_canonical_fix_preview_schema();
    for instance in [
        json!({
            "schema_version": "kernrift_canonical_fix_preview_v1",
            "surface": "stable",
            "file": "fixture.kr",
            "rewrites_planned": 0,
            "would_change": false,
            "rewrites": []
        }),
        json!({
            "schema_version": "kernrift_canonical_fix_preview_v1",
            "surface": "experimental",
            "file": "fixture.kr",
            "rewrites_planned": 1,
            "would_change": true,
            "rewrites": [{
                "function": "helper",
                "classification": "compatibility_alias",
                "surface_form": "@irq_handler",
                "canonical_replacement": "@ctx(irq)",
                "migration_safe": true
            }]
        }),
    ] {
        if let Err(errors) = compiled.validate(&instance) {
            let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            panic!(
                "canonical fix preview JSON must validate against canonical fix preview v1 schema: {}",
                details.join(" | ")
            );
        }
    }
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
fn check_accepts_typed_mmio_statement_fixture() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("mmio_typed.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_trailing_comma_canonical_fact_lists_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("fact_trailing_commas.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_rejects_malformed_trailing_comma_fact_list_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("fact_trailing_comma_malformed.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "@ctx(...) contains an empty context entry for 'entry' at 1:1",
            "  1 | @ctx(thread,, boot)",
        ],
    );
}

#[test]
fn check_rejects_invalid_typed_mmio_element_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_type.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "unsupported mmio element type 'u128'; expected one of: u8, u16, u32, u64 at 2:3",
            "  2 |   mmio_read<u128>(mmio_base);",
        ],
        "unexpected diagnostic: {}",
        stderr
    );
}

#[test]
fn check_rejects_invalid_typed_mmio_arity_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_arity.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "mmio_write<T>(addr, value) requires exactly two arguments: address and value at 2:3",
            "  2 |   mmio_write<u32>(mmio_base);",
        ],
        "unexpected diagnostic: {}",
        stderr
    );
}

#[test]
fn check_rejects_invalid_typed_mmio_operand_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_operand.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "unsupported mmio address operand 'a + b'; expected identifier, integer literal, or identifier + integer literal at 2:3",
            "  2 |   mmio_read<u32>(a + b);",
        ],
        "unexpected diagnostic: {}",
        stderr
    );
}

#[test]
fn check_accepts_declared_mmio_base_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_declared_base.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_rejects_undeclared_mmio_base_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_undeclared_base.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("undeclared mmio base 'UART0' used in mmio_read<u32>(UART0)")
    );
}

#[test]
fn check_rejects_invalid_mmio_base_declaration_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_invalid_declaration.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "invalid mmio base declaration for 'UART0': expected integer literal at 1:23",
            "  1 | mmio UART0 = BASE + 4;",
        ],
        "unexpected diagnostic: {}",
        stderr
    );
}

#[test]
fn check_accepts_mmio_register_declared_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_register_declared.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_mmio_register_base_zero_symbolic_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_reg_base_zero_declared.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_mmio_register_mixed_offset_literal_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_reg_offset_mixed_literal.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_mmio_register_absolute_literal_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_reg_absolute_literal_declared.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_mmio_raw_literal_with_opt_in_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_reg_raw_literal_opt_in.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_raw_mmio_with_opt_in_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_accepts_raw_mmio_bypass_register_checks_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_bypass_register_checks.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn check_rejects_symbolic_mmio_base_without_offset_zero_register_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_base_zero_missing.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("undeclared mmio register offset '0' for base 'UART0'")
    );
}

#[test]
fn check_rejects_undeclared_mmio_register_offset_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_undeclared_offset.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("undeclared mmio register offset '0x44' for base 'UART0'")
    );
}

#[test]
fn check_rejects_symbolic_mmio_base_register_access_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_base_zero_access_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(UART0, x) violates register access: 'UART0.SR' is read-only")
    );
}

#[test]
fn check_rejects_symbolic_mmio_base_register_width_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_base_zero_width_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(UART0, x) width mismatch: register 'UART0.CR' is u16")
    );
}

#[test]
fn check_rejects_duplicate_mmio_register_semantic_offset_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_duplicate_semantic_offset.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("duplicate mmio register offset '0x04' for base 'UART0'")
    );
}

#[test]
fn check_rejects_duplicate_mmio_register_absolute_address_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_duplicate_absolute_address.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("duplicate mmio register absolute address '0x1004' between 'A.R0' and 'B.R1'")
    );
}

#[test]
fn check_rejects_mmio_register_access_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_access_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(UART0 + 0x04, x) violates register access: 'UART0.SR' is read-only")
    );
}

#[test]
fn check_rejects_mmio_register_absolute_literal_access_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_absolute_literal_access_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(0x1004, x) violates register access: 'UART0.SR' is read-only")
    );
}

#[test]
fn check_rejects_mmio_register_width_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_width_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(UART0 + 0x08, x) width mismatch: register 'UART0.CR' is u16")
    );
}

#[test]
fn check_rejects_mmio_register_absolute_literal_width_mismatch_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_absolute_literal_width_mismatch.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("mmio_write<u32>(0x1008, x) width mismatch: register 'UART0.CR' is u16")
    );
}

#[test]
fn check_rejects_mmio_raw_literal_without_opt_in_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_raw_literal_without_opt_in.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some(
            "unresolved raw mmio address '0x1014'; declare a matching mmio_reg or enable raw mmio access"
        )
    );
}

#[test]
fn check_rejects_raw_mmio_without_opt_in_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("raw_mmio_without_cap.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("raw_mmio_write<u32>(0x1014, x) requires @module_caps(MmioRaw)")
    );
}

#[test]
fn check_rejects_mmio_register_with_undeclared_base_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_undeclared_base.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("undeclared mmio base 'UART0' in register declaration 'UART0.DR'")
    );
}

#[test]
fn check_rejects_invalid_mmio_register_declaration_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_fail")
        .join("mmio_reg_invalid_declaration.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root).arg("check").arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "invalid mmio register declaration for 'UART0.DR': expected integer literal offset at 2:31",
            "  2 | mmio_reg UART0.DR = BASE + 4 : u32 rw;",
        ],
        "unexpected diagnostic: {}",
        stderr
    );
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
    assert!(json["report"]["effects"]["raw_mmio_sites_count"].is_u64());
    assert!(json["report"]["critical"]["depth_max"].is_u64());
    assert!(json["report"]["critical"]["violations"].is_array());

    fs::remove_file(&out_path).ok();
}

#[test]
fn contracts_v2_raw_mmio_reporting_distinguishes_raw_and_structured_usage() {
    let root = repo_root();
    let raw_fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let structured_fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_reg_offset_mixed_literal.kr");
    let mixed_fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_mixed_structured_raw.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let raw_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-raw-only-{}.json", ts));
    let structured_out =
        std::env::temp_dir().join(format!("kernrift-contracts-v2-structured-only-{}.json", ts));
    let mixed_out = std::env::temp_dir().join(format!("kernrift-contracts-v2-mixed-{}.json", ts));
    fs::remove_file(&raw_out).ok();
    fs::remove_file(&structured_out).ok();
    fs::remove_file(&mixed_out).ok();

    for (fixture, out) in [
        (&raw_fixture, &raw_out),
        (&structured_fixture, &structured_out),
        (&mixed_fixture, &mixed_out),
    ] {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root)
            .arg("check")
            .arg("--contracts-schema")
            .arg("v2")
            .arg("--contracts-out")
            .arg(out.as_os_str())
            .arg(fixture.as_os_str());
        cmd.assert().success();
    }

    let raw_json: Value =
        serde_json::from_str(&fs::read_to_string(&raw_out).expect("raw json")).expect("raw value");
    let structured_json: Value =
        serde_json::from_str(&fs::read_to_string(&structured_out).expect("structured json"))
            .expect("structured value");
    let mixed_json: Value =
        serde_json::from_str(&fs::read_to_string(&mixed_out).expect("mixed json"))
            .expect("mixed value");
    validate_contracts_schema_v2(&raw_json);
    validate_contracts_schema_v2(&structured_json);
    validate_contracts_schema_v2(&mixed_json);

    let raw_entry = raw_json["facts"]["symbols"]
        .as_array()
        .expect("raw symbols")
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("raw entry");
    let structured_entry = structured_json["facts"]["symbols"]
        .as_array()
        .expect("structured symbols")
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("structured entry");
    let mixed_entry = mixed_json["facts"]["symbols"]
        .as_array()
        .expect("mixed symbols")
        .iter()
        .find(|sym| sym["name"] == "entry")
        .expect("mixed entry");

    assert_eq!(raw_entry["raw_mmio_used"], Value::Bool(true));
    assert_eq!(
        raw_entry["raw_mmio_sites_count"],
        Value::Number(1_u64.into())
    );
    assert_eq!(
        raw_json["report"]["effects"]["raw_mmio_sites_count"],
        Value::Number(1_u64.into())
    );

    assert_eq!(structured_entry["raw_mmio_used"], Value::Bool(false));
    assert_eq!(
        structured_entry["raw_mmio_sites_count"],
        Value::Number(0_u64.into())
    );
    assert_eq!(
        structured_json["report"]["effects"]["raw_mmio_sites_count"],
        Value::Number(0_u64.into())
    );

    assert_eq!(mixed_entry["raw_mmio_used"], Value::Bool(true));
    assert_eq!(
        mixed_entry["raw_mmio_sites_count"],
        Value::Number(1_u64.into())
    );
    assert!(
        mixed_entry["eff_used"]
            .as_array()
            .expect("mixed eff_used")
            .contains(&Value::String("mmio".to_string())),
        "mixed fixture should preserve ordinary mmio effect signal"
    );
    assert_eq!(
        mixed_json["report"]["effects"]["raw_mmio_sites_count"],
        Value::Number(1_u64.into())
    );

    fs::remove_file(&raw_out).ok();
    fs::remove_file(&structured_out).ok();
    fs::remove_file(&mixed_out).ok();
}

#[test]
fn inspect_contracts_v2_summary_includes_raw_mmio_signals() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_mixed_structured_raw.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-summary");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect")
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let lines = stdout.lines().collect::<Vec<_>>();

    assert!(
        lines.contains(&"raw_mmio_symbols: 1 [entry]"),
        "expected raw_mmio symbol summary line, got:\n{}",
        stdout
    );
    assert!(
        lines.contains(&"raw_mmio_sites_count: 1"),
        "expected raw_mmio site count summary line, got:\n{}",
        stdout
    );

    fs::remove_file(&contracts_path).ok();
}

#[test]
fn policy_denies_raw_mmio_by_default_when_configured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-deny");
    let policy_path = write_temp_policy_file("raw-mmio-deny", "[kernel]\nallow_raw_mmio = false\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_FORBID: raw_mmio is not allowed"]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_raw_mmio_when_explicitly_enabled() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-allow");
    let policy_path = write_temp_policy_file("raw-mmio-allow", "[kernel]\nallow_raw_mmio = true\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_structured_mmio_when_raw_mmio_is_denied() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("mmio_register_declared.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-structured-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-structured-pass",
        "[kernel]\nallow_raw_mmio = false\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_raw_mmio_site_limit() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_bypass_register_checks.kr");
    let contracts_path = write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-site-limit");
    let policy_path = write_temp_policy_file(
        "raw-mmio-site-limit",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites = 1\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_RAW_MMIO_SITE_LIMIT: raw_mmio_sites_count 2 exceeds allowed maximum 1"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_raw_mmio_symbol_allowlist() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_two_symbols.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-symbol-allowlist");
    let policy_path = write_temp_policy_file(
        "raw-mmio-symbol-allowlist",
        "[kernel]\nallow_raw_mmio_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST: raw_mmio symbol 'helper' is not allowed"]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_non_irq_raw_mmio_when_irq_raw_mmio_is_forbidden() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-non-irq-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_raw_mmio_in_irq_context() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_structured_mmio_in_irq_when_only_irq_raw_mmio_is_forbidden() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-structured-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-structured-pass",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_reachable_helper_that_uses_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-helper");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-helper",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via helper)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_formats_irq_raw_mmio_forbid_with_multihop_contract_path() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-path-helper");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "helper"]
    }]);
    let contracts_path = write_temp_contracts_file("raw-mmio-irq-forbid-path-helper", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-path-helper",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry -> helper)"
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_forbid_is_structured_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-evidence-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-evidence-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry)",
            "evidence: symbol=entry",
            "evidence: irq_path=[entry]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_forbid_multihop_path_is_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-evidence-multihop");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path =
        write_temp_contracts_file("raw-mmio-irq-forbid-evidence-multihop", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-evidence-multihop",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_FORBID: raw_mmio is not allowed in irq context (via entry -> dispatch -> helper)",
            "evidence: symbol=helper",
            "evidence: irq_path=[entry,dispatch,helper]",
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_site_limit_ignores_non_irq_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_bypass_register_checks.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-non-irq-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_irq_raw_mmio_site_limit_for_direct_irq_usage() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-direct",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SITE_LIMIT: irq raw_mmio_sites_count 1 exceeds allowed maximum 0"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_enforces_irq_raw_mmio_site_limit_for_irq_reachable_helper() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-helper");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-helper",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SITE_LIMIT: irq raw_mmio_sites_count 1 exceeds allowed maximum 0"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_site_limit_does_not_affect_structured_irq_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-site-limit-structured");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-site-limit-structured",
        "[kernel]\nallow_raw_mmio = true\nmax_raw_mmio_sites_in_irq = 0\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_symbol_allowlist_ignores_non_irq_raw_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_two_symbols.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-non-irq-pass",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-non-irq-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"nobody\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_allows_irq_raw_mmio_when_symbol_is_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-symbol-allowlist-pass");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_raw_mmio_when_symbol_is_not_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-direct-deny",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-direct-deny",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"helper\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'entry' is not allowed (via entry)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_denies_irq_reachable_raw_mmio_helper_when_not_allowlisted() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-helper-deny",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-helper-deny",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via helper)"
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_formats_irq_raw_mmio_symbol_allowlist_with_deep_contract_path() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-symbol-allowlist-deep-path");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path =
        write_temp_contracts_file("raw-mmio-irq-symbol-allowlist-deep-path", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-deep-path",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via entry -> dispatch -> helper)"
        ]
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_evidence_irq_raw_mmio_symbol_allowlist_is_structured_and_deterministic() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-evidence-helper",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-evidence-helper",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--evidence")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        vec![
            "policy: KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST: irq raw_mmio symbol 'helper' is not allowed (via helper)",
            "evidence: symbol=helper",
            "evidence: irq_path=[helper]",
        ]
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_irq_raw_mmio_forbid_is_exact_and_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-forbid-json-direct");
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-forbid-json-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "json policy mode must not write stderr: {}",
        stderr
    );
    assert_json_transport(&stdout, &stderr, "kernrift_policy_violations_v1");
    assert_eq!(
        stdout,
        concat!(
            "{\n",
            "  \"schema_version\": \"kernrift_policy_violations_v1\",\n",
            "  \"result\": \"deny\",\n",
            "  \"exit_code\": 1,\n",
            "  \"violations\": [\n",
            "    {\n",
            "      \"rule\": \"KERNEL_IRQ_RAW_MMIO_FORBID\",\n",
            "      \"family\": \"effect\",\n",
            "      \"message\": \"raw_mmio is not allowed in irq context (via entry)\",\n",
            "      \"evidence\": [\n",
            "        {\n",
            "          \"kind\": \"scalar\",\n",
            "          \"key\": \"symbol\",\n",
            "          \"value\": \"entry\"\n",
            "        },\n",
            "        {\n",
            "          \"kind\": \"list\",\n",
            "          \"key\": \"irq_path\",\n",
            "          \"values\": [\n",
            "            \"entry\"\n",
            "          ]\n",
            "        }\n",
            "      ]\n",
            "    }\n",
            "  ]\n",
            "}\n"
        )
    );
    let json: Value = serde_json::from_str(&stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert_eq!(
        object_keys(&json),
        BTreeSet::from([
            "exit_code".to_string(),
            "result".to_string(),
            "schema_version".to_string(),
            "violations".to_string(),
        ]),
        "policy json envelope drifted"
    );
    let violation = json["violations"][0].as_object().expect("violation object");
    assert_eq!(
        violation.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "evidence".to_string(),
            "family".to_string(),
            "message".to_string(),
            "rule".to_string(),
        ]),
        "policy json violation shape drifted"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_irq_raw_mmio_allowlist_deep_path_is_exact_and_structured() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let emitted_contracts =
        write_v2_contracts_for_fixture(&root, &fixture, "raw-mmio-irq-allowlist-json-deep");
    let mut contracts: Value =
        serde_json::from_str(&fs::read_to_string(&emitted_contracts).expect("contracts text"))
            .expect("contracts json");
    let helper = contracts["facts"]["symbols"]
        .as_array_mut()
        .expect("facts symbols")
        .iter_mut()
        .find(|sym| sym["name"] == "helper")
        .expect("helper symbol");
    helper["ctx_path_provenance"] = json!([{
        "ctx": "irq",
        "path": ["entry", "dispatch", "helper"]
    }]);
    let contracts_path = write_temp_contracts_file("raw-mmio-irq-allowlist-json-deep", &contracts);
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-allowlist-json-deep",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "json policy mode must not write stderr: {}",
        stderr
    );
    assert_json_transport(&stdout, &stderr, "kernrift_policy_violations_v1");
    assert_eq!(
        stdout,
        concat!(
            "{\n",
            "  \"schema_version\": \"kernrift_policy_violations_v1\",\n",
            "  \"result\": \"deny\",\n",
            "  \"exit_code\": 1,\n",
            "  \"violations\": [\n",
            "    {\n",
            "      \"rule\": \"KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST\",\n",
            "      \"family\": \"effect\",\n",
            "      \"message\": \"irq raw_mmio symbol 'helper' is not allowed (via entry -> dispatch -> helper)\",\n",
            "      \"evidence\": [\n",
            "        {\n",
            "          \"kind\": \"scalar\",\n",
            "          \"key\": \"symbol\",\n",
            "          \"value\": \"helper\"\n",
            "        },\n",
            "        {\n",
            "          \"kind\": \"list\",\n",
            "          \"key\": \"irq_path\",\n",
            "          \"values\": [\n",
            "            \"entry\",\n",
            "            \"dispatch\",\n",
            "            \"helper\"\n",
            "          ]\n",
            "        }\n",
            "      ]\n",
            "    }\n",
            "  ]\n",
            "}\n"
        )
    );
    let json: Value = serde_json::from_str(&stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    let evidence = json["violations"][0]["evidence"]
        .as_array()
        .expect("evidence array");
    assert_eq!(
        evidence[0]["kind"],
        json!("scalar"),
        "scalar evidence variant drifted"
    );
    assert_eq!(
        evidence[1]["kind"],
        json!("list"),
        "list evidence variant drifted"
    );

    fs::remove_file(&emitted_contracts).ok();
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_json_schema_accepts_scalar_list_and_empty_list_evidence_variants() {
    let compiled = compile_policy_violations_schema();

    let scalar_and_list = json!({
        "schema_version": "kernrift_policy_violations_v1",
        "result": "deny",
        "exit_code": 1,
        "violations": [{
            "rule": "KERNEL_IRQ_RAW_MMIO_SYMBOL_ALLOWLIST",
            "family": "effect",
            "message": "irq raw_mmio symbol 'helper' is not allowed (via entry -> helper)",
            "evidence": [
                { "kind": "scalar", "key": "symbol", "value": "helper" },
                { "kind": "list", "key": "irq_path", "values": ["entry", "helper"] }
            ]
        }]
    });
    if let Err(errors) = compiled.validate(&scalar_and_list) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "scalar/list policy evidence must validate against schema: {}",
            details.join(" | ")
        );
    }

    let empty_list = json!({
        "schema_version": "kernrift_policy_violations_v1",
        "result": "deny",
        "exit_code": 1,
        "violations": [{
            "rule": "KERNEL_RAW_MMIO_SYMBOL_ALLOWLIST",
            "family": "effect",
            "message": "raw_mmio symbol 'helper' is not allowed",
            "evidence": [
                { "kind": "list", "key": "irq_path", "values": [] }
            ]
        }]
    });
    if let Err(errors) = compiled.validate(&empty_list) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "empty-list policy evidence must validate against schema: {}",
            details.join(" | ")
        );
    }
}

#[test]
fn check_json_policy_irq_raw_mmio_forbid_matches_policy_json_contract_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_direct.kr");
    let contracts_path = unique_temp_output_path("check-policy-json-direct-contracts", "json");
    let denied_contracts_path =
        unique_temp_output_path("check-policy-json-direct-denied-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-json-direct",
        "[kernel]\nallow_raw_mmio = true\nforbid_raw_mmio_in_irq = true\n",
    );
    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&denied_contracts_path).ok();

    let mut emit_cmd: Command = cargo_bin_cmd!("kernriftc");
    emit_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    emit_cmd.assert().success();

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let policy_assert = policy_cmd.assert().failure().code(1);
    let policy_stdout =
        String::from_utf8(policy_assert.get_output().stdout.clone()).expect("policy stdout utf8");
    let policy_stderr =
        String::from_utf8(policy_assert.get_output().stderr.clone()).expect("policy stderr utf8");
    assert!(
        policy_stderr.is_empty(),
        "policy json mode must not write stderr: {}",
        policy_stderr
    );

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-out")
        .arg(denied_contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    let check_assert = check_cmd.assert().failure().code(1);
    let check_stdout =
        String::from_utf8(check_assert.get_output().stdout.clone()).expect("check stdout utf8");
    let check_stderr =
        String::from_utf8(check_assert.get_output().stderr.clone()).expect("check stderr utf8");
    assert!(
        check_stderr.is_empty(),
        "check json policy deny must not write stderr: {}",
        check_stderr
    );
    assert_json_transport(
        &check_stdout,
        &check_stderr,
        "kernrift_policy_violations_v1",
    );
    assert_eq!(
        check_stdout, policy_stdout,
        "check json policy deny must reuse exact policy JSON envelope"
    );
    let json: Value = serde_json::from_str(&check_stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert!(
        !denied_contracts_path.exists(),
        "contracts output should not be written when policy denies in json mode"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&denied_contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_json_policy_irq_raw_mmio_allowlist_helper_path_matches_policy_json_contract_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_irq_helper.kr");
    let contracts_path = unique_temp_output_path("check-policy-json-helper-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-json-helper",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );
    fs::remove_file(&contracts_path).ok();

    let mut emit_cmd: Command = cargo_bin_cmd!("kernriftc");
    emit_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    emit_cmd.assert().success();

    let mut policy_cmd: Command = cargo_bin_cmd!("kernriftc");
    policy_cmd
        .current_dir(&root)
        .arg("policy")
        .arg("--format")
        .arg("json")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    let policy_assert = policy_cmd.assert().failure().code(1);
    let policy_stdout =
        String::from_utf8(policy_assert.get_output().stdout.clone()).expect("policy stdout utf8");
    let policy_stderr =
        String::from_utf8(policy_assert.get_output().stderr.clone()).expect("policy stderr utf8");
    assert!(
        policy_stderr.is_empty(),
        "policy json mode must not write stderr: {}",
        policy_stderr
    );

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let check_assert = check_cmd.assert().failure().code(1);
    let check_stdout =
        String::from_utf8(check_assert.get_output().stdout.clone()).expect("check stdout utf8");
    let check_stderr =
        String::from_utf8(check_assert.get_output().stderr.clone()).expect("check stderr utf8");
    assert!(
        check_stderr.is_empty(),
        "check json policy deny must not write stderr: {}",
        check_stderr
    );
    assert_json_transport(
        &check_stdout,
        &check_stderr,
        "kernrift_policy_violations_v1",
    );
    assert_eq!(
        check_stdout, policy_stdout,
        "check json policy deny must reuse exact policy JSON envelope"
    );
    let json: Value = serde_json::from_str(&check_stdout).expect("policy json");
    validate_policy_violations_schema(&json);
    assert_eq!(
        json["violations"][0]["message"],
        json!("irq raw_mmio symbol 'helper' is not allowed (via helper)")
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn policy_irq_raw_mmio_symbol_allowlist_does_not_affect_structured_irq_mmio() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("irq_ctx_chain.kr");
    let contracts_path = write_v2_contracts_for_fixture(
        &root,
        &fixture,
        "raw-mmio-irq-symbol-allowlist-structured-pass",
    );
    let policy_path = write_temp_policy_file(
        "raw-mmio-irq-symbol-allowlist-structured-pass",
        "[kernel]\nallow_raw_mmio = true\nallow_raw_mmio_in_irq_symbols = [\"entry\"]\n",
    );

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("policy")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts")
        .arg(contracts_path.as_os_str());
    cmd.assert().success();

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
}

#[test]
fn check_with_policy_denies_raw_mmio_and_suppresses_contract_output() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("raw_mmio_with_cap.kr");
    let contracts_path = unique_temp_output_path("check-policy-raw-mmio-deny-contracts", "json");
    let policy_path = write_temp_policy_file(
        "check-policy-raw-mmio-deny",
        "[kernel]\nallow_raw_mmio = false\n",
    );
    fs::remove_file(&contracts_path).ok();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg("--contracts-schema")
        .arg("v2")
        .arg("--contracts-out")
        .arg(contracts_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec!["policy: KERNEL_RAW_MMIO_FORBID: raw_mmio is not allowed"]
    );
    assert!(
        !contracts_path.exists(),
        "contracts output should not be written when raw mmio policy denies"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&policy_path).ok();
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
    assert_eq!(
        entry["ctx_provenance"],
        json!([{
            "ctx": "irq",
            "sources": ["entry"]
        }])
    );
    assert_eq!(
        entry["ctx_path_provenance"],
        json!([{
            "ctx": "irq",
            "path": ["entry"]
        }])
    );
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

fn compile_policy_violations_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(POLICY_VIOLATIONS_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn compile_canonical_findings_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FINDINGS_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn compile_canonical_edit_plan_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_EDIT_PLAN_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn compile_canonical_fix_result_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FIX_RESULT_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn compile_canonical_fix_preview_schema() -> JSONSchema {
    let schema_json: Value =
        serde_json::from_str(CANONICAL_FIX_PREVIEW_SCHEMA_V1).expect("schema json");
    JSONSchema::compile(&schema_json).expect("compile schema")
}

fn validate_policy_violations_schema(instance: &Value) {
    let compiled = compile_policy_violations_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "policy JSON must validate against policy violations v1 schema: {}",
            details.join(" | ")
        );
    }
}

fn validate_canonical_findings_schema(instance: &Value) {
    let compiled = compile_canonical_findings_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical findings JSON must validate against canonical findings v1 schema: {}",
            details.join(" | ")
        );
    }
}

fn validate_canonical_edit_plan_schema(instance: &Value) {
    let compiled = compile_canonical_edit_plan_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical edit plan JSON must validate against canonical edit plan v1 schema: {}",
            details.join(" | ")
        );
    }
}

fn validate_canonical_fix_result_schema(instance: &Value) {
    let compiled = compile_canonical_fix_result_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical fix result JSON must validate against canonical fix result v1 schema: {}",
            details.join(" | ")
        );
    }
}

fn validate_canonical_fix_preview_schema(instance: &Value) {
    let compiled = compile_canonical_fix_preview_schema();
    if let Err(errors) = compiled.validate(instance) {
        let details = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!(
            "canonical fix preview JSON must validate against canonical fix preview v1 schema: {}",
            details.join(" | ")
        );
    }
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

#[test]
fn verify_report_signature_pass_is_stable_and_path_stripped() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("must_pass")
        .join("callee_acquires_lock.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-{}.report.json", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-pass-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
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
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");

    assert_eq!(report_json["result"], Value::String("pass".to_string()));
    assert_eq!(report_json["signature"]["checked"], Value::Bool(true));
    assert_eq!(report_json["signature"]["valid"], Value::Bool(true));
    assert_eq!(
        report_json["diagnostics"],
        Value::Array(vec![]),
        "verify report diagnostics should be empty on signature pass"
    );

    let sig_name = sig_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("sig basename");
    let pubkey_name = pubkey_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("pubkey basename");
    let report_sig_path = report_json["inputs"]["sig"]
        .as_str()
        .expect("report sig path");
    let report_pubkey_path = report_json["inputs"]["pubkey"]
        .as_str()
        .expect("report pubkey path");
    assert_eq!(report_sig_path, sig_name);
    assert_eq!(report_pubkey_path, pubkey_name);
    assert!(
        !report_sig_path.contains('/'),
        "verify report should strip absolute signature path"
    );
    assert!(
        !report_pubkey_path.contains('/'),
        "verify report should strip absolute pubkey path"
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn verify_report_signature_mismatch_records_deny_deterministically() {
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
        std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-contracts-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-sig-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-report-{}.json", ts));
    let secret_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-secret-{}.hex", ts));
    let pubkey_path = std::env::temp_dir().join(format!("kernrift-vrf-sig-deny-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
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
        &sig_path,
        format!("{}\n", BASE64_STANDARD.encode([0_u8; 64])),
    )
    .expect("tamper signature");

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
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().failure().code(1);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(report_json["result"], Value::String("deny".to_string()));
    assert_eq!(report_json["hash"]["matched"], Value::Bool(true));
    assert_eq!(report_json["signature"]["checked"], Value::Bool(true));
    assert_eq!(report_json["signature"]["valid"], Value::Bool(false));

    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with("verify: SIG_MISMATCH:"),
        "unexpected diagnostics: {:?}",
        diagnostics
    );

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}

#[test]
fn verify_report_invalid_input_normalizes_diagnostic_paths() {
    let root = repo_root();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let contracts_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.json", ts));
    let hash_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.sha256", ts));
    let report_path = std::env::temp_dir().join(format!("kernrift-vrf-missing-{}.report.json", ts));

    fs::remove_file(&contracts_path).ok();
    fs::write(
        &hash_path,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("write hash");

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
    verify_cmd.assert().failure().code(2);

    let report_text = fs::read_to_string(&report_path).expect("read verify report");
    let report_json: Value = serde_json::from_str(&report_text).expect("verify report json");
    assert_eq!(
        report_json["result"],
        Value::String("invalid_input".to_string())
    );
    assert_eq!(report_json["signature"]["checked"], Value::Bool(false));
    assert_eq!(report_json["signature"]["valid"], Value::Null);
    assert_eq!(report_json["hash"]["matched"], Value::Bool(false));

    let contracts_name = contracts_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("contracts basename");
    let report_contracts_path = report_json["inputs"]["contracts"]
        .as_str()
        .expect("report contracts path");
    assert_eq!(report_contracts_path, contracts_name);

    let diagnostics = report_json["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|v| v.as_str().expect("diag string").to_string())
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].starts_with(&format!("failed to read contracts '{}':", contracts_name)),
        "unexpected diagnostics: {:?}",
        diagnostics
    );
    assert!(
        !diagnostics[0].contains('/'),
        "diagnostic path should be normalized to basename"
    );

    fs::remove_file(&hash_path).ok();
    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_generated_signature_pass_summary_is_exact() {
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
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-contracts-{}.json", ts));
    let hash_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-hash-{}.sha256", ts));
    let sig_path = std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-sig-{}.sig", ts));
    let report_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-report-{}.json", ts));
    let secret_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-secret-{}.hex", ts));
    let pubkey_path =
        std::env::temp_dir().join(format!("kernrift-inspect-sig-pass-pubkey-{}.hex", ts));

    write_test_keypair(&secret_path, &pubkey_path);

    let mut check_cmd: Command = cargo_bin_cmd!("kernriftc");
    check_cmd
        .current_dir(&root)
        .arg("check")
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
        .arg(pubkey_path.as_os_str())
        .arg("--report")
        .arg(report_path.as_os_str());
    verify_cmd.assert().success();

    let mut inspect_cmd: Command = cargo_bin_cmd!("kernriftc");
    inspect_cmd
        .current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str());
    let assert = inspect_cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let lines = stdout.lines().collect::<Vec<_>>();

    let sig_name = sig_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("sig basename");
    let pubkey_name = pubkey_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("pubkey basename");

    assert_eq!(lines[0], "schema: kernrift_verify_report_v1");
    assert_eq!(lines[1], "result: pass");
    assert!(lines.contains(&format!("sig: {}", sig_name).as_str()));
    assert!(lines.contains(&format!("pubkey: {}", pubkey_name).as_str()));
    assert!(lines.contains(&"checked: true"));
    assert!(lines.contains(&"valid: true"));
    assert!(lines.contains(&"diagnostics: 0"));

    fs::remove_file(&contracts_path).ok();
    fs::remove_file(&hash_path).ok();
    fs::remove_file(&sig_path).ok();
    fs::remove_file(&report_path).ok();
    fs::remove_file(&secret_path).ok();
    fs::remove_file(&pubkey_path).ok();
}
