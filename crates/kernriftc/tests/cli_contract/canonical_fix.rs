
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
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert = run_fix_canonical_dry_run_file(&root, &temp_fixture, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert = run_fix_canonical_dry_run_file(&root, &temp_fixture, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert =
        run_fix_canonical_dry_run_file(&root, &temp_fixture, Some("experimental"), None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
    let fixture = living_compiler_fixture("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert =
        run_fix_canonical_dry_run_file(&root, &temp_fixture, Some("experimental"), Some("json"))
            .success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
fn fix_canonical_dry_run_from_stdin_reports_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture("canonical_check_aliases.kr"));
    let assert =
        run_fix_canonical_dry_run_stdin(&root, &input, Some("experimental"), None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "fix dry-run stdin mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: experimental",
            "rewrites_planned: 3",
            "file: <stdin>",
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
}

#[test]
fn fix_canonical_dry_run_from_stdin_reports_legacy_unary_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert = run_fix_canonical_dry_run_stdin(&root, &input, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "fix dry-run stdin mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec![
            "surface: stable",
            "rewrites_planned: 5",
            "file: <stdin>",
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
}

#[test]
fn fix_canonical_dry_run_json_from_stdin_reports_legacy_unary_exactly() {
    let root = repo_root();
    let input = fixture_text(&living_compiler_fixture(
        "migration_preview_legacy_unary.kr",
    ));
    let assert = run_fix_canonical_dry_run_stdin(&root, &input, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_preview_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_preview_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_fix_preview_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"<stdin>\",\n  \"rewrites_planned\": 5,\n  \"would_change\": true,\n  \"rewrites\": [\n    {\n      \"function\": \"alloc_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@alloc\",\n      \"canonical_replacement\": \"@eff(alloc)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"block_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@block\",\n      \"canonical_replacement\": \"@eff(block)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"irq_entry\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@irq\",\n      \"canonical_replacement\": \"@ctx(irq)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"noirq_worker\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@noirq\",\n      \"canonical_replacement\": \"@ctx(thread, boot)\",\n      \"migration_safe\": true\n    },\n    {\n      \"function\": \"preempt_guarded\",\n      \"classification\": \"compatibility_alias\",\n      \"surface_form\": \"@preempt_off\",\n      \"canonical_replacement\": \"@eff(preempt_off)\",\n      \"migration_safe\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn fix_canonical_dry_run_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert = run_fix_canonical_dry_run_file(&root, &temp_fixture, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
fn fix_canonical_dry_run_from_stdin_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_fix_canonical_dry_run_stdin(&root, &input, None, None).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert!(
        stderr.is_empty(),
        "fix dry-run stdin mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["surface: stable", "rewrites_planned: 0", "file: <stdin>",]
    );
}

#[test]
fn fix_canonical_dry_run_json_is_empty_for_canonical_source() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-dry-run-json-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let assert = run_fix_canonical_dry_run_file(&root, &temp_fixture, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
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
fn fix_canonical_dry_run_json_from_stdin_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_fix_canonical_dry_run_stdin(&root, &input, None, Some("json")).success();
    let stdout = stdout_string(&assert);
    let stderr = stderr_string(&assert);
    assert_json_transport(&stdout, &stderr, "kernrift_canonical_fix_preview_v1");
    let json: Value = serde_json::from_str(&stdout).expect("json stdout");
    validate_canonical_fix_preview_schema(&json);
    assert_eq!(
        stdout,
        "{\n  \"schema_version\": \"kernrift_canonical_fix_preview_v1\",\n  \"surface\": \"stable\",\n  \"file\": \"<stdin>\",\n  \"rewrites_planned\": 0,\n  \"would_change\": false,\n  \"rewrites\": []\n}\n"
    );
}

#[test]
fn fix_canonical_dry_run_json_from_stdin_uses_stdin_file_label() {
    let root = repo_root();
    let input = fixture_text(&must_pass_fixture("basic.kr"));
    let assert = run_fix_canonical_dry_run_stdin(&root, &input, None, Some("json")).success();
    let json = stdout_json(&assert);
    assert_eq!(json.get("file").and_then(Value::as_str), Some("<stdin>"));
}

#[test]
fn fix_canonical_dry_run_text_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_stdout =
        stdout_string(&run_fix_canonical_dry_run_file(&root, &fixture, None, None).success());
    let stdin_stdout =
        stdout_string(&run_fix_canonical_dry_run_stdin(&root, &input, None, None).success());

    assert_eq!(
        normalized_lines_without_file_label(&file_stdout),
        normalized_lines_without_file_label(&stdin_stdout)
    );
}

#[test]
fn fix_canonical_dry_run_json_file_and_stdin_are_parity_locked_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");
    let input = fixture_text(&fixture);

    let file_stdout = stdout_string(
        &run_fix_canonical_dry_run_file(&root, &fixture, None, Some("json")).success(),
    );
    let stdin_stdout = stdout_string(
        &run_fix_canonical_dry_run_stdin(&root, &input, None, Some("json")).success(),
    );

    assert_eq!(
        normalized_fix_preview_json(&file_stdout),
        normalized_fix_preview_json(&stdin_stdout)
    );
}

#[test]
fn fix_canonical_dry_run_text_count_matches_json_count_for_legacy_unary() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    let text_stdout =
        stdout_string(&run_fix_canonical_dry_run_file(&root, &fixture, None, None).success());
    let json =
        stdout_json(&run_fix_canonical_dry_run_file(&root, &fixture, None, Some("json")).success());

    assert_eq!(
        text_count_value(&text_stdout, "rewrites_planned"),
        json.get("rewrites_planned")
            .and_then(Value::as_u64)
            .expect("rewrites_planned field") as usize
    );
}

#[test]
fn fix_canonical_non_mutating_modes_keep_source_file_unchanged() {
    let root = repo_root();
    let fixture = living_compiler_fixture("migration_preview_legacy_unary.kr");

    for (label, extra_args) in [
        ("dry-run", vec!["--dry-run"]),
        ("stdout", vec!["--stdout"]),
        ("diff", vec!["--diff"]),
    ] {
        let temp_fixture =
            copy_fixture_to_temp(&format!("canonical-non-mutating-{label}"), &fixture);
        let before = fs::read_to_string(&temp_fixture).expect("read temp fixture before");

        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root).arg("fix").arg("--canonical");
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.arg(temp_fixture.as_os_str()).assert().success();

        let after = fs::read_to_string(&temp_fixture).expect("read temp fixture after");
        assert_eq!(
            after, before,
            "fix --canonical {label} must not mutate source files"
        );
    }
}

