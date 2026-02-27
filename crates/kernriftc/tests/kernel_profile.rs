use std::path::{Path, PathBuf};

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
        "critical_yield.kr",
        "irq_alloc_effect.kr",
        "noyield_not_critical.kr",
    ] {
        let path = root.join("tests").join("kernel_profile").join(fixture);
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("check").arg(path.as_os_str());
        cmd.assert().success();
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
        .join("critical_yield.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("policy: KERNEL_CRITICAL_YIELD:"),
        "expected KERNEL_CRITICAL_YIELD violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_noyield_marker_is_not_treated_as_critical() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("noyield_not_critical.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--profile")
        .arg("kernel")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        !stderr.contains("policy: KERNEL_CRITICAL_YIELD:"),
        "did not expect KERNEL_CRITICAL_YIELD violation, got:\n{}",
        stderr
    );
}

#[test]
fn kernel_profile_denies_alloc_in_irq() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("kernel_profile")
        .join("irq_alloc_effect.kr");

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
