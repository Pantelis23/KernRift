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
