
#[test]
fn migrate_preview_canonical_edits_text_reports_legacy_unary_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan file text mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        format!(
            "surface: stable\nedits_count: 5\nfile: {}\nfunction: alloc_worker\nclassification: compatibility_alias\nsurface_form: @alloc\ncanonical_replacement: @eff(alloc)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.\nfunction: block_worker\nclassification: compatibility_alias\nsurface_form: @block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@block` with `@eff(block)`.\nfunction: irq_entry\nclassification: compatibility_alias\nsurface_form: @irq\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.\nfunction: noirq_worker\nclassification: compatibility_alias\nsurface_form: @noirq\ncanonical_replacement: @ctx(thread, boot)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.\nfunction: preempt_guarded\nclassification: compatibility_alias\nsurface_form: @preempt_off\ncanonical_replacement: @eff(preempt_off)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.\n",
            fixture.display()
        )
    );
}

#[test]
fn migrate_preview_canonical_edits_from_stdin_reports_legacy_unary_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert = run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan stdin text mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "surface: stable\nedits_count: 5\nfile: <stdin>\nfunction: alloc_worker\nclassification: compatibility_alias\nsurface_form: @alloc\ncanonical_replacement: @eff(alloc)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@alloc` with `@eff(alloc)`.\nfunction: block_worker\nclassification: compatibility_alias\nsurface_form: @block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@block` with `@eff(block)`.\nfunction: irq_entry\nclassification: compatibility_alias\nsurface_form: @irq\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@irq` with `@ctx(irq)`.\nfunction: noirq_worker\nclassification: compatibility_alias\nsurface_form: @noirq\ncanonical_replacement: @ctx(thread, boot)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@noirq` with `@ctx(thread, boot)`.\nfunction: preempt_guarded\nclassification: compatibility_alias\nsurface_form: @preempt_off\ncanonical_replacement: @eff(preempt_off)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_reports_legacy_unary_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 5,\n  \"edits\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@alloc` with `@eff(alloc)`.\"\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@noirq` with `@ctx(thread, boot)`.\"\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_from_stdin_reports_legacy_unary_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert =
        run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 5,\n  \"edits\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@alloc` with `@eff(alloc)`.\"\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@noirq` with `@ctx(thread, boot)`.\"\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@preempt_off` with `@eff(preempt_off)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_omitted_surface_file_text_matches_explicit_stable() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    let default_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file_default_surface(&root, &fixture, "text").success(),
    );
    let explicit_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success(),
    );

    assert_eq!(default_stdout, explicit_stdout);
}

#[test]
fn migrate_preview_canonical_edits_omitted_surface_file_json_matches_explicit_stable() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    let default_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file_default_surface(&root, &fixture, "json").success(),
    );
    let explicit_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success(),
    );

    assert_eq!(default_stdout, explicit_stdout);
}

#[test]
fn migrate_preview_canonical_edits_omitted_surface_stdin_text_matches_explicit_stable() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("migration_preview_legacy_unary.kr"));

    let default_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin_default_surface(&root, &input, None).success(),
    );
    let explicit_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", None).success(),
    );

    assert_eq!(default_stdout, explicit_stdout);
}

#[test]
fn migrate_preview_canonical_edits_omitted_surface_stdin_json_matches_explicit_stable() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("migration_preview_legacy_unary.kr"));

    let default_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin_default_surface(&root, &input, Some("json")).success(),
    );
    let explicit_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", Some("json")).success(),
    );

    assert_eq!(default_stdout, explicit_stdout);
}

#[test]
fn migrate_preview_canonical_edits_text_reports_experimental_aliases_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "experimental", "text").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan file text mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        format!(
            "surface: experimental\nedits_count: 3\nfile: {}\nfunction: blocker\nclassification: compatibility_alias\nsurface_form: @may_block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.\nfunction: isr\nclassification: compatibility_alias\nsurface_form: @irq_handler\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.\nfunction: worker\nclassification: compatibility_alias\nsurface_form: @thread_entry\ncanonical_replacement: @ctx(thread)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.\n",
            fixture.display()
        )
    );
}

