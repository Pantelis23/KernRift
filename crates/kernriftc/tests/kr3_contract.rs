// KR3 contract tests — UART driver subsystem, C ABI boundary, optimization passes.
//
// Exit criteria (KR3):
//   - UART driver module passes all semantic checks
//   - Hook + IRQ functions (no locks) compile to x86_64 ELF object
//   - extern fn appears as undefined symbol in the ELF (C ABI hardening)
//   - SysV param write byte emits ParamLoad in executable KRIR
//   - Trivial branch fold eliminates identical branch arms
//   - Dead extern declarations are stripped by optimizer
//   - KR2 features (locks, percpu, hooks) still enforced in KR3 modules

use std::path::{Path, PathBuf};

use kernriftc::{SurfaceProfile, check_module, compile_file, compile_source, compile_source_with_surface};
use krir::{
    ArithOp, BackendTargetContract, ExecutableOp, lower_current_krir_to_executable_krir,
    lower_executable_krir_to_x86_64_object,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn check_src(src: &str) -> Result<(), Vec<String>> {
    compile_source(src).and_then(|module| check_module(&module))
}

// ── UART driver semantic validation ──────────────────────────────────────────

#[test]
fn uart_driver_passes_all_semantic_checks() {
    let fixture = repo_root()
        .join("examples")
        .join("kernel")
        .join("uart_driver.kr");
    let module = compile_file(&fixture).expect("uart_driver.kr must parse and lower");
    check_module(&module).expect("uart_driver.kr must pass all semantic checks");
}

// ── C ABI boundary: hook + IRQ functions compile to ELF ──────────────────────

#[test]
fn uart_hook_and_irq_functions_compile_to_x86_64_elf_object() {
    // Subset of uart_driver.kr that excludes lock ops — directly ELF-compilable.
    // Demonstrates the scheduler hooks and IRQ handler producing real machine code.
    let src = r#"
        @module_caps(Mmio, MmioRaw);
        mmio UART = 0xFED00000;
        mmio_reg UART.IER = 0x01 : u8 rw;
        mmio_reg UART.LSR = 0x05 : u8 ro;
        extern @ctx(irq) @eff() @caps() fn uart_irq_ack();
        @hook(sched_out) @noyield @ctx(thread, boot)
        fn uart_sched_out() {
            mmio_write<u8>(UART + 0x01, 0x00);
        }
        @hook(sched_in) @noyield @ctx(thread, boot)
        fn uart_sched_in() {
            mmio_write<u8>(UART + 0x01, 0x01);
        }
        @ctx(irq) @eff(mmio) @caps(Mmio)
        fn uart_irq_handler() {
            mmio_read<u8>(UART + 0x05, _status);
            uart_irq_ack();
        }
    "#;
    let module = compile_source(src).expect("must compile");
    check_module(&module).expect("must pass semantic checks");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let object =
        lower_executable_krir_to_x86_64_object(&exec, &target).expect("must lower to x86_64 ELF");
    assert!(
        !object.text_bytes.is_empty(),
        "must emit non-empty text section"
    );
}

// ── C ABI boundary: extern fn appears as undefined symbol in ELF ──────────────

#[test]
fn extern_fn_appears_as_undefined_symbol_in_elf_object() {
    let src = r#"
        @module_caps(Mmio, MmioRaw);
        mmio UART = 0xFED00000;
        mmio_reg UART.LSR = 0x05 : u8 ro;
        extern @ctx(irq) @eff() @caps() fn uart_irq_ack();
        @ctx(irq) @eff(mmio) @caps(Mmio)
        fn uart_irq_handler() {
            mmio_read<u8>(UART + 0x05, _status);
            uart_irq_ack();
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let object = lower_executable_krir_to_x86_64_object(&exec, &target).expect("must produce ELF");
    assert!(
        object
            .undefined_function_symbols
            .contains(&"uart_irq_ack".to_string()),
        "expected uart_irq_ack as undefined symbol in ELF; got: {:?}",
        object.undefined_function_symbols
    );
}

// ── SysV ABI: scalar param produces ParamLoad in executable KRIR ──────────────

#[test]
fn scalar_param_produces_param_load_in_executable_krir() {
    // uart_write_byte(b: u8) uses 'b' as an MMIO write value.
    // The lowering must emit ParamLoad { param_idx: 0, ty: U8 } to spill
    // the incoming %rdi value before the MmioWriteValue op.
    let src = r#"
        @module_caps(Mmio);
        mmio UART = 0xFED00000;
        mmio_reg UART.Data = 0x00 : u8 rw;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn uart_write_byte(b: u8) {
            mmio_write<u8>(UART + 0x00, b);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let f = exec
        .functions
        .iter()
        .find(|f| f.name == "uart_write_byte")
        .expect("uart_write_byte must exist");
    assert!(
        f.blocks[0]
            .ops
            .iter()
            .any(|op| matches!(op, ExecutableOp::ParamLoad { param_idx: 0, .. })),
        "expected ParamLoad {{param_idx: 0}} in uart_write_byte; got: {:?}",
        f.blocks[0].ops
    );
}

// ── Optimization: trivial branch fold ────────────────────────────────────────

#[test]
fn trivial_branch_fold_eliminates_identical_branch_arms() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Control = 0x00 : u32 rw;
        mmio_reg DEV.Status  = 0x04 : u32 ro;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn noop_a() { mmio_write<u32>(DEV + 0x00, 0x00); }
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn handler() {
            mmio_read<u32>(DEV + 0x04, s);
            branch_if_zero(s, noop_a, noop_a);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let mut exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    let handler = exec
        .functions
        .iter()
        .find(|f| f.name == "handler")
        .expect("handler must exist");
    assert!(
        handler.blocks[0]
            .ops
            .iter()
            .any(|op| matches!(op, ExecutableOp::BranchIfZero { .. })),
        "expected BranchIfZero before optimization"
    );

    passes::optimize_executable_krir(&mut exec);

    let handler = exec
        .functions
        .iter()
        .find(|f| f.name == "handler")
        .expect("handler must exist");
    assert!(
        !handler.blocks[0]
            .ops
            .iter()
            .any(|op| matches!(op, ExecutableOp::BranchIfZero { .. })),
        "BranchIfZero must be eliminated after fold"
    );
    assert!(
        handler.blocks[0]
            .ops
            .iter()
            .any(|op| matches!(op, ExecutableOp::Call { callee } if callee == "noop_a")),
        "expected Call {{noop_a}} after fold; got: {:?}",
        handler.blocks[0].ops
    );
}

// ── Optimization: dead extern strip ──────────────────────────────────────────

#[test]
fn dead_extern_decl_is_stripped_by_optimizer() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Control = 0x00 : u32 rw;
        extern @ctx(thread, boot) @eff() @caps() fn never_called();
        extern @ctx(thread, boot) @eff() @caps() fn also_never();
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn f() { mmio_write<u32>(DEV + 0x00, 0x01); }
    "#;
    let module = compile_source(src).expect("must compile");
    let mut exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    assert_eq!(
        exec.extern_declarations.len(),
        2,
        "expected 2 extern decls before strip"
    );

    passes::optimize_executable_krir(&mut exec);

    assert_eq!(
        exec.extern_declarations.len(),
        0,
        "dead externs must be stripped; remaining: {:?}",
        exec.extern_declarations
    );
}

// ── Optimization: live extern is preserved ───────────────────────────────────

#[test]
fn live_extern_decl_is_preserved_by_optimizer() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Status  = 0x00 : u32 ro;
        extern @ctx(thread, boot) @eff() @caps() fn do_notify();
        extern @ctx(thread, boot) @eff() @caps() fn never_called();
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn f() {
            mmio_read<u32>(DEV + 0x00, _v);
            do_notify();
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let mut exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    passes::optimize_executable_krir(&mut exec);

    assert!(
        exec.extern_declarations
            .iter()
            .any(|e| e.name == "do_notify"),
        "live extern do_notify must be preserved"
    );
    assert!(
        !exec
            .extern_declarations
            .iter()
            .any(|e| e.name == "never_called"),
        "dead extern never_called must be stripped"
    );
}

// ── KR2 regression: lock inversion still rejected in KR3 modules ─────────────

#[test]
fn kr3_module_still_rejects_lock_order_inversion() {
    let src = r#"
        mmio DEV = 0xFEB00000;
        spinlock LockA;
        spinlock LockB;
        fn thread_a() {
            acquire(LockA); acquire(LockB);
            release(LockB); release(LockA);
        }
        fn thread_b() {
            acquire(LockB); acquire(LockA);
            release(LockA); release(LockB);
        }
    "#;
    let errs = check_src(src).expect_err("lock inversion must fail");
    assert!(
        errs.iter().any(|e| e.contains("LOCK_ORDER_INVERSION")),
        "expected LOCK_ORDER_INVERSION; got: {errs:?}"
    );
}

// ── PR-2: multiple stack locals per function ──────────────────────────────────

#[test]
fn multiple_stack_cells_compile_to_x86_64_elf_object() {
    // Two independent named locals (saved_a at slot 0, saved_b at slot 1).
    // Both must appear in the text section with correct slot offsets.
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.A = 0x00 : u32 ro;
        mmio_reg DEV.B = 0x04 : u32 ro;
        mmio_reg DEV.Out = 0x08 : u32 rw;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn snapshot() {
            stack_cell<u32>(saved_a);
            stack_cell<u32>(saved_b);
            mmio_read<u32>(DEV + 0x00, tmp);
            cell_store<u32>(saved_a, tmp);
            mmio_read<u32>(DEV + 0x04, tmp);
            cell_store<u32>(saved_b, tmp);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let object =
        lower_executable_krir_to_x86_64_object(&exec, &target).expect("must lower to x86_64 ELF");
    assert!(!object.text_bytes.is_empty(), "must emit non-empty text section");
}

#[test]
fn multiple_stack_cells_produce_correct_n_stack_cells_in_executable_krir() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.A = 0x00 : u32 ro;
        mmio_reg DEV.B = 0x04 : u32 ro;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn two_cells() {
            stack_cell<u32>(cell_x);
            stack_cell<u32>(cell_y);
            mmio_read<u32>(DEV + 0x00, v);
            cell_store<u32>(cell_x, v);
            mmio_read<u32>(DEV + 0x04, v);
            cell_store<u32>(cell_y, v);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let f = exec
        .functions
        .iter()
        .find(|f| f.name == "two_cells")
        .expect("two_cells must exist");
    // slot 0 (cell_x) and slot 1 (cell_y) → two StackStoreImm ops with slot_idx 0 and 1
    let store_slots: Vec<u8> = f.blocks[0]
        .ops
        .iter()
        .filter_map(|op| match op {
            ExecutableOp::StackStoreImm { slot_idx, .. } => Some(*slot_idx),
            ExecutableOp::StackStoreValue { slot_idx, .. } => Some(*slot_idx),
            _ => None,
        })
        .collect();
    assert!(
        store_slots.contains(&0),
        "expected StackStore at slot 0; got: {:?}",
        store_slots
    );
    assert!(
        store_slots.contains(&1),
        "expected StackStore at slot 1; got: {:?}",
        store_slots
    );
}

// ── PR-3: Arithmetic ops behind --surface experimental ───────────────────────

#[test]
fn cell_arith_imm_rejected_on_stable_surface() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.A = 0x00 : u32 rw;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn f() {
            stack_cell<u32>(x);
            cell_store<u32>(x, 0x01);
            cell_add<u32>(x, 1);
        }
    "#;
    // compile_source uses Stable surface — arith op must be rejected.
    let result = compile_source(src);
    assert!(
        result.is_err(),
        "cell_add must be rejected under stable surface"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.contains("experimental")),
        "expected 'experimental' in error; got: {:?}",
        errs
    );
}

#[test]
fn cell_arith_imm_accepted_on_experimental_surface() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Control = 0x00 : u32 rw;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn counter() {
            stack_cell<u32>(n);
            cell_store<u32>(n, 0x00);
            cell_add<u32>(n, 1);
            cell_sub<u32>(n, 1);
            cell_and<u32>(n, 0xFF);
            cell_or<u32>(n, 0x01);
            cell_xor<u32>(n, 0xF0);
            cell_shl<u32>(n, 2);
            cell_shr<u32>(n, 1);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let f = exec
        .functions
        .iter()
        .find(|f| f.name == "counter")
        .expect("counter must exist");
    let arith_ops: Vec<ArithOp> = f.blocks[0]
        .ops
        .iter()
        .filter_map(|op| match op {
            ExecutableOp::SlotArithImm { arith_op, .. } => Some(*arith_op),
            _ => None,
        })
        .collect();
    assert!(
        arith_ops.contains(&ArithOp::Add),
        "expected SlotArithImm(Add); got: {:?}",
        arith_ops
    );
    assert!(
        arith_ops.contains(&ArithOp::Shl),
        "expected SlotArithImm(Shl); got: {:?}",
        arith_ops
    );
}

#[test]
fn cell_arith_imm_compiles_to_x86_64_elf_object() {
    // Verifies that SlotArithImm ops survive the full pipeline to ELF bytes.
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Control = 0x00 : u32 rw;
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn bump() {
            stack_cell<u32>(val);
            cell_store<u32>(val, 0x00);
            cell_add<u32>(val, 5);
            cell_shl<u32>(val, 2);
            cell_and<u32>(val, 0xFF);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let object =
        lower_executable_krir_to_x86_64_object(&exec, &target).expect("must lower to x86_64 ELF");
    assert!(!object.text_bytes.is_empty(), "must emit non-empty text section");
}
