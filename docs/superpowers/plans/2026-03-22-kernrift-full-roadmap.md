# KernRift Full Vision Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** This is a master roadmap. Phase 1 is fully task-detailed and executable now. Phases 2–6 are spec-level; each needs its own detailed plan (`docs/superpowers/plans/YYYY-MM-DD-<phase>.md`) before execution.

**Goal:** Bring KernRift from its current KR0-complete state to the full original vision: a kernel-first systems language with raw memory access, kernel intrinsics, C interop, a portable execution model, and a governed self-evolving surface layer.

**Architecture:** Six sequential phases, each producing working software. Phases 1–2 close the language primitive gaps. Phase 3 delivers the portable `.krbo` loader (already in progress). Phases 4–5 complete the Living Compiler vision. Phase 6 closes KR2/KR3 real-integration work.

**Tech Stack:** Rust, existing `parser`/`krir`/`passes`/`emit`/`kernriftc` crates, `libc`/`windows-sys` (loader), no new external dependencies for the language itself.

---

## What already exists (do not re-implement)

Before reading further, know what is already implemented so you do not duplicate it:

- `unsafe { }` — parser and KRIR-level syntax exists (`Stmt::Unsafe`, token `unsafe`). Current semantics: suppresses `MmioRaw` cap requirement. **Needs extension** to also cover raw pointer deref (Phase 1).
- `critical { }` — fully implemented (`CriticalEnter`/`CriticalExit` ops, passes enforcing allocation/block/yield restrictions).
- `yieldpoint()`, `AllocPoint`, `BlockPoint` — fully implemented as `KrirOp` variants.
- `@noyield`, `@lock_budget(N)`, `@leaf`, `@hotpath` — `FunctionAttrs` struct has all of these.
- Scheduler hook check — `sched_hook_check()` pass exists in `passes/src/lib.rs`.
- Lock graph cycle check — fully implemented.
- Living Compiler infrastructure — `AdaptiveSurfaceFeature`, `AdaptiveFeatureProposal`, `SurfaceProfile` (Stable/Experimental), migration tables all exist in `hir/src/lib.rs`.
- Per-cpu variables — `percpu NAME: TYPE`, `percpu_read<T>`, `percpu_write<T>` — parser, KRIR ops, and `%gs`-relative codegen.
- MMIO — `device` blocks and `raw_read<T>`/`raw_write<T>` fully implemented.
- Extern fn (import) — fully implemented.

---

## Codebase map (key files)

| File | Lines | Role |
|------|-------|------|
| `crates/parser/src/lib.rs` | 5,029 | Lexer + parser → `Stmt` AST (23 variants) |
| `crates/hir/src/lib.rs` | 5,111 | Adaptive feature mgmt, Living Compiler infra |
| `crates/krir/src/lib.rs` | 10,960 | KRIR IR (`KrirOp` 40 variants), x86-64 lowering, binary emission |
| `crates/passes/src/lib.rs` | 1,515 | 8 semantic passes (ctx/effect/cap/lock/sched) |
| `crates/emit/src/lib.rs` | 2,572 | JSON serialization (contracts, lockgraph, reports) |
| `crates/kernriftc/src/lib.rs` | ~1,340 | Compiler driver, artifact dispatch |
| `crates/kernriftc/src/runner.rs` | 53 | `kernrift` runner (subprocess launcher, to be replaced in Phase 3) |
| `crates/kernriftc/tests/kr0_contract.rs` | 1,095 | Main test suite (must_pass + must_fail) |

---

## Phase 1: Core Language Primitives — Raw Pointers + Kernel Intrinsics + C Export

**Covers:** The true KR0 gaps. Unsafe blocks already exist syntactically; this phase extends their semantics to enable raw memory access and kernel instruction emission. Produces a language you can write a real freestanding kernel driver in.

**Exit criteria:**
- `unsafe { *addr = val }` compiles and emits a store instruction
- `unsafe { asm!(cli) }` emits the `0xFA` byte
- `@export fn foo()` marks the symbol as the public C ABI boundary
- Raw pointer deref outside `unsafe` is a compile error
- `cargo test --workspace` passes

---

