#[test]
fn usage_includes_artifact_json_consumer_commands() {
    let root = repo_root();

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root);
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");

    assert!(stderr.contains("kernriftc inspect-report --report <verify-report.json> --format json"));
    assert!(stderr.contains("kernriftc inspect-artifact <artifact-path> --format json"));
    assert!(stderr.contains("kernriftc verify-artifact-meta --format json <artifact> <meta.json>"));
    assert!(stderr.contains(
        "kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>"
    ));
    assert!(stderr.contains("kernriftc check --format json --policy <policy.toml> <file.kr>"));
    assert!(stderr.contains("kernriftc check --canonical <file.kr>"));
    assert!(stderr.contains("kernriftc check --canonical --stdin"));
    assert!(stderr.contains("kernriftc check --canonical --format json <file.kr>"));
    assert!(stderr.contains("kernriftc check --canonical --stdin --format json"));
    assert!(stderr.contains("kernriftc migrate-preview <file.kr>"));
    assert!(stderr.contains("kernriftc migrate-preview --canonical-edits --format text <file.kr>"));
    assert!(stderr.contains("kernriftc migrate-preview --canonical-edits --format json <file.kr>"));
    assert!(stderr.contains(
        "kernriftc migrate-preview --canonical-edits --format text --surface stable <file.kr>"
    ));
    assert!(stderr.contains(
        "kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>"
    ));
    assert!(stderr.contains(
        "kernriftc migrate-preview --canonical-edits --format text --surface experimental <file.kr>"
    ));
    assert!(stderr.contains("kernriftc migrate-preview --canonical-edits --stdin"));
    assert!(stderr.contains("kernriftc migrate-preview --canonical-edits --stdin --format json"));
    assert!(stderr.contains("kernriftc fix --canonical --write <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --write --format json <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run --format json <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run --stdin"));
    assert!(stderr.contains("kernriftc fix --canonical --dry-run --stdin --format json"));
    assert!(stderr.contains("kernriftc fix --canonical --stdout <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --stdout --surface experimental <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --stdout --stdin"));
    assert!(stderr.contains("kernriftc fix --canonical --diff <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --diff --surface experimental <file.kr>"));
    assert!(stderr.contains("kernriftc fix --canonical --diff --stdin"));
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
fn emit_asm_supports_uart_console_probe_proof_program_exactly() {
    let root = repo_root();
    let fixture = root.join("examples").join("uart_console_probe.kr");
    let output_path = unique_temp_output_path("emit-asm-uart-console-probe", "s");
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
        concat!(
            ".text\n",
            "\n",
            ".globl entry\n",
            "entry:\n",
            "    call uart_init\n",
            "    call uart_send_break\n",
            "    call uart_kick_watchdog\n",
            "    movabs $0x1004, %rax\n",
            "    movl (%rax), %eax\n",
            "    ret\n",
            "\n",
            ".globl uart_init\n",
            "uart_init:\n",
            "    movabs $0x1008, %rax\n",
            "    movl $0x1, %ecx\n",
            "    movl %ecx, (%rax)\n",
            "    call platform_barrier\n",
            "    call uart_status\n",
            "    ret\n",
            "\n",
            ".globl uart_kick_watchdog\n",
            "uart_kick_watchdog:\n",
            "    movabs $0x1014, %rax\n",
            "    movl $0xdeadbeef, %ecx\n",
            "    movl %ecx, (%rax)\n",
            "    ret\n",
            "\n",
            ".globl uart_send_break\n",
            "uart_send_break:\n",
            "    movabs $0x1000, %rax\n",
            "    movb $0x0, %cl\n",
            "    movb %cl, (%rax)\n",
            "    call platform_barrier\n",
            "    ret\n",
            "\n",
            ".globl uart_status\n",
            "uart_status:\n",
            "    movabs $0x1004, %rax\n",
            "    movl (%rax), %eax\n",
            "    ret\n"
        )
    );

    fs::remove_file(&output_path).ok();
}

