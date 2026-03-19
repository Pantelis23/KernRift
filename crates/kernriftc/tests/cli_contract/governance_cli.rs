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