#[test]
fn fix_canonical_dry_run_noop_file_and_stdin_text_and_json_are_parity_locked() {
    let root = repo_root();
    let fixture = must_pass_fixture("basic.kr");
    let input = fixture_text(&fixture);

    let file_text =
        stdout_string(&run_fix_canonical_dry_run_file(&root, &fixture, None, None).success());
    let stdin_text =
        stdout_string(&run_fix_canonical_dry_run_stdin(&root, &input, None, None).success());

    let file_json_stdout = stdout_string(
        &run_fix_canonical_dry_run_file(&root, &fixture, None, Some("json")).success(),
    );
    let stdin_json_stdout = stdout_string(
        &run_fix_canonical_dry_run_stdin(&root, &input, None, Some("json")).success(),
    );

    assert_eq!(
        normalized_lines_without_file_label(&file_text),
        normalized_lines_without_file_label(&stdin_text)
    );
    assert_eq!(
        normalized_fix_preview_json(&file_json_stdout),
        normalized_fix_preview_json(&stdin_json_stdout)
    );
    assert_eq!(text_count_value(&file_text, "rewrites_planned"), 0);
    assert_eq!(
        normalized_fix_preview_json(&file_json_stdout)
            .get("rewrites_planned")
            .and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn fix_canonical_stdout_rewrites_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-stdout-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix stdout mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "@eff(alloc)\nfn alloc_worker() { }\n\n@eff(block)\nfn block_worker() { }\n\n@ctx(irq)\nfn irq_entry() { }\n\n@ctx(thread, boot)\nfn noirq_worker() { }\n\n@eff(preempt_off)\nfn preempt_guarded() { }\n"
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_stdout_from_stdin_rewrites_legacy_unary_exactly() {
    let root = repo_root();
    let input = fs::read_to_string(
        root.join("tests")
            .join("living_compiler")
            .join("migration_preview_legacy_unary.kr"),
    )
    .expect("read legacy unary fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--stdin")
        .write_stdin(input);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix stdout stdin mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "@eff(alloc)\nfn alloc_worker() { }\n\n@eff(block)\nfn block_worker() { }\n\n@ctx(irq)\nfn irq_entry() { }\n\n@ctx(thread, boot)\nfn noirq_worker() { }\n\n@eff(preempt_off)\nfn preempt_guarded() { }\n"
    );
}

#[test]
fn fix_canonical_stdout_rewrites_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-stdout-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix stdout mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "@eff(block)\nfn blocker() { }\n\n@ctx(irq)\nfn isr() { }\n\n@ctx(thread)\nfn worker() { }\n"
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_stdout_from_stdin_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fs::read_to_string(root.join("tests").join("must_pass").join("basic.kr"))
        .expect("read canonical fixture")
        .replace("\r\n", "\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--stdin")
        .write_stdin(input.clone());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix stdout stdin mode must keep stderr empty on success"
    );
    assert_eq!(stdout, input);
}

#[test]
fn fix_canonical_stdout_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-stdout-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture)
        .expect("read original fixture")
        .replace("\r\n", "\n");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix stdout mode must keep stderr empty on success"
    );
    assert_eq!(stdout, original);
    assert_eq!(
        fs::read_to_string(&temp_fixture)
            .expect("read unchanged fixture")
            .replace("\r\n", "\n"),
        original
    );
}

