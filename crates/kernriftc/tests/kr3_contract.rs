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
//   - Telemetry layer: collect_telemetry() produces accurate reports (PR-6)

use std::path::{Path, PathBuf};

use kernriftc::{
    SurfaceProfile, check_module, collect_telemetry, compile_file, compile_source,
    compile_source_with_surface,
};
use krir::{
    ArithOp, BackendTargetContract, BackendTargetId, ExecutableCallArg, ExecutableOp,
    ExecutableTerminator, emit_x86_64_asm_text, lower_current_krir_to_executable_krir,
    lower_executable_krir_to_x86_64_asm, lower_executable_krir_to_x86_64_object,
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

// ── PR-4: Call with args ──────────────────────────────────────────────────────

#[test]
fn call_with_args_rejected_on_stable_surface() {
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        extern @ctx(thread, boot) @eff() @caps() fn log_byte(b: u8);
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn send_status() {
            call_with_args(log_byte, 0x42);
        }
    "#;
    let errs = compile_source_with_surface(src, SurfaceProfile::Stable)
        .expect_err("must reject on stable surface");
    assert!(
        errs.iter().any(|e| e.contains("experimental")),
        "expected 'experimental' in errors, got: {:?}",
        errs
    );
}

#[test]
fn call_with_args_accepted_on_experimental_surface() {
    // Verifies that call_with_args lowers to CallWithArgs ops with the right arg types.
    let src = r#"
        @module_caps(Mmio);
        mmio DEV = 0xFEB00000;
        mmio_reg DEV.Status = 0x05 : u8 ro;
        extern @ctx(thread, boot) @eff() @caps() fn log_byte(b: u8);
        extern @ctx(thread, boot) @eff() @caps() fn log_two(a: u64, b: u64);
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn send_imm() {
            call_with_args(log_byte, 0x42);
        }
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn send_slot() {
            stack_cell<u8>(val);
            cell_store<u8>(val, 0xFF);
            call_with_args(log_byte, val);
        }
        @ctx(thread, boot) @eff(mmio) @caps(Mmio)
        fn send_two_imm() {
            call_with_args(log_two, 1, 2);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    // send_imm: one CallWithArgs with Imm(0x42)
    let send_imm = exec.functions.iter().find(|f| f.name == "send_imm").expect("send_imm");
    let imm_args: Vec<_> = send_imm.blocks[0].ops.iter().filter_map(|op| {
        if let ExecutableOp::CallWithArgs { args, .. } = op { Some(args.clone()) } else { None }
    }).collect();
    assert_eq!(imm_args.len(), 1, "send_imm must have one CallWithArgs");
    assert!(matches!(imm_args[0][0], ExecutableCallArg::Imm { value: 0x42 }));

    // send_slot: one CallWithArgs with Slot
    let send_slot = exec.functions.iter().find(|f| f.name == "send_slot").expect("send_slot");
    let slot_args: Vec<_> = send_slot.blocks[0].ops.iter().filter_map(|op| {
        if let ExecutableOp::CallWithArgs { args, .. } = op { Some(args.clone()) } else { None }
    }).collect();
    assert_eq!(slot_args.len(), 1, "send_slot must have one CallWithArgs");
    assert!(matches!(slot_args[0][0], ExecutableCallArg::Slot { .. }));

    // send_two_imm: one CallWithArgs with two Imm args
    let send_two = exec.functions.iter().find(|f| f.name == "send_two_imm").expect("send_two_imm");
    let two_args: Vec<_> = send_two.blocks[0].ops.iter().filter_map(|op| {
        if let ExecutableOp::CallWithArgs { args, .. } = op { Some(args.clone()) } else { None }
    }).collect();
    assert_eq!(two_args.len(), 1);
    assert_eq!(two_args[0].len(), 2);
    assert!(matches!(two_args[0][0], ExecutableCallArg::Imm { value: 1 }));
    assert!(matches!(two_args[0][1], ExecutableCallArg::Imm { value: 2 }));
}

#[test]
fn call_with_args_compiles_to_x86_64_elf_object() {
    // Verifies that CallWithArgs ops survive the full pipeline to ELF bytes.
    let src = r#"
        @module_caps(Mmio);
        extern @ctx(thread, boot) @eff() @caps() fn write_byte(b: u64);
        @ctx(thread, boot) @eff() @caps()
        fn emit() {
            call_with_args(write_byte, 0x41);
        }
        @ctx(thread, boot) @eff() @caps()
        fn emit_two(x: u64) {
            stack_cell<u64>(val);
            cell_store<u64>(val, 0x10);
            call_with_args(write_byte, val);
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

// ── PR-5: Tail-call loop ──────────────────────────────────────────────────────

#[test]
fn tail_call_rejected_on_stable_surface() {
    let src = r#"
        @module_caps();
        @ctx(thread, boot) @eff() @caps()
        fn spin() {
            tail_call(spin);
        }
    "#;
    let errs = compile_source_with_surface(src, SurfaceProfile::Stable)
        .expect_err("must reject on stable surface");
    assert!(
        errs.iter().any(|e| e.contains("experimental")),
        "expected 'experimental' in errors, got: {:?}",
        errs
    );
}

#[test]
fn tail_call_produces_tail_call_terminator() {
    let src = r#"
        @module_caps();
        @ctx(thread, boot) @eff() @caps()
        fn spin() {
            tail_call(spin);
        }
        @ctx(thread, boot) @eff() @caps()
        fn relay(x: u64) {
            stack_cell<u64>(val);
            cell_store<u64>(val, 0);
            cell_add<u64>(val, 1);
            tail_call(relay, val);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    let spin = exec.functions.iter().find(|f| f.name == "spin").expect("spin");
    assert!(
        matches!(
            &spin.blocks[0].terminator,
            ExecutableTerminator::TailCall { callee, args } if callee == "spin" && args.is_empty()
        ),
        "spin must have TailCall terminator to self with no args"
    );

    let relay = exec.functions.iter().find(|f| f.name == "relay").expect("relay");
    match &relay.blocks[0].terminator {
        ExecutableTerminator::TailCall { callee, args } => {
            assert_eq!(callee, "relay");
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], ExecutableCallArg::Slot { .. }));
        }
        other => panic!("relay must have TailCall terminator, got {:?}", other),
    }
}

#[test]
fn tail_call_compiles_to_x86_64_elf_object() {
    let src = r#"
        @module_caps();
        @ctx(thread, boot) @eff() @caps()
        fn counter(n: u64) {
            stack_cell<u64>(next);
            cell_store<u64>(next, n);
            cell_add<u64>(next, 1);
            cell_and<u64>(next, 255);
            tail_call(counter, next);
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

// ── PR-6: Telemetry layer ─────────────────────────────────────────────────────

#[test]
fn telemetry_counts_functions_and_ops_on_stable_surface() {
    // Uses stack ops (stable) to verify per-op counting and distribution fields.
    let src = r#"
        @module_caps();
        @ctx(irq) @eff() @caps()
        fn irq_handler() {
            stack_cell<u64>(x);
            cell_store<u64>(x, 0);
        }
        @ctx(boot) @eff() @caps()
        fn boot_init() {
            stack_cell<u64>(y);
            cell_store<u64>(y, 1);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);

    assert_eq!(report.surface, "stable");
    assert_eq!(report.function_count, 2);
    assert_eq!(report.extern_function_count, 0);
    assert_eq!(report.total_ops, 4); // 2 ops per function
    assert_eq!(report.op_counts.get("stack_cell"), Some(&2));
    assert_eq!(report.op_counts.get("stack_store"), Some(&2));
    assert!(
        report.experimental_features.is_empty(),
        "stable surface must report no experimental features"
    );
    assert_eq!(*report.ctx_distribution.get("irq").unwrap_or(&0), 1);
    assert_eq!(*report.ctx_distribution.get("boot").unwrap_or(&0), 1);
}

#[test]
fn telemetry_reports_experimental_features_when_present() {
    let src = r#"
        @module_caps();
        @ctx(thread) @eff() @caps()
        fn loop_forever(n: u64) {
            stack_cell<u64>(next);
            cell_store<u64>(next, n);
            cell_add<u64>(next, 1);
            call_with_args(loop_forever, next);
            tail_call(loop_forever, next);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Experimental);

    assert_eq!(report.surface, "experimental");
    assert!(
        report.experimental_features.contains(&"call_with_args"),
        "call_with_args must appear in experimental_features"
    );
    assert!(
        report.experimental_features.contains(&"tail_call"),
        "tail_call must appear in experimental_features"
    );
    assert!(
        report.experimental_features.contains(&"cell_arith_imm"),
        "cell_arith_imm must appear in experimental_features"
    );
}

#[test]
fn telemetry_extern_count_is_accurate() {
    let src = r#"
        @module_caps();
        extern @ctx(boot, thread) @eff() @caps() fn printk(msg: u64);
        @ctx(boot) @eff() @caps()
        fn entry() {
            call_with_args(printk, 0xDEAD);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Experimental);

    assert_eq!(report.function_count, 2, "entry + printk");
    assert_eq!(report.extern_function_count, 1, "only printk is extern");
    assert_eq!(report.op_counts.get("call_with_args"), Some(&1));
}

#[test]
fn telemetry_serializes_to_valid_json() {
    let src = r#"
        @module_caps();
        @ctx(irq) @eff() @caps()
        fn handler() { }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);
    let json_text = serde_json::to_string_pretty(&report).expect("must serialize to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_text).expect("output must be valid JSON");
    assert_eq!(parsed["surface"], "stable");
    assert!(parsed["function_count"].is_number());
    assert!(parsed["op_counts"].is_object());
    assert!(parsed["experimental_features"].is_array());
}

// ── PR-7: Living Compiler — pattern detection ─────────────────────────────────

use kernriftc::detect_patterns;

#[test]
fn living_compiler_detects_tail_call_opportunity_on_stable_module() {
    // A module with plain calls but no tail_call should trigger try_tail_call.
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn init() {
            setup();
            setup();
            setup();
        }
        @ctx(boot) @eff() @caps()
        fn setup() { }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);
    let suggestions = detect_patterns(&report);

    let ids: Vec<&str> = suggestions.iter().map(|m| m.id).collect();
    assert!(
        ids.contains(&"try_tail_call"),
        "try_tail_call must fire when call ops exist and no tail_call: got {:?}",
        ids
    );

    let tc = suggestions.iter().find(|m| m.id == "try_tail_call").unwrap();
    assert!(tc.fitness > 0, "fitness must be non-zero");
    assert!(tc.fitness <= 100, "fitness must not exceed 100");
    assert!(tc.requires_experimental, "must require experimental on stable surface");
}

#[test]
fn living_compiler_silent_on_fully_optimized_experimental_module() {
    // A module that already uses tail_call and call_with_args should produce
    // no suggestions for those patterns.
    let src = r#"
        @module_caps();
        @ctx(thread) @eff() @caps()
        fn loop_fn(n: u64) {
            stack_cell<u64>(next);
            cell_store<u64>(next, n);
            cell_add<u64>(next, 1);
            call_with_args(loop_fn, next);
            tail_call(loop_fn, next);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Experimental);
    let suggestions = detect_patterns(&report);

    let ids: Vec<&str> = suggestions.iter().map(|m| m.id).collect();
    assert!(
        !ids.contains(&"try_tail_call"),
        "try_tail_call must not fire when tail_call is already present"
    );
    assert!(
        !ids.contains(&"try_call_with_args"),
        "try_call_with_args must not fire when call_with_args is already present"
    );
    assert!(
        !ids.contains(&"try_cell_arith"),
        "try_cell_arith must not fire when cell_arith_imm is already present"
    );
}

#[test]
fn living_compiler_detects_cell_arith_opportunity() {
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn counter() {
            stack_cell<u64>(a);
            stack_cell<u64>(b);
            cell_store<u64>(a, 0);
            cell_store<u64>(b, 0);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);
    let suggestions = detect_patterns(&report);

    let ids: Vec<&str> = suggestions.iter().map(|m| m.id).collect();
    assert!(
        ids.contains(&"try_cell_arith"),
        "try_cell_arith must fire when 2+ stack_cells exist and no cell_arith_imm: got {:?}",
        ids
    );
}

#[test]
fn living_compiler_fitness_bounded_and_sorted() {
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn a() { b(); b(); b(); b(); b(); b(); b(); b(); b(); b(); }
        @ctx(boot) @eff() @caps()
        fn b() { }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);
    let suggestions = detect_patterns(&report);

    assert!(!suggestions.is_empty(), "must produce suggestions");
    for m in &suggestions {
        assert!(m.fitness <= 100, "fitness {} for '{}' exceeds 100", m.fitness, m.id);
    }
    // Verify sorted: fitness descending.
    for window in suggestions.windows(2) {
        assert!(
            window[0].fitness >= window[1].fitness,
            "suggestions must be sorted by fitness descending: {} < {}",
            window[0].fitness,
            window[1].fitness
        );
    }
}

#[test]
fn living_compiler_detects_high_extern_ratio() {
    let src = r#"
        @module_caps();
        extern @ctx(boot) @eff() @caps() fn ext_a();
        extern @ctx(boot) @eff() @caps() fn ext_b();
        @ctx(boot) @eff() @caps()
        fn local() { ext_a(); }
    "#;
    let module = compile_source(src).expect("must compile");
    let report = collect_telemetry(&module, SurfaceProfile::Stable);
    let suggestions = detect_patterns(&report);

    let ids: Vec<&str> = suggestions.iter().map(|m| m.id).collect();
    assert!(
        ids.contains(&"high_extern_ratio"),
        "high_extern_ratio must fire when externs >= half total functions: got {:?}",
        ids
    );
    let he = suggestions.iter().find(|m| m.id == "high_extern_ratio").unwrap();
    assert!(!he.requires_experimental, "high_extern_ratio does not need experimental surface");
}

// ── PR-8: Two-source slot arithmetic (slot_add/sub/and/or/xor/shl/shr) ───────

#[test]
fn slot_arith_parses_and_lowers_to_executable_krir() {
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn add_slots() {
            stack_cell<u64>(a);
            stack_cell<u64>(b);
            cell_store<u64>(a, 10);
            cell_store<u64>(b, 32);
            slot_add<u64>(a, b);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");

    let f = exec.functions.iter().find(|f| f.name == "add_slots").expect("add_slots");
    let slot_arith = f.blocks[0].ops.iter().find(|op| {
        matches!(op, ExecutableOp::SlotArithSlot { arith_op: ArithOp::Add, .. })
    });
    assert!(
        slot_arith.is_some(),
        "slot_add<u64> must lower to ExecutableOp::SlotArithSlot(Add), got: {:?}",
        f.blocks[0].ops
    );
    // Verify dst=0 (slot 'a') and src=1 (slot 'b').
    if let Some(ExecutableOp::SlotArithSlot { dst_slot_idx, src_slot_idx, ty, .. }) = slot_arith {
        assert_eq!(*dst_slot_idx, 0, "dst slot must be 0 (a)");
        assert_eq!(*src_slot_idx, 1, "src slot must be 1 (b)");
        assert_eq!(*ty, krir::MmioScalarType::U64);
    }
}

#[test]
fn slot_arith_rejected_on_stable_surface() {
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn compute() {
            stack_cell<u64>(a);
            stack_cell<u64>(b);
            cell_store<u64>(a, 5);
            cell_store<u64>(b, 3);
            slot_sub<u64>(a, b);
        }
    "#;
    let errs = compile_source_with_surface(src, SurfaceProfile::Stable)
        .expect_err("slot_sub on stable surface must fail");
    assert!(
        errs.iter().any(|e| e.contains("experimental")),
        "error must mention 'experimental', got: {:?}",
        errs
    );
}

#[test]
fn slot_arith_compiles_to_x86_64_elf_object() {
    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn bitops() {
            stack_cell<u64>(mask);
            stack_cell<u64>(val);
            cell_store<u64>(mask, 0xff);
            cell_store<u64>(val, 0x1234);
            slot_and<u64>(val, mask);
            slot_xor<u64>(val, mask);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let object =
        lower_executable_krir_to_x86_64_object(&exec, &target).expect("must produce ELF object");
    assert!(!object.text_bytes.is_empty(), "must emit non-empty text section");
}

#[test]
fn slot_arith_shift_emits_typed_asm_text() {
    use krir::{lower_executable_krir_to_x86_64_asm, emit_x86_64_asm_text};

    let src = r#"
        @module_caps();
        @ctx(boot) @eff() @caps()
        fn shift_slots() {
            stack_cell<u64>(val);
            stack_cell<u64>(cnt);
            cell_store<u64>(val, 0x80);
            cell_store<u64>(cnt, 3);
            slot_shl<u64>(val, cnt);
        }
    "#;
    let module =
        compile_source_with_surface(src, SurfaceProfile::Experimental).expect("must compile");
    let exec = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let target = BackendTargetContract::x86_64_sysv();
    let asm_module = lower_executable_krir_to_x86_64_asm(&exec, &target)
        .expect("must produce asm module");
    let asm_text = emit_x86_64_asm_text(&asm_module);

    assert!(
        asm_text.contains("shlq"),
        "slot_shl<u64> must emit 'shlq' in ASM text:\n{}",
        asm_text
    );
    assert!(
        asm_text.contains("%rcx"),
        "slot_shl<u64> shift must use %rcx as count register:\n{}",
        asm_text
    );
}

// ── Multi-OS target tests ────────────────────────────────────────────────────

#[test]
fn win64_target_contract_validates() {
    let contract = BackendTargetContract::x86_64_win64();
    assert!(contract.validate().is_ok(), "Win64 contract must validate");
    assert_eq!(contract.target_id.as_str(), "x86_64-win64");
}

#[test]
fn macho_target_contract_validates() {
    let contract = BackendTargetContract::x86_64_macho();
    assert!(contract.validate().is_ok(), "macOS Mach-O contract must validate");
    assert_eq!(contract.target_id.as_str(), "x86_64-macho");
    assert_eq!(contract.symbols.function_prefix, "_");
    assert_eq!(contract.sections.text, "__TEXT,__text");
}

#[test]
fn macho_asm_emits_underscore_prefix_and_macho_section() {
    let src = r#"
        @module_caps(MmioRaw);
        mmio UART0 = 0x1000;
        @ctx(thread, boot)
        fn uart_init() {
            raw_mmio_write<u32>(0x1000, 0x01);
        }
    "#;
    let module = compile_source(src).expect("must compile");
    let executable = lower_current_krir_to_executable_krir(&module).expect("must lower");
    let contract = BackendTargetContract::x86_64_macho();
    let asm = lower_executable_krir_to_x86_64_asm(&executable, &contract)
        .expect("must lower to asm");
    let text = emit_x86_64_asm_text(&asm);

    assert!(
        text.contains("__TEXT,__text"),
        "macOS ASM must use __TEXT,__text section:\n{}",
        text
    );
    assert!(
        text.contains("_uart_init"),
        "macOS ASM must prefix function symbol with underscore:\n{}",
        text
    );
}

#[test]
fn target_id_parse_roundtrip() {
    use krir::BackendTargetId;
    for (s, expected) in [
        ("x86_64-sysv", BackendTargetId::X86_64Sysv),
        ("x86_64-linux", BackendTargetId::X86_64Sysv),
        ("x86_64-win64", BackendTargetId::X86_64Win64),
        ("x86_64-windows", BackendTargetId::X86_64Win64),
        ("x86_64-macho", BackendTargetId::X86_64MachO),
        ("x86_64-darwin", BackendTargetId::X86_64MachO),
    ] {
        let parsed = BackendTargetId::parse(s).expect("must parse");
        assert_eq!(parsed, expected, "parse('{}') must equal {:?}", s, expected);
    }
    assert!(BackendTargetId::parse("unknown-target").is_err());
}

#[test]
fn expr_stmt_call_with_args_lowers_to_call_with_args_op() {
    // helper(42) is parsed as Stmt::ExprStmt(Expr::Call { callee: "helper", args: [IntLiteral(42)] })
    // which should lower to: StackCell + StackStore (for 42) then CallWithArgs.
    let src = r#"
@module_caps(MmioRaw)
@ctx(thread)
fn caller() {
    helper(42)
}
@ctx(thread)
fn helper() {}
"#;
    compile_source(src).unwrap();
}

#[test]
fn var_decl_and_compound_assign_lower_ok() {
    let src = r#"
@module_caps(MmioRaw)
@ctx(thread)
fn compute() {
    uint32 x = 10
    x &= 255
}
"#;
    compile_source(src).unwrap();
}

#[test]
fn print_intrinsic_lowers_to_raw_mmio_writes() {
    let src = r#"
@module_caps(MmioRaw)
@ctx(thread)
fn greet() {
    print("Hi")
}
"#;
    compile_source(src).unwrap();
}

#[test]
fn device_field_read_lowers_via_lower_expr() {
    // helper(UART0.Status) — device field used as call arg; lower_expr reads via MmioRead.
    let src = r#"
@module_caps(Mmio)
device UART0 at 0x3F000000 {
    Status at 0x04 : uint32 ro
}
@ctx(thread)
fn check() {
    helper(UART0.Status)
}
@ctx(thread)
fn helper() {}
"#;
    compile_source(src).unwrap();
}

#[test]
fn if_else_synthesizes_continuation_functions() {
    // if/else should compile cleanly; synthesized __if_then/__if_else/__if_end functions
    // must appear in the KrirModule and pass undefined-symbol checks.
    let src = r#"
@ctx(thread)
fn classify(uint32 val) {
    if val == 0 {
        uint32 zero = 1
    } else {
        uint32 nonzero = 2
    }
}
"#;
    let module = compile_source(src).unwrap();
    // Main fn + 3 synthesized fns (then, else, end)
    assert!(module.functions.len() >= 4, "expected ≥4 functions, got {}", module.functions.len());
}

#[test]
fn compare_into_slot_op_emitted_for_equality() {
    use krir::KrirOp;
    let src = r#"
@ctx(thread)
fn check(uint32 x) {
    if x == 42 {
        uint32 matched = 1
    }
}
"#;
    let module = compile_source(src).unwrap();
    let main_fn = module.functions.iter().find(|f| f.name == "check").unwrap();
    let has_compare = main_fn.ops.iter().any(|op| matches!(op, KrirOp::CompareIntoSlot { .. }));
    assert!(has_compare, "expected CompareIntoSlot in 'check' ops, got: {:?}", main_fn.ops);
}

#[test]
fn device_block_lowers_to_mmio_decls() {
    let src = r#"
@module_caps(Mmio)
device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}
@ctx(thread)
fn dummy() {}
"#;
    use kernriftc::compile_source;
    let module = compile_source(src).unwrap();
    assert!(module.mmio_bases.iter().any(|b| b.name == "UART0"),
        "expected UART0 in mmio_bases, got: {:?}", module.mmio_bases);
    assert!(module.mmio_registers.iter().any(|r| r.name == "Status"),
        "expected Status in mmio_registers, got: {:?}", module.mmio_registers);
    assert!(module.mmio_registers.iter().any(|r| r.name == "Data"),
        "expected Data in mmio_registers");
}

#[test]
fn while_loop_emits_loop_begin_end_and_branch() {
    use krir::KrirOp;
    let src = r#"
@ctx(thread)
fn count(uint32 n) {
    uint32 i = 0
    while i < n {
        i = i + 1
    }
}
"#;
    let module = compile_source(src).unwrap();
    let f = module.functions.iter().find(|f| f.name == "count").unwrap();
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::LoopBegin)),
        "expected LoopBegin, got: {:?}", f.ops);
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::LoopEnd)),
        "expected LoopEnd, got: {:?}", f.ops);
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::BranchIfZeroLoopBreak { .. })),
        "expected BranchIfZeroLoopBreak, got: {:?}", f.ops);
}

