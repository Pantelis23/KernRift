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
fn migrate_preview_surface_omitted_surface_matches_explicit_stable() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_aliases.kr");

    let run = |explicit_stable: bool| {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("migrate-preview");
        if explicit_stable {
            cmd.arg("--surface").arg("stable");
        }
        cmd.arg(fixture.as_os_str());
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    assert_eq!(run(false), run(true));
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
fn migrate_preview_legacy_unary_omitted_surface_matches_explicit_stable() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");

    let run = |explicit_stable: bool| {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("migrate-preview");
        if explicit_stable {
            cmd.arg("--surface").arg("stable");
        }
        cmd.arg(fixture.as_os_str());
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    assert_eq!(run(false), run(true));
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