#[test]
fn fix_canonical_diff_from_stdin_rewrites_legacy_unary_exactly() {
    let root = repo_root();
    let input = fs::read_to_string(
        root.join("tests")
            .join("living_compiler")
            .join("migration_preview_legacy_unary.kr"),
    )
    .expect("read legacy unary fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg("--stdin")
        .write_stdin(input);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix diff stdin mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "--- original\n+++ canonical\n@@ -1,14 +1,14 @@\n-@alloc\n-fn alloc_worker() { }\n-\n-@block\n-fn block_worker() { }\n-\n-@irq\n-fn irq_entry() { }\n-\n-@noirq\n-fn noirq_worker() { }\n-\n-@preempt_off\n-fn preempt_guarded() { }\n+@eff(alloc)\n+fn alloc_worker() { }\n+\n+@eff(block)\n+fn block_worker() { }\n+\n+@ctx(irq)\n+fn irq_entry() { }\n+\n+@ctx(thread, boot)\n+fn noirq_worker() { }\n+\n+@eff(preempt_off)\n+fn preempt_guarded() { }\n"
    );
}

#[test]
fn fix_canonical_diff_rewrites_legacy_unary_shorthands_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("migration_preview_legacy_unary.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-diff-legacy-unary", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix diff mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "--- original\n+++ canonical\n@@ -1,14 +1,14 @@\n-@alloc\n-fn alloc_worker() { }\n-\n-@block\n-fn block_worker() { }\n-\n-@irq\n-fn irq_entry() { }\n-\n-@noirq\n-fn noirq_worker() { }\n-\n-@preempt_off\n-fn preempt_guarded() { }\n+@eff(alloc)\n+fn alloc_worker() { }\n+\n+@eff(block)\n+fn block_worker() { }\n+\n+@ctx(irq)\n+fn irq_entry() { }\n+\n+@ctx(thread, boot)\n+fn noirq_worker() { }\n+\n+@eff(preempt_off)\n+fn preempt_guarded() { }\n"
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_diff_from_stdin_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let input = fs::read_to_string(root.join("tests").join("must_pass").join("basic.kr"))
        .expect("read canonical fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg("--stdin")
        .write_stdin(input);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix diff stdin mode must keep stderr empty on success"
    );
    assert_eq!(stdout, "");
}

#[test]
fn fix_canonical_diff_rewrites_accepted_aliases_under_experimental_surface_exactly() {
    let root = repo_root();
    let fixture = root
        .join("tests")
        .join("living_compiler")
        .join("canonical_check_aliases.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-diff-aliases", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg("--surface")
        .arg("experimental")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix diff mode must keep stderr empty on success"
    );
    assert_eq!(
        stdout,
        "--- original\n+++ canonical\n@@ -1,8 +1,8 @@\n-@may_block\n-fn blocker() { }\n-\n-@irq_handler\n-fn isr() { }\n-\n-@thread_entry\n-fn worker() { }\n+@eff(block)\n+fn blocker() { }\n+\n+@ctx(irq)\n+fn isr() { }\n+\n+@ctx(thread)\n+fn worker() { }\n"
    );
    assert_eq!(
        fs::read_to_string(&temp_fixture).expect("read unchanged fixture"),
        original
    );
}

#[test]
fn fix_canonical_diff_noops_cleanly_for_canonical_source() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let temp_fixture = copy_fixture_to_temp("fix-canonical-diff-noop", &fixture);
    let original = fs::read_to_string(&temp_fixture).expect("read original fixture");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg(temp_fixture.as_os_str());
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.is_empty(),
        "fix diff mode must keep stderr empty on success"
    );
    assert_eq!(stdout, "");
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
fn fix_canonical_rejects_conflicting_write_and_stdout_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--stdout")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --stdout cannot be combined with --write"));
}

#[test]
fn fix_canonical_rejects_conflicting_dry_run_and_stdout_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--stdout")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --stdout cannot be combined with --dry-run"));
}

#[test]
fn fix_canonical_rejects_conflicting_write_and_diff_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--diff")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --diff cannot be combined with --write"));
}

#[test]
fn fix_canonical_rejects_stdin_with_write() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--write")
        .arg("--stdin")
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --stdin cannot be combined with --write"));
}

#[test]
fn fix_canonical_rejects_conflicting_dry_run_and_diff_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--dry-run")
        .arg("--diff")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --diff cannot be combined with --dry-run"));
}

#[test]
fn fix_canonical_rejects_duplicate_stdin_flag() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--stdin")
        .arg("--stdin")
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: duplicate --stdin"));
}

#[test]
fn fix_canonical_rejects_stdin_and_file_together() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--stdin")
        .arg(fixture.as_os_str())
        .write_stdin("@irq\nfn entry() { }\n");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --stdin cannot be combined with an input file"));
}

#[test]
fn fix_canonical_rejects_conflicting_stdout_and_diff_flags() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--diff")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: --diff cannot be combined with --stdout"));
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
fn fix_canonical_rejects_duplicate_stdout_flag() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--stdout")
        .arg("--stdout")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: duplicate --stdout"));
}

#[test]
fn fix_canonical_rejects_duplicate_diff_flag() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("fix")
        .arg("--canonical")
        .arg("--diff")
        .arg("--diff")
        .arg(fixture.as_os_str());
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("invalid fix mode: duplicate --diff"));
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