#[test]
fn emit_elfobj_supports_uart_console_probe_proof_program_and_metadata_verifies() {
    let root = repo_root();
    let fixture = root.join("examples").join("uart_console_probe.kr");
    let artifact_path = unique_temp_output_path("emit-elfobj-uart-console-probe", "o");
    let meta_path = unique_temp_output_path("emit-elfobj-uart-console-probe", "json");

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
fn emit_backend_artifact_rejects_nonliteral_mmio_write_value_in_current_subset() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("mmio_typed.kr");
    let output_path = unique_temp_output_path("emit-mmio-nonliteral-value", "o");
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
        vec![
            "canonical-exec: function 'entry' contains unsupported mmio_write<u8>(UART0, value): non-literal write value 'value' is not executable in v0.1"
        ]
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
    validate_inspect_artifact_schema(&json);
    assert_eq!(json["schema_version"], "kernrift_inspect_artifact_v2");
    assert_eq!(json["file"], artifact_path.display().to_string());
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
    validate_inspect_artifact_schema(&json);
    assert_eq!(json["schema_version"], "kernrift_inspect_artifact_v2");
    assert_eq!(json["file"], artifact_path.display().to_string());
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
        validate_inspect_artifact_schema(report);
        assert_eq!(
            report["schema_version"],
            json!("kernrift_inspect_artifact_v2")
        );
        for key in [
            "schema_version",
            "file",
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
    assert_json_transport(&stdout, &stderr, "kernrift_inspect_artifact_v2");

    fs::remove_file(&artifact_path).ok();
}

#[test]
fn inspect_artifact_json_rejects_malformed_bytes_without_emitting_json() {
    let root = repo_root();
    let input_path = unique_temp_output_path("inspect-artifact-json-random-text", "txt");
    fs::write(&input_path, "hello artifact\n").expect("write random text");

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-artifact")
        .arg(input_path.as_os_str())
        .arg("--format")
        .arg("json");
    let assert = cmd.assert().failure().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout}");
    assert_eq!(
        stderr.lines().next(),
        Some("inspect-artifact: unsupported artifact bytes")
    );

    fs::remove_file(&input_path).ok();
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

    let expected_json = |file: &Path,
                         artifact_kind: &str,
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
            "schema_version": "kernrift_inspect_artifact_v2",
            "file": file.display().to_string(),
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
            &basic_krbo,
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
            &basic_elf,
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
            &basic_asm,
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
            &extern_elf,
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
            &extern_asm,
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
            &mixed_elf,
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
            &mixed_asm,
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
    validate_verify_artifact_meta_schema(&json);
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_verify_artifact_meta_v2",
            "file": artifact_path.display().to_string(),
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
    validate_verify_artifact_meta_schema(&json);
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_verify_artifact_meta_v2",
            "file": artifact_path.display().to_string(),
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
fn verify_artifact_meta_json_rejects_invalid_input_without_emitting_json() {
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
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout}");
    assert_eq!(
        stderr.lines().next(),
        Some("verify-artifact-meta: unsupported artifact bytes")
    );

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
    let json: Value = serde_json::from_str(&stdout).expect("parse verify-artifact-meta JSON");
    validate_verify_artifact_meta_schema(&json);
    assert_json_transport(&stdout, &stderr, "kernrift_verify_artifact_meta_v2");

    fs::remove_file(&artifact_path).ok();
    fs::remove_file(&meta_path).ok();
}

#[test]
fn verify_artifact_meta_json_rejects_malformed_metadata_without_emitting_json() {
    let root = repo_root();
    let fixture = root.join("tests").join("must_pass").join("basic.kr");
    let artifact_path = unique_temp_output_path("verify-meta-json-malformed-meta", "krbo");
    let meta_path = unique_temp_output_path("verify-meta-json-malformed-meta", "json");
    emit_backend_artifact_with_sidecar(&root, "krbo", &fixture, &artifact_path, &meta_path, false);
    fs::write(&meta_path, b"{not valid json").expect("write malformed metadata");

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
    assert!(stdout.is_empty(), "expected empty stdout, got: {stdout}");
    assert!(
        stderr
            .lines()
            .next()
            .is_some_and(|line| line.starts_with("failed to decode artifact metadata '")),
        "expected malformed metadata decode error, got: {stderr}"
    );

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
fn inspect_report_rejects_duplicate_format_flag() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "duplicate-format",
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

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str())
        .arg("--format")
        .arg("json")
        .arg("--format")
        .arg("text");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid inspect-report mode: duplicate --format")
    );

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_rejects_format_missing_value() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "missing-format-value",
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

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str())
        .arg("--format");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid inspect-report mode: --format requires 'text' or 'json'")
    );

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_rejects_invalid_format_value() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "invalid-format-value",
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

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str())
        .arg("--format")
        .arg("yaml");
    let assert = cmd.assert().failure().code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_eq!(
        stderr.lines().next(),
        Some("invalid inspect-report mode: unsupported --format 'yaml' (expected 'text' or 'json')")
    );

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_default_text_matches_explicit_text() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "default-text-parity",
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

    let run = |explicit_text: bool| {
        let mut cmd: Command = cargo_bin_cmd!("kernriftc");
        cmd.current_dir(&root)
            .arg("inspect-report")
            .arg("--report")
            .arg(report_path.as_os_str());
        if explicit_text {
            cmd.arg("--format").arg("text");
        }
        let assert = cmd.assert().success();
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
    };

    assert_eq!(run(false), run(true));

    fs::remove_file(&report_path).ok();
}

#[test]
fn inspect_report_json_rejects_malformed_report_without_emitting_json() {
    let root = repo_root();
    let report_path = write_verify_report_fixture("malformed-json", &json!({}));

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str())
        .arg("--format")
        .arg("json");
    let assert = cmd.assert().failure().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");

    assert!(
        stdout.is_empty(),
        "malformed inspect-report JSON mode must not emit stdout payload: {:?}",
        stdout
    );
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
fn inspect_report_json_is_stable_and_exact() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "json",
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
        .arg(report_path.as_os_str())
        .arg("--format")
        .arg("json");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert_json_transport(&stdout, &stderr, "kernrift_inspect_report_v1");
    let json: Value = serde_json::from_str(&stdout).expect("inspect-report json");
    validate_inspect_report_schema(&json);
    assert_eq!(
        json,
        json!({
            "schema_version": "kernrift_inspect_report_v1",
            "file": report_path.display().to_string(),
            "report_schema_version": "kernrift_verify_report_v1",
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
        })
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
fn inspect_report_json_transport_is_stdout_only_and_newline_terminated() {
    let root = repo_root();
    let report_path = write_verify_report_fixture(
        "transport",
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

    let mut cmd: Command = cargo_bin_cmd!("kernriftc");
    cmd.current_dir(&root)
        .arg("inspect-report")
        .arg("--report")
        .arg(report_path.as_os_str())
        .arg("--format")
        .arg("json");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");

    assert_json_transport(&stdout, &stderr, "kernrift_inspect_report_v1");
    let json: Value = serde_json::from_str(&stdout).expect("inspect-report json");
    assert_eq!(json["file"], json!(report_path.display().to_string()));
    assert_eq!(json["report_schema_version"], json!("kernrift_verify_report_v1"));

    fs::remove_file(&report_path).ok();
}
