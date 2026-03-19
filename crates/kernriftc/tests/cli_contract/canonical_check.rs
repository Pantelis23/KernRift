
#[test]
fn check_canonical_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let assert = run_check_canonical_file(&root, &fixture, None, None)
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check must report via stdout only"
    );
    assert_eq!(
        stdout,
        format!(
            "surface: stable\ncanonical_findings: 5\nfile: {}\nfunction: alloc_worker\nclassification: compatibility_alias\nsurface_form: @alloc\ncanonical_replacement: @eff(alloc)\nmigration_safe: true\nfunction: block_worker\nclassification: compatibility_alias\nsurface_form: @block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nfunction: irq_entry\nclassification: compatibility_alias\nsurface_form: @irq\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nfunction: noirq_worker\nclassification: compatibility_alias\nsurface_form: @noirq\ncanonical_replacement: @ctx(thread, boot)\nmigration_safe: true\nfunction: preempt_guarded\nclassification: compatibility_alias\nsurface_form: @preempt_off\ncanonical_replacement: @eff(preempt_off)\nmigration_safe: true\n",
            fixture.display()
        )
    );
}

#[test]
fn check_canonical_json_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let assert = run_check_canonical_file(&root, &fixture, None, Some("json"))
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let assert = run_check_canonical_file(&root, &fixture, Some("experimental"), None)
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check must report via stdout only"
    );
    assert_eq!(
        stdout,
        format!(
            "surface: experimental\ncanonical_findings: 3\nfile: {}\nfunction: blocker\nclassification: compatibility_alias\nsurface_form: @may_block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nfunction: isr\nclassification: compatibility_alias\nsurface_form: @irq_handler\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nfunction: worker\nclassification: compatibility_alias\nsurface_form: @thread_entry\ncanonical_replacement: @ctx(thread)\nmigration_safe: true\n",
            fixture.display()
        )
    );
}

#[test]
fn check_canonical_json_reports_accepted_aliases_under_experimental_surface() {
    let root = repo_root();
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let assert = run_check_canonical_file(&root, &fixture, Some("experimental"), Some("json"))
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"experimental\",\n  \"canonical_findings\": 3,\n  \"findings\": [\n    {\n      \"function\": \"blocker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@may_block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"isr\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq_handler\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@thread_entry\",\n      \"canonical_replacement\": \"@ctx(thread)\",\n      \"migration_safe\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn check_canonical_from_stdin_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert = run_check_canonical_stdin(&root, &input, None, None)
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check stdin mode must report via stdout only"
    );
    assert_eq!(
        stdout,
        "surface: stable\ncanonical_findings: 5\nfile: <stdin>\nfunction: alloc_worker\nclassification: compatibility_alias\nsurface_form: @alloc\ncanonical_replacement: @eff(alloc)\nmigration_safe: true\nfunction: block_worker\nclassification: compatibility_alias\nsurface_form: @block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nfunction: irq_entry\nclassification: compatibility_alias\nsurface_form: @irq\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nfunction: noirq_worker\nclassification: compatibility_alias\nsurface_form: @noirq\ncanonical_replacement: @ctx(thread, boot)\nmigration_safe: true\nfunction: preempt_guarded\nclassification: compatibility_alias\nsurface_form: @preempt_off\ncanonical_replacement: @eff(preempt_off)\nmigration_safe: true\n"
    );
}

#[test]
fn check_canonical_json_from_stdin_reports_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert = run_check_canonical_stdin(&root, &input, None, Some("json"))
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"stable\",\n  \"canonical_findings\": 5,\n  \"findings\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn check_canonical_from_stdin_reports_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("canonical_check_aliases.kr"));
    let assert = run_check_canonical_stdin(&root, &input, Some("experimental"), None)
        .failure()
        .code(1);
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check stdin mode must report via stdout only"
    );
    assert_eq!(
        stdout,
        "surface: experimental\ncanonical_findings: 3\nfile: <stdin>\nfunction: blocker\nclassification: compatibility_alias\nsurface_form: @may_block\ncanonical_replacement: @eff(block)\nmigration_safe: true\nfunction: isr\nclassification: compatibility_alias\nsurface_form: @irq_handler\ncanonical_replacement: @ctx(irq)\nmigration_safe: true\nfunction: worker\nclassification: compatibility_alias\nsurface_form: @thread_entry\ncanonical_replacement: @ctx(thread)\nmigration_safe: true\n"
    );
}