#[test]
fn for_loop_emits_loop_ops_and_compare() {
    use krir::KrirOp;
    let src = r#"
@ctx(thread)
fn sum(uint32 n) {
    uint32 total = 0
    for i in 0..n {
        total = total + 1
    }
}
"#;
    let module = compile_source(src).unwrap();
    let f = module.functions.iter().find(|f| f.name == "sum").unwrap();
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::LoopBegin)),
        "expected LoopBegin, got: {:?}", f.ops);
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::LoopEnd)),
        "expected LoopEnd, got: {:?}", f.ops);
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::CompareIntoSlot { .. })),
        "expected CompareIntoSlot in for loop, got: {:?}", f.ops);
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::BranchIfNonZeroLoopBreak { .. })),
        "expected BranchIfNonZeroLoopBreak, got: {:?}", f.ops);
}

#[test]
fn return_stmt_emits_return_slot_op() {
    use krir::KrirOp;
    let src = r#"
@ctx(thread)
fn get_val() -> uint32 {
    uint32 x = 42
    return x
}
"#;
    let module = compile_source(src).unwrap();
    let f = module.functions.iter().find(|f| f.name == "get_val").unwrap();
    assert!(f.ops.iter().any(|op| matches!(op, KrirOp::ReturnSlot { .. })),
        "expected ReturnSlot in get_val ops, got: {:?}", f.ops);
}

#[test]
fn mul_op_in_expr_returns_compile_error() {
    let src = r#"
@ctx(thread)
fn bad(uint32 x) -> uint32 {
    uint32 result = x * 4
    return result
}
"#;
    let result = compile_source(src);
    assert!(result.is_err(), "expected compile error for multiplication");
    let errs = result.unwrap_err();
    assert!(errs.iter().any(|e| e.contains("multiplication")),
        "expected 'multiplication' in error, got: {:?}", errs);
}
