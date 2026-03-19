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