#[test]
fn check_canonical_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let assert = run_check_canonical_file(&root, &fixture, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check success must keep stderr empty"
    );
    assert_eq!(
        stdout,
        format!("surface: stable\ncanonical_findings: 0\nfile: {}\n", fixture.display())
    );
}

#[test]
fn check_canonical_from_stdin_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_check_canonical_stdin(&root, &input, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "canonical check stdin success must keep stderr empty"
    );
    assert_eq!(stdout, "surface: stable\ncanonical_findings: 0\nfile: <stdin>\n");
}

#[test]
fn check_canonical_json_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let assert = run_check_canonical_file(&root, &fixture, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"stable\",\n  \"canonical_findings\": 0,\n  \"findings\": []\n}\n"
    );
}

#[test]
fn check_canonical_json_from_stdin_succeeds_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_check_canonical_stdin(&root, &input, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_findings_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_findings_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_findings_v1\",\n  \"surface\": \"stable\",\n  \"canonical_findings\": 0,\n  \"findings\": []\n}\n"
    );
}

#[test]
fn check_canonical_text_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_assert = run_check_canonical_file(&root, &fixture, None, None)
        .failure()
        .code(1);
    let file_stdout = stdout_string(&file_assert);
    let file_stderr = stderr_string(&file_assert);

    let stdin_assert = run_check_canonical_stdin(&root, &input, None, None)
        .failure()
        .code(1);
    let stdin_stdout = stdout_string(&stdin_assert);
    let stdin_stderr = stderr_string(&stdin_assert);

    assert!(file_stderr.is_empty());
    assert!(stdin_stderr.is_empty());
    assert_eq!(
        normalized_lines_without_file_label(&file_stdout),
        normalized_lines_without_file_label(&stdin_stdout)
    );
}

#[test]
fn check_canonical_json_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_assert = run_check_canonical_file(&root, &fixture, None, Some("json"))
        .failure()
        .code(1);
    let stdin_assert = run_check_canonical_stdin(&root, &input, None, Some("json"))
        .failure()
        .code(1);

    let file_json = stdout_json(&file_assert);
    let stdin_json = stdout_json(&stdin_assert);
    assert_eq!(file_json, stdin_json);
}

#[test]
fn check_canonical_text_count_matches_json_count_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    let text_assert = run_check_canonical_file(&root, &fixture, None, None)
        .failure()
        .code(1);
    let text_stdout = stdout_string(&text_assert);

    let json_assert = run_check_canonical_file(&root, &fixture, None, Some("json"))
        .failure()
        .code(1);
    let json = stdout_json(&json_assert);

    assert_eq!(
        text_count_value(&text_stdout, "canonical_findings"),
        json.get("canonical_findings")
            .and_then(Value::as_u64)
            .expect("canonical_findings field") as usize
    );
}

#[test]
fn check_canonical_noop_file_and_stdin_text_and_json_are_parity_locked() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let input = fixture_text(&fixture);

    let file_text = stdout_string(&run_check_canonical_file(&root, &fixture, None, None).success());
    let stdin_text = stdout_string(&run_check_canonical_stdin(&root, &input, None, None).success());

    let file_json =
        stdout_json(&run_check_canonical_file(&root, &fixture, None, Some("json")).success());
    let stdin_json =
        stdout_json(&run_check_canonical_stdin(&root, &input, None, Some("json")).success());

    assert_eq!(
        normalized_lines_without_file_label(&file_text),
        normalized_lines_without_file_label(&stdin_text)
    );
    assert_eq!(file_json, stdin_json);
    assert_eq!(text_count_value(&file_text, "canonical_findings"), 0);
    assert_eq!(
        file_json.get("canonical_findings").and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn check_canonical_rejects_duplicate_stdin_flag() {
    let root = repo_root();
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--stdin")
        .arg("--stdin")
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid check mode: duplicate --stdin")
    );
}

#[test]
fn check_canonical_rejects_stdin_and_file_together() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("check")
        .arg("--canonical")
        .arg("--stdin")
        .arg(fixture.as_os_str())
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid check mode: --stdin cannot be combined with an input file")
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