### Task 1: `UnsafeEnter` / `UnsafeExit` markers in KRIR

**Files:**
- Modify: `crates/krir/src/lib.rs`

Currently `Stmt::Unsafe(inner)` lowers by changing a validation context flag — there are no KRIR markers. We need markers so the passes can enforce that raw pointer ops only appear inside unsafe blocks (same pattern as `CriticalEnter`/`CriticalExit`).

- [ ] **Step 1: Write a failing test**

In `crates/krir/src/lib.rs`, at the bottom of the `#[cfg(test)]` block, add:

```rust
#[test]
fn krir_unsafe_markers_round_trip() {
    // A function with UnsafeEnter/UnsafeExit should survive validation
    let mut f = Function {
        name: "test".to_string(),
        is_extern: false,
        params: vec![],
        ctx_ok: vec![Ctx::Thread],
        eff_used: vec![],
        caps_req: vec![],
        attrs: FunctionAttrs::default(),
        ops: vec![
            KrirOp::UnsafeEnter,
            KrirOp::UnsafeExit,
        ],
    };
    // Should not panic or error — these are no-op markers
    assert_eq!(f.ops.len(), 2);
}
```

Run: `cargo test -p krir krir_unsafe_markers_round_trip 2>&1 | head -10`
Expected: FAIL — `KrirOp::UnsafeEnter` does not exist yet.

- [ ] **Step 2: Run test to verify it fails**

Run the command above.

- [ ] **Step 3: Add `UnsafeEnter` and `UnsafeExit` to `KrirOp`**

In `crates/krir/src/lib.rs`, locate the `KrirOp` enum. After the `CriticalEnter` and `CriticalExit` variants, add:

```rust
/// Marks the entry of an unsafe block. No code is emitted.
UnsafeEnter,
/// Marks the exit of an unsafe block. No code is emitted.
UnsafeExit,
```

In the x86-64 lowering match arm for `KrirOp` (look for `lower_executable_krir_to_compiler_owned_object` or the codegen match), add:

```rust
KrirOp::UnsafeEnter | KrirOp::UnsafeExit => {
    // No code emitted — markers only.
}
```