#[test]
fn migrate_preview_canonical_edits_from_stdin_reports_experimental_aliases_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("canonical_check_aliases.kr"));
    let assert =
        run_migrate_preview_canonical_edits_stdin(&root, &input, "experimental", None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan stdin text mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "surface: experimental\nedits_count: 3\nfile: <stdin>\nfunction: blocker\nclassification: compatibility_alias\nsurface_form: @may_block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@may_block` with `@eff(block)`.\nfunction: isr\nclassification: compatibility_alias\nsurface_form: @irq_handler\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@irq_handler` with `@ctx(irq)`.\nfunction: worker\nclassification: compatibility_alias\nsurface_form: @thread_entry\ncanonical_replacement: @ctx(thread)\nmigration_safe: true\nrewrite_intent: Replace the attribute token `@thread_entry` with `@ctx(thread)`.\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_reports_experimental_aliases_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "experimental", "json").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"experimental\",\n  \"edits_count\": 3,\n  \"edits\": [\n    {\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@may_block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq_handler` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@thread_entry` with `@ctx(thread)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_from_stdin_reports_experimental_aliases_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("canonical_check_aliases.kr"));
    let assert =
        run_migrate_preview_canonical_edits_stdin(&root, &input, "experimental", Some("json"))
            .success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"experimental\",\n  \"edits_count\": 3,\n  \"edits\": [\n    {\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@may_block` with `@eff(block)`.\"\n    },\n    {\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@irq_handler` with `@ctx(irq)`.\"\n    },\n    {\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true,\n      \"rewrite_intent\": \"Replace the attribute token `@thread_entry` with `@ctx(thread)`.\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_text_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan file text mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        format!("surface: stable\nedits_count: 0\nfile: {}\n", fixture.display())
    );
}

#[test]
fn migrate_preview_canonical_edits_from_stdin_is_empty_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical edit-plan stdin text mode must keep stderr empty on success"
    );
    assert_eq!(stdout, "surface: stable\nedits_count: 0\nfile: <stdin>\n");
}

#[test]
fn migrate_preview_canonical_edits_json_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let assert =
        run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 0,\n  \"edits\": []\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_json_from_stdin_is_empty_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert =
        run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_edit_plan_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_edit_plan_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_edit_plan_v1\",\n  \"surface\": \"stable\",\n  \"edits_count\": 0,\n  \"edits\": []\n}\n"
    );
}

#[test]
fn migrate_preview_canonical_edits_text_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success(),
    );
    let stdin_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", None).success(),
    );

    assert_eq!(
        normalized_lines_without_file_label(&file_stdout),
        normalized_lines_without_file_label(&stdin_stdout)
    );
}

#[test]
fn migrate_preview_canonical_edits_json_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_json = stdout_json(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success(),
    );
    let stdin_json = stdout_json(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", Some("json")).success(),
    );
    assert_eq!(file_json, stdin_json);
}

#[test]
fn migrate_preview_canonical_edits_text_and_json_count_match_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    let text_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success(),
    );
    let json = stdout_json(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success(),
    );

    assert_eq!(
        text_count_value(&text_stdout, "edits_count"),
        json.get("edits_count")
            .and_then(Value::as_u64)
            .expect("edits_count field") as usize
    );
}

#[test]
fn migrate_preview_canonical_edits_experimental_text_file_and_stdin_are_parity_locked() {
    let root = repo_root();
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let input = fixture_text(&fixture);

    let file_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "experimental", "text")
            .success(),
    );
    let stdin_stdout = stdout_string(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "experimental", None).success(),
    );

    assert_eq!(
        normalized_lines_without_file_label(&file_stdout),
        normalized_lines_without_file_label(&stdin_stdout)
    );
}

#[test]
fn migrate_preview_canonical_edits_noop_file_and_stdin_text_and_json_are_parity_locked() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let input = fixture_text(&fixture);

    let file_text = stdout_string(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "text").success(),
    );
    let stdin_text = stdout_string(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", None).success(),
    );

    let file_json = stdout_json(
        &run_migrate_preview_canonical_edits_file(&root, &fixture, "stable", "json").success(),
    );
    let stdin_json = stdout_json(
        &run_migrate_preview_canonical_edits_stdin(&root, &input, "stable", Some("json")).success(),
    );

    assert_eq!(
        normalized_lines_without_file_label(&file_text),
        normalized_lines_without_file_label(&stdin_text)
    );
    assert_eq!(file_json, stdin_json);
    assert_eq!(text_count_value(&file_text, "edits_count"), 0);
    assert_eq!(
        file_json.get("edits_count").and_then(Value::as_u64),
        Some(0)
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
fn migrate_preview_canonical_edits_rejects_duplicate_stdin_flag() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--stdin")
        .arg("--stdin")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("stable")
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: duplicate --stdin")
    );
}

#[test]
fn migrate_preview_canonical_edits_rejects_stdin_and_file_together() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("migrate-preview")
        .arg("--canonical-edits")
        .arg("--stdin")
        .arg("--format")
        .arg("json")
        .arg("--surface")
        .arg("stable")
        .arg(fixture.as_os_str())
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid migrate-preview mode: --stdin cannot be combined with an input file")
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
