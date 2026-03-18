# KernRift KR0-KR3 Plan

## KR0 (Language + Freestanding Artifact)

### Deliverables

- Parser + type checker for structs, enums, pointers, slices
- `unsafe` blocks
- Structured MMIO declarations plus typed MMIO operations:
  - `mmio NAME = INT_LITERAL;`
  - `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;`
  - `mmio_read<T>(addr)` / `mmio_write<T>(addr, value)`
- Freestanding static library output (`.a`) and ELF object emission
- Facts-only mode:
- parse minimal syntax and attach `ctx_ok`, `eff_used`, `caps_req`
- emit stable KRIR JSON
- Deterministic codegen stability test (same source -> stable object metadata/code shape)

### Exit Criteria

- Build a tiny freestanding library callable from C
- MMIO sample compiles and emits expected volatile operations
- No runtime or libc dependency
- `kernriftc --emit krir` produces stable machine-readable KRIR facts

## KR1 (Driver Subset + Effects)

### Deliverables

- Canonical frontend facts: `@ctx(...)`, `@eff(...)`, `@caps(...)`
- Effect-aware call checking with builtin `eff_allowed(ctx)` policies
- Basic capability declarations for privileged ops
- `yieldpoint()`, `@noyield`, `critical { ... }`, and `lock_budget(N)` semantics + checks
- Negative-compile test suite (`must_fail`) for context/effect/capability violations

### Exit Criteria

- Compiler rejects:
  - blocking call from `@ctx(irq)`
  - allocation in disallowed context
  - yield in `@ctx(irq)`
  - missing capability for privileged operation
  - yield under spinlock or in IRQ context
  - yield inside `@noyield` region
- Minimal PCI/IRQ demo driver compiled as object

## KR2 (Kernel Module Demo)

### Deliverables

- Spinlock primitives with lock classes
- Link-time lock graph merge with per-object artifact emission (`lockgraph.json`)
- Global lock-order cycle rejection at final link
- Per-cpu data primitive and scheduler hook interfaces

### Exit Criteria

- Sample module integrates with existing kernel build flow
- Intentional lock-order inversion is compile-time failure
- Per-cpu access rules validated in effect checks
- Build emits worst-case lock/yield report (`max_lock_depth`, `no_yield_spans`)

## KR3 (Real Integration Experiment)

### Deliverables

- One meaningful subsystem implemented in KernRift
- Mixed-language boundary hardened (C ABI + linker script + symbols)
- Basic optimization passes for hot-path shaping
- Performance + safety comparison script against C baseline

### Exit Criteria

- End-to-end demo in either:
  - microkernel slice in KernRift, or
  - Linux-side / AOS-side kernel experiment
- Performance and safety report against C baseline
- Report includes compile-time rejected bug classes and latency deltas

## Guardrails

- Keep scope at kernel/drivers core, not general-purpose ecosystem
- Prioritize deterministic builds and diagnostics quality
- Maintain incremental migration path; avoid rewrite requirement

## CLI Naming

Compiler CLI name is frozen as `kernriftc` for docs and tooling consistency.