In any serialization / JSON emission that iterates `KrirOp`, add the two new variants (they serialize as `"UnsafeEnter"` and `"UnsafeExit"` strings, same pattern as `CriticalEnter`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p krir krir_unsafe_markers_round_trip 2>&1`
Expected: PASS

- [ ] **Step 5: Run full krir tests**

Run: `cargo test -p krir 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 6: Update the lowering of `Stmt::Unsafe` in the parser/lowering path**

The `Stmt::Unsafe(inner)` lowering lives in `crates/hir/src/lib.rs` at the `lower_stmt` function (around line 2846) — **not** in `krir/src/lib.rs`. Change the match arm there to emit `UnsafeEnter` at the start of the block and `UnsafeExit` at the end, in addition to the existing `MmioValidationCtx` flag logic:

```rust
Stmt::Unsafe(inner_stmts) => {
    ops.push(KrirOp::UnsafeEnter);
    let prev = ctx.module_allows_raw_mmio_literals;
    ctx.module_allows_raw_mmio_literals = true;
    for stmt in inner_stmts {
        lower_stmt(stmt, ops, ctx);
    }
    ctx.module_allows_raw_mmio_literals = prev;
    ops.push(KrirOp::UnsafeExit);
}
```

- [ ] **Step 7: Commit**

```bash
git add crates/krir/src/lib.rs crates/hir/src/lib.rs
git commit -m "feat(krir): add UnsafeEnter/UnsafeExit KRIR markers for unsafe block tracking"
```

---

### Task 2: Raw pointer load and store ops in KRIR

**Files:**
- Modify: `crates/krir/src/lib.rs`

Add `RawPtrLoad` and `RawPtrStore` ops. These take an address from a named slot (a `uint64` variable), not a literal address. This is how `*ptr = val` and `val = *ptr` lower.

- [ ] **Step 1: Write a failing test**

In `crates/krir/src/lib.rs` `#[cfg(test)]` block:

```rust
#[test]
fn raw_ptr_load_op_exists() {
    // Verify the KrirOp variant exists and is constructible.
    // Full codegen is tested via the kernriftc integration test below.
    let op = KrirOp::RawPtrLoad {
        ty: MmioScalarType::U32,
        addr_slot: "p".to_string(),
        out_slot: "v".to_string(),
    };
    let op2 = KrirOp::RawPtrStore {
        ty: MmioScalarType::U32,
        addr_slot: "p".to_string(),
        value: MmioValueExpr::IntLiteral(42),
    };
    // Just verify construction — no panic means the types are correct
    match op { KrirOp::RawPtrLoad { .. } => {} _ => panic!("wrong variant") }
    match op2 { KrirOp::RawPtrStore { .. } => {} _ => panic!("wrong variant") }
}
```

Run: `cargo test -p krir raw_ptr_load_lowers_to_mov 2>&1 | head -10`
Expected: FAIL — `ExecutableOp::RawPtrLoad` does not exist.

- [ ] **Step 2: Run test to verify it fails**

Run the command above.

- [ ] **Step 3: Add `RawPtrLoad` and `RawPtrStore` to `KrirOp` and `ExecutableOp`**

In `crates/krir/src/lib.rs`, in the `KrirOp` enum, add after `RawMmioWrite`:

```rust
/// Load a scalar value from an address stored in a named slot.
/// Only valid inside an unsafe block (enforced by passes).
RawPtrLoad {
    ty: MmioScalarType,
    addr_slot: String,  // KrirOp level uses String names (resolved to u8 indices later)
    out_slot: String,
},
/// Store a scalar value to an address stored in a named slot.
/// Only valid inside an unsafe block (enforced by passes).
RawPtrStore {
    ty: MmioScalarType,
    addr_slot: String,
    value: MmioValueExpr,
},
```

In `ExecutableOp` (the lowered x86-64 op enum), slot names have already been resolved to stack frame indices (`u8`). Add:

```rust
RawPtrLoad  { ty: MmioScalarType, addr_slot_idx: u8, out_slot_idx: u8 },
RawPtrStore { ty: MmioScalarType, addr_slot_idx: u8, value: MmioValueExpr },
```

In the `KrirOp` → `ExecutableOp` lowering function (look for where other `String`-named slot ops like `StackLoad` are resolved via `cell_slot_map`), add:

```rust
KrirOp::RawPtrLoad { ty, addr_slot, out_slot } => {
    let addr_idx = *cell_slot_map.get(&addr_slot).expect("addr_slot in map");
    let out_idx  = *cell_slot_map.get(&out_slot).expect("out_slot in map");
    ops.push(ExecutableOp::RawPtrLoad { ty, addr_slot_idx: addr_idx, out_slot_idx: out_idx });
}
KrirOp::RawPtrStore { ty, addr_slot, value } => {
    let addr_idx = *cell_slot_map.get(&addr_slot).expect("addr_slot in map");
    ops.push(ExecutableOp::RawPtrStore { ty, addr_slot_idx: addr_idx, value });
}
```

- [ ] **Step 4: Add x86-64 codegen for `RawPtrLoad` and `RawPtrStore`**

Two places to update:

**4a. `executable_op_encoded_len`** — this function (search for it by name in `krir/src/lib.rs`) is an exhaustive match used to precompute code size. Adding new `ExecutableOp` variants without updating it is a compile error. Add:

```rust
ExecutableOp::RawPtrLoad { ty, .. } => {
    // movabs rax, [rbp+offset] (load addr, 7 bytes) + mov reg, [rax] (3-4 bytes)
    7 + mmio_load_from_reg_len(ty)
}
ExecutableOp::RawPtrStore { ty, .. } => {
    // movabs rax, [rbp+offset] (load addr, 7 bytes) + mov [rax], reg (3-4 bytes)
    7 + mmio_store_to_reg_len(ty)
}
```

Use the same size helper functions as `RawMmioRead`/`RawMmioWrite` — the instruction shapes are identical except the address comes from a register rather than an immediate.

**4b. The codegen emit match** — add alongside `RawMmioRead`/`RawMmioWrite`:

```rust
ExecutableOp::RawPtrLoad { ty, addr_slot_idx, out_slot_idx } => {
    // Load the pointer address from the stack frame slot into RAX
    emit_stack_slot_to_rax(*addr_slot_idx, code);
    // MOV [size_prefix] out_slot_reg, [RAX]
    emit_load_from_rax_indirect(ty, *out_slot_idx, &slot_layout, code);
}
ExecutableOp::RawPtrStore { ty, addr_slot_idx, value } => {
    emit_stack_slot_to_rax(*addr_slot_idx, code);
    emit_store_to_rax_indirect(ty, value, &slot_layout, code);
}
```

Follow the exact same pattern as `RawMmioRead`/`RawMmioWrite` — only difference is address source (slot vs. immediate).

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p krir raw_ptr_load_lowers_to_mov 2>&1`
Expected: PASS

- [ ] **Step 6: Run full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add crates/krir/src/lib.rs
git commit -m "feat(krir): add RawPtrLoad/RawPtrStore ops for raw pointer access in unsafe blocks"
```

---

### Task 3: Surface syntax for raw pointer deref

**Files:**
- Modify: `crates/parser/src/lib.rs`

Add `Stmt::PtrLoad` and `Stmt::PtrStore` to the parser, triggered by `*var` (load) and `*var = expr` (store) syntax. Both must only appear inside `unsafe { }` (enforced by the pass in Task 4 — parser does not enforce context here).

Also add `Stmt::PtrCast` for `expr as *T` (cast integer to pointer) and `expr as uint64` (cast pointer to integer) — these are common in kernel code.

- [ ] **Step 1: Write a failing test**

In `crates/kernriftc/tests/kr0_contract.rs`, add to the must_pass section:

```rust
#[test]
fn ptr_deref_in_unsafe_compiles() {
    let src = r#"
@ctx(thread)
fn entry() {
    unsafe {
        uint64 addr = 0x1000
        uint32 val = 0
        *addr -> val  // load uint32 from addr into val
        *addr = 42u32 // store 42 to addr
    }
}
"#;
    let result = kernriftc::compile_source(src);
    assert!(result.is_ok(), "ptr deref in unsafe should compile: {:?}", result);
}
```

Run: `cargo test -p kernriftc ptr_deref_in_unsafe_compiles 2>&1 | head -10`
Expected: FAIL — syntax not recognized.

- [ ] **Step 2: Run test to verify it fails**

Run the command above.

- [ ] **Step 3: Add ptr syntax to the parser**

In `crates/parser/src/lib.rs`, add to the `Stmt` enum:

```rust
/// Load through raw pointer: `*addr_var -> out_var`
PtrLoad {
    ty: MmioScalarType,
    addr_var: String,
    out_var: String,
},
/// Store through raw pointer: `*addr_var = value`
PtrStore {
    ty: MmioScalarType,
    addr_var: String,
    value: Box<Expr>,
},
```

In the token-based parser (`TokParser`), add parsing for these forms in the statement parser. The syntax:
- `*IDENT -> IDENT` — ptr load (reads from address in first ident into second ident)
- `*IDENT = expr` — ptr store

The type for ptr load/store is inferred from context or specified with a suffix: `*addr as uint32 -> val`. For the MVP, default to `uint64` if unspecified, or require explicit type cast syntax.

In the lowering from `Stmt` to `KrirOp`, add:

```rust
Stmt::PtrLoad { ty, addr_var, out_var } => {
    ops.push(KrirOp::RawPtrLoad {
        ty,
        addr_slot: addr_var,
        out_slot: out_var,
    });
}
Stmt::PtrStore { ty, addr_var, value } => {
    let value_expr = lower_expr_to_mmio_value(value)?;
    ops.push(KrirOp::RawPtrStore {
        ty,
        addr_slot: addr_var,
        value: value_expr,
    });
}
```

- [ ] **Step 4: Add must_fail case for ptr deref outside unsafe**

```rust
#[test]
fn ptr_deref_outside_unsafe_is_error() {
    let src = r#"
@ctx(thread)
fn entry() {
    uint64 addr = 0x1000
    uint32 val = 0
    *addr -> val  // should fail: not inside unsafe
}
"#;
    let result = kernriftc::compile_source(src);
    assert!(result.is_err(), "ptr deref outside unsafe must be rejected");
}
```

(This test will be used after Task 4 adds the pass enforcement.)

- [ ] **Step 5: Run tests**

Run: `cargo test -p kernriftc ptr_deref 2>&1`
Expected: `ptr_deref_in_unsafe_compiles` PASS, `ptr_deref_outside_unsafe_is_error` FAIL (pass not yet added).

- [ ] **Step 6: Commit**

```bash
git add crates/parser/src/lib.rs crates/hir/src/lib.rs crates/krir/src/lib.rs
git commit -m "feat(parser,hir,krir): add raw pointer deref syntax (*addr -> slot and *addr = val)"
```

---

### Task 4: Passes — enforce unsafe boundary for raw pointer ops

**Files:**
- Modify: `crates/passes/src/lib.rs`

Add `unsafe_ptr_check()`: scan each function's ops, track whether currently inside an `UnsafeEnter`/`UnsafeExit` region (identical to how `critical_alloc_boundary_check` tracks `CriticalEnter`/`CriticalExit`), reject `RawPtrLoad`/`RawPtrStore` outside unsafe.

- [ ] **Step 1: Write a failing test**

In `crates/kernriftc/tests/kr0_contract.rs`, the `ptr_deref_outside_unsafe_is_error` test from Task 3 Step 4 should now pass after this task. Verify it currently fails:

Run: `cargo test -p kernriftc ptr_deref_outside_unsafe_is_error 2>&1`
Expected: FAIL — the pass does not exist yet so it compiles when it should not.

- [ ] **Step 2: Add `unsafe_ptr_check` to `passes/src/lib.rs`**

```rust
pub fn unsafe_ptr_check(module: &KrirModule) -> Vec<CheckError> {
    let mut errors = Vec::new();
    for function in &module.functions {
        let mut depth: usize = 0;
        for op in &function.ops {
            match op {
                KrirOp::UnsafeEnter => depth += 1,
                KrirOp::UnsafeExit => {
                    if depth > 0 { depth -= 1; }
                }
                KrirOp::RawPtrLoad { .. } | KrirOp::RawPtrStore { .. } => {
                    if depth == 0 {
                        errors.push(CheckError {
                            pass: "unsafe_ptr_check",
                            message: format!(
                                "function '{}': raw pointer access outside unsafe block",
                                function.name
                            ),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    errors
}
```

Wire it into `analyze_module()` alongside the other checks.

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p kernriftc ptr_deref_outside_unsafe_is_error 2>&1`
Expected: PASS

- [ ] **Step 4: Run full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/passes/src/lib.rs
git commit -m "feat(passes): add unsafe_ptr_check — raw pointer access requires unsafe block"
```

---

### Task 5: Kernel intrinsic instructions (`asm!`)

**Files:**
- Modify: `crates/parser/src/lib.rs` — add `asm!` parsing, `KernelIntrinsic` enum
- Modify: `crates/krir/src/lib.rs` — add `KrirOp::InlineAsm(KernelIntrinsic)`, codegen
- Modify: `crates/passes/src/lib.rs` — enforce `InlineAsm` only inside unsafe

This implements named kernel intrinsics as the MVP inline assembly. No operands, no input/output constraints — just the common no-argument kernel instructions that have known encodings.

**Supported intrinsics and their x86-64 encodings:**

| Name | Bytes | Purpose |
|------|-------|---------|
| `cli` | `[0xFA]` | Disable interrupts |
| `sti` | `[0xFB]` | Enable interrupts |
| `hlt` | `[0xF4]` | Halt (used in idle loop) |
| `nop` | `[0x90]` | No-operation |
| `mfence` | `[0x0F, 0xAE, 0xF0]` | Full memory fence |
| `sfence` | `[0x0F, 0xAE, 0xF8]` | Store fence |
| `lfence` | `[0x0F, 0xAE, 0xE8]` | Load fence |
| `wbinvd` | `[0x0F, 0x09]` | Write-back and invalidate cache |
| `pause` | `[0xF3, 0x90]` | Spin-wait hint |
| `int3` | `[0xCC]` | Breakpoint trap |
| `cpuid` | `[0x0F, 0xA2]` | CPU identification (EAX/EBX/ECX/EDX) |

- [ ] **Step 1: Write a failing test**

```rust
#[test]
fn asm_cli_in_unsafe_compiles() {
    let src = r#"
@ctx(irq)
fn irq_handler() {
    unsafe {
        asm!(cli)
    }
}
"#;
    let result = kernriftc::compile_source(src);
    assert!(result.is_ok(), "asm!(cli) in unsafe should compile: {:?}", result);
}

#[test]
fn asm_outside_unsafe_is_error() {
    let src = r#"
@ctx(thread)
fn entry() {
    asm!(sti)
}
"#;
    let result = kernriftc::compile_source(src);
    assert!(result.is_err(), "asm! outside unsafe must be rejected");
}
```

Run: `cargo test -p kernriftc asm_cli_in_unsafe_compiles 2>&1 | head -5`
Expected: FAIL — `asm!` not recognized.

- [ ] **Step 2: Add `KernelIntrinsic` enum to the parser**

In `crates/parser/src/lib.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelIntrinsic {
    Cli, Sti, Hlt, Nop,
    Mfence, Sfence, Lfence,
    Wbinvd, Pause, Int3, Cpuid,
}
```

Add `Stmt::InlineAsm(KernelIntrinsic)` to the `Stmt` enum.

Parse `asm!(NAME)` in the statement parser — `NAME` is matched case-insensitively against the intrinsic list. Unknown names are a parse error: `"unknown kernel intrinsic 'NAME'; supported: cli, sti, hlt, nop, mfence, sfence, lfence, wbinvd, pause, int3, cpuid"`.

- [ ] **Step 3: Add `KrirOp::InlineAsm` and codegen**

In `crates/krir/src/lib.rs`:

```rust
// In KrirOp enum:
/// Emit a named kernel intrinsic instruction.
/// Only valid inside an unsafe block.
InlineAsm(KernelIntrinsic),
```

In the lowering from `Stmt::InlineAsm(intr)` → `KrirOp::InlineAsm(intr)`.

In the x86-64 codegen match:

```rust
KrirOp::InlineAsm(intr) | ExecutableOp::InlineAsm(intr) => {
    let bytes: &[u8] = match intr {
        KernelIntrinsic::Cli    => &[0xFA],
        KernelIntrinsic::Sti    => &[0xFB],
        KernelIntrinsic::Hlt    => &[0xF4],
        KernelIntrinsic::Nop    => &[0x90],
        KernelIntrinsic::Mfence => &[0x0F, 0xAE, 0xF0],
        KernelIntrinsic::Sfence => &[0x0F, 0xAE, 0xF8],
        KernelIntrinsic::Lfence => &[0x0F, 0xAE, 0xE8],
        KernelIntrinsic::Wbinvd => &[0x0F, 0x09],
        KernelIntrinsic::Pause  => &[0xF3, 0x90],
        KernelIntrinsic::Int3   => &[0xCC],
        KernelIntrinsic::Cpuid  => &[0x0F, 0xA2],
    };
    code.extend_from_slice(bytes);
}
```

Extend `unsafe_ptr_check` from Task 4 to also reject `InlineAsm` outside unsafe blocks — add `KrirOp::InlineAsm(_)` to the same depth-guarded match arm. Do not create a separate pass; the depth tracking is already there.

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/parser/src/lib.rs crates/krir/src/lib.rs crates/passes/src/lib.rs
git commit -m "feat: add kernel intrinsic instructions (asm!(cli/sti/hlt/nop/mfence/...))"
```

---

### Task 6: C ABI export annotation (`@export`)

**Files:**
- Modify: `crates/parser/src/lib.rs` — parse `@export`
- Modify: `crates/krir/src/lib.rs` — `Function.is_export: bool`
- Modify: `crates/emit/src/lib.rs` — include exported symbols in caps manifest

All KernRift functions are already emitted as global ELF symbols. The `@export` annotation is:
1. A declaration of intent — "this function is the stable C-callable boundary"
2. A validation gate — exported functions are checked for C-compatible type signatures (no slice params yet, only scalars)

- [ ] **Step 1: Write a failing test**

```rust
#[test]
fn export_annotation_compiles() {
    let src = r#"
@export
@ctx(thread, boot)
fn init_driver(uint32 flags) -> uint32 {
    return flags
}
"#;
    let result = kernriftc::compile_source(src);
    assert!(result.is_ok(), "@export fn should compile: {:?}", result);
}
```

Run: `cargo test -p kernriftc export_annotation_compiles 2>&1 | head -5`
Expected: FAIL — `@export` not parsed.

- [ ] **Step 2: Add `@export` to the parser**

In `crates/parser/src/lib.rs`, add `TokenKind::Export` keyword. In the function annotation parser (where `@ctx`, `@eff`, `@leaf`, etc. are parsed), add recognition of `@export` → set `FunctionAnnotations.is_export = true`.

- [ ] **Step 3: Thread `is_export` through KRIR**

In `crates/krir/src/lib.rs`, add `is_export: bool` to `Function`. Default `false`. During lowering, pass through from the parsed annotation.

Add a validation: if `is_export = true`, verify all parameter types and return type are scalar (`MmioScalarType`) — no slices, no per-cpu. Add a `CheckError` if violated: `"exported function 'NAME' uses non-C-compatible type; only scalar types are allowed on @export functions"`.

- [ ] **Step 4: Surface exported symbols in the caps manifest**

In `crates/emit/src/lib.rs`, update `emit_caps_manifest_json` to include an `"exported_symbols"` array listing all `@export` functions by name. This gives tooling a machine-readable list of the C API surface.

- [ ] **Step 5: Run all tests**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/parser/src/lib.rs crates/hir/src/lib.rs crates/krir/src/lib.rs crates/emit/src/lib.rs
git commit -m "feat: add @export annotation for C ABI boundary declaration"
```

---

## Phase 2: Portable `.krbo` Loader

**Status:** Plan written at `docs/superpowers/plans/2026-03-22-krbo-portable-loader-design.md`. Detailed execution plan pending at `docs/superpowers/plans/2026-03-22-krbo-loader.md`.

**Deliverables:**
- `kernriftc hello.kr` → `hello.krbo` (16-byte KRBO header + raw x86-64 code)
- `kernrift hello.krbo` → executes on Linux, macOS, Windows without external tools
- No linker, no assembler, no C compiler required
- Same `.krbo` file runs on all x86-64 platforms

**Architecture reference:** `docs/superpowers/specs/2026-03-22-krbo-portable-loader-design.md`

---

## Phase 3: Living Compiler — Profile Enforcement

**Status:** Infrastructure exists in `hir/src/lib.rs` (`SurfaceProfile`, `AdaptiveSurfaceFeature`, `AdaptiveFeatureProposal`, migration tables). What's missing: actually enforcing profiles during compilation.

**Deliverables:**
- `#lang stable` / `#lang experimental` directive at top of `.kr` files
- Compiler flag `--profile stable|experimental|systems`
- Features gated by `surface_profile_gate: SurfaceProfile` in `AdaptiveSurfaceFeature` are rejected in the wrong profile
- `kernriftc --emit features` outputs the feature table as JSON
- Stable profile: reject experimental syntax with a clear migration hint

**Key files to modify:**
- `crates/parser/src/lib.rs` — parse `#lang PROFILE` directive
- `crates/krir/src/lib.rs` — pass `SurfaceProfile` through the pipeline
- `crates/passes/src/lib.rs` — new `profile_check` pass using `AdaptiveSurfaceFeature.surface_profile_gate`
- `crates/kernriftc/src/lib.rs` — `--profile` flag, default = `stable`

**Detailed plan:** Write `docs/superpowers/plans/YYYY-MM-DD-living-compiler-profiles.md` when ready to execute.

---

## Phase 4: Living Compiler — Evolution Engine

**Status:** `AdaptiveFeatureProposal` struct exists with full metadata schema. Nothing generates proposals or evaluates fitness.

**Deliverables:**
- Telemetry hooks: `kernriftc --telemetry` mode records feature usage counts in a local JSON log
- Proposal engine: reads telemetry + pattern library, generates candidate `AdaptiveFeatureProposal` entries
- Fitness evaluation: score proposals by readability delta, safety impact, adoption ratio, ambiguity risk, migration cost
- `kernriftc propose` command: run the engine, output ranked proposals
- Human governance gate: proposals require explicit approval before they become `AdaptiveFeatureStatus::Stable`

**Key new components:**
- `crates/telemetry/` (new crate): usage event logging, aggregation, pattern detection
- `crates/proposals/` (new crate): proposal generation, fitness scoring, ranking
- Governance state stored in `docs/superpowers/proposals/` as TOML files with approval metadata

**Detailed plan:** Write `docs/superpowers/plans/YYYY-MM-DD-living-compiler-evolution.md` when Phase 3 is complete.

---

## Phase 5: Living Compiler — Migration Engine

**Status:** Migration metadata exists (`migration_safe: bool`, `canonical_replacement: String`, `rewrite_intent: String` in `AdaptiveSurfaceFeature`). Nothing acts on it.

**Deliverables:**
- `kernriftc migrate FILE.kr` — rewrites deprecated surface forms to canonical replacements
- Idempotent (run twice, same result)
- Only migrates `migration_safe = true` features
- Produces a diff report: what was rewritten and why
- Version pinning: `.kr` files can declare `#lang 1.0`, `#lang 1.1`, etc.; compiler rejects files whose required version exceeds the current stable version

**Key new components:**
- `crates/migrate/` (new crate): AST rewriter using `canonical_replacement` and `rewrite_intent` from feature table
- Version manifest: `docs/versions/CHANGELOG.md` maps language versions to feature sets

**Detailed plan:** Write `docs/superpowers/plans/YYYY-MM-DD-living-compiler-migration.md` when Phase 4 is complete.

---

## Phase 6: KR2/KR3 — Real Integration

**Status:** Foundation exists (`lock_classes` field in `KrirModule`, `sched_hook_check` pass, `@hotpath` in `FunctionAttrs`). Frontend not wired.

**KR2 Deliverables:**
- Lock class declarations: `lock_class NAME { priority: N }` in `.kr` files
- `acquire`/`release` tied to classes, not just names
- Link-time lock graph merge: `kernriftc link *.o` merges per-object `lockgraph.json` and rejects global cycles
- Per-cpu enforcement: `%gs`-relative codegen validated against `percpu` declarations at the pass level
- Scheduler hook interfaces: `@hook(sched_in)` / `@hook(sched_out)` function annotations, validated by existing `sched_hook_check`

**KR3 Deliverables:**
- Hot-path compiler biasing: `@hotpath` on a function triggers inlining hints and layout prefetching in the x86-64 backend
- Whole-module dead code elimination: functions not reachable from `@export` or `@ctx(boot)` entry points are removed from the output object
- C ABI boundary hardening: `kernriftc link --emit-linker-script` produces a linker script fragment for mixing KernRift objects into a C/asm kernel build
- Performance + safety comparison report: `kernriftc report FILE.kr` emits a summary of rejected bug classes, max lock depth, no-yield spans, and estimated hot-path call cost

**Detailed plan:** Write `docs/superpowers/plans/YYYY-MM-DD-kr2-kr3.md` when Phase 3 is complete.

---

## Execution order

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5
                                         │
                                    Phase 6 (can start after Phase 3)
```

Phase 1 unblocks Phase 2 (portable loader needs the language to be complete enough to run real programs).
Phase 3 (profile enforcement) unblocks Phases 4 and 5 (evolution requires a stable/experimental split).
Phase 6 (KR2/KR3) can begin after Phase 3 — it does not depend on the evolution engine.

---

## Guardrails (from original spec, preserved)

- Keep scope at kernel/drivers core — no general-purpose ecosystem creep
- Prioritize deterministic builds and diagnostic quality above all else
- Maintain incremental migration path — never require a full rewrite
- Living Compiler changes are governed — no language change without approval
- Performance and safety wins must be measurable, not asserted
