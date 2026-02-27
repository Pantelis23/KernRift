use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn default_profile_behavior_is_unchanged() {
    let root = repo_root();

    for fixture in [
        "deny_unbounded_no_yield.kr",
        "deny_lock_depth_and_order.kr",
        "critical_region_yield.kr",
        "critical_region_alloc.kr",
        "critical_region_block.kr",
        "critical_region_balanced.kr",
        "policy_families_order.kr",
        "irq_alloc_effect.kr",
        "irq_alloc_site.kr",
        "irq_block_site.kr",
        "irq_caps_transitive.kr",
        "irq_caps_unlisted.kr",
        "irq_caps_extern.kr",
    ] {
        let path = root.join("tests").join("kernel_profile").join(fixture);
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("check").arg(path.as_os_str());
        cmd.assert().success();
    }

    for fixture in [
        "irq_alloc_transitive.kr",
        "irq_block_transitive.kr",
        "irq_alloc_extern_stub.kr",
        "irq_block_extern_stub.kr",
    ] {
        let path = root.join("tests").join("kernel_profile").join(fixture);
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("check").arg(path.as_os_str());
        cmd.assert().failure().code(1);
    }
}

#[test]
fn kernel_profile_denies_unbounded_no_yield() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("deny_unbounded_no_yield.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert!(
        lines.iter().any(|line| line.contains("NO_YIELD_UNBOUNDED")),
        "expected NO_YIELD_UNBOUNDED violation, got: {lines:?}"
    );
}

#[test]
fn kernel_profile_denies_lock_depth_and_order() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("deny_lock_depth_and_order.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("policy: "))
        .collect::<Vec<_>>();
    assert!(
        lines
            .iter()
            .any(|line| line == &"policy: LIMIT_MAX_LOCK_DEPTH: max_lock_depth 2 exceeds limit 1"),
        "expected LIMIT_MAX_LOCK_DEPTH violation, got: {lines:?}"
    );
    assert!(
        lines.iter().any(
            |line| line
                == &"policy: LOCK_FORBID_EDGE: forbidden lock edge 'ConsoleLock -> SchedLock' is present"
        ),
        "expected LOCK_FORBID_EDGE violation, got: {lines:?}"
    );
}

#[test]
fn kernel_profile_allows_minimal_fixture() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("pass_minimal.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn kernel_profile_rejects_unknown_profile_name() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("pass_minimal.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("nope")
        .arg(fixture.as_os_str());
    cmd.assert().failure().code(2);
}

#[test]
fn kernel_profile_denies_yield_in_critical() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_yield.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_CRITICAL_REGION_YIELD:"),
        "expected KERNEL_CRITICAL_REGION_YIELD violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_alloc_in_critical_region() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_alloc.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_CRITICAL_REGION_ALLOC:"),
        "expected KERNEL_CRITICAL_REGION_ALLOC violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_block_in_critical_region() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_block.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_CRITICAL_REGION_BLOCK:"),
        "expected KERNEL_CRITICAL_REGION_BLOCK violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_allows_balanced_critical_region() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("critical_region_balanced.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    cmd.assert().success();
}

#[test]
fn kernel_profile_denies_alloc_in_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_site.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_ALLOC:"),
        "expected KERNEL_IRQ_ALLOC violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_alloc_in_irq_transitive() {
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
        stderr.contains("policy: KERNEL_IRQ_ALLOC:"),
        "expected KERNEL_IRQ_ALLOC violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_block_in_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_block_site.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_BLOCK:"),
        "expected KERNEL_IRQ_BLOCK violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_block_in_irq_transitive() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_block_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_BLOCK:"),
        "expected KERNEL_IRQ_BLOCK violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_does_not_emit_irq_yield_without_policy() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_yield_transitive.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        !stderr.contains("policy: KERNEL_IRQ_YIELD:"),
        "did not expect KERNEL_IRQ_YIELD without explicit policy knob, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_custom_policy_denies_yield_in_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_yield_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-kernel-yield-irq-policy-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_yield_in_irq = true
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_YIELD:"),
        "expected KERNEL_IRQ_YIELD violation, got:\n{}",
        stderr
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn kernel_profile_custom_policy_denies_caps_in_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-kernel-cap-policy-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_CAP_FORBID:"),
        "expected KERNEL_IRQ_CAP_FORBID violation, got:\n{}",
        stderr
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn kernel_profile_custom_policy_allow_caps_in_irq_overrides_forbid() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_transitive.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-kernel-cap-allow-overrides-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    fs::remove_file(&policy_path).ok();
}

#[test]
fn kernel_profile_custom_policy_non_listed_capability_is_allowed() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_unlisted.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path =
        std::env::temp_dir().join(format!("kernrift-kernel-cap-non-listed-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
allow_caps_in_irq = []
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    cmd.assert().success();

    fs::remove_file(&policy_path).ok();
}

#[test]
fn kernel_profile_custom_policy_denies_caps_in_irq_via_extern() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_caps_extern.kr");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let policy_path = std::env::temp_dir().join(format!("kernrift-kernel-cap-extern-{}.toml", ts));
    fs::write(
        &policy_path,
        r#"
[kernel]
forbid_caps_in_irq = ["PhysMap"]
"#,
    )
    .expect("write policy");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg("--policy")
        .arg(policy_path.as_os_str())
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_CAP_FORBID:"),
        "expected KERNEL_IRQ_CAP_FORBID violation via extern capability propagation, got:\n{}",
        stderr
    );

    fs::remove_file(&policy_path).ok();
}

#[test]
fn kernel_profile_denies_alloc_in_irq_via_extern_eff_stub() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_extern_stub.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_ALLOC:"),
        "expected KERNEL_IRQ_ALLOC violation via extern effect stub, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_block_in_irq_via_extern_eff_stub() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_block_extern_stub.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_IRQ_BLOCK:"),
        "expected KERNEL_IRQ_BLOCK violation via extern effect stub, got:\n{}",
        stderr
    );
}
