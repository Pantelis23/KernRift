# KernRift Architecture (Initial)

## Scope

Freestanding, ahead-of-time language + compiler targeting kernel and driver development with strict interop and code-shape control.

## Compiler Pipeline

1. Front-end
- Lexer/parser
- AST + symbol resolution
- Type checker with effect and capability rules

2. Mid-end (KRIR)
- Control-flow graph in SSA-like form
- Effect facts attached to functions/basic blocks
- Lock-order metadata and callgraph summaries

3. OS-aware optimization passes
- Fast-path shaping for designated syscall/interrupt hot paths
- Stack-budget verification on exception/IRQ paths
- MMIO ordering validation and fence-requirement checks
- Callgraph pinning and section placement constraints

4. Backend
- LLVM backend first, with strict lowering constraints
- Native object emission (ELF), section control, symbol visibility
- Linker script integration for kernel builds

## Core Language Features (MVP)

- `unsafe` blocks for explicit escape hatches
- Structured MMIO declarations plus typed MMIO operations:
  - `mmio NAME = INT_LITERAL;`
  - `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;`
  - `mmio_read<T>(addr)` / `mmio_write<T>(addr, value)`
- Canonical surface facts: `@ctx(...)`, `@eff(...)`, `@caps(...)`, `@module_caps(...)`
- Canonical surface facts lower into KRIR facts (`ctx_ok`, `eff_used`, `caps_req`)
- Capabilities for privileged operations (I/O ports, page-table writes, IRQ routing)
- Lock-order declarations with compile-time cycle detection (or proof artifact emission)
- Execution-shaping primitives: `yieldpoint()`, `@noyield`, `lock_budget(N)`, `critical { ... }`

## Formal Semantics (KRIR Facts)

### Closed vocabularies (MVP)

- `ctx in {boot, thread, irq, nmi}`
- `effects in {alloc, block, preempt_off, ioport, mmio, dma_map, yield}`
- `capabilities in {Cap::PhysMap, Cap::PageTableWrite, Cap::IrqRoute, Cap::IoPort(range), Cap::Mmio(base,len), Cap::DmaMap(dev_id)}`

### Function fact model

Each function carries:

- `ctx_ok: CtxSet` (contexts where function may execute)
- `eff_used: EffSet` (effects function may perform)
- `caps_req: CapSet` (capabilities required to call/execute)
- region attrs: `@noyield`, `lock_budget(N)`, `@hotpath`, optional `@leaf`

Kernel-critical resources (frames, DMA buffers, transient mappings) are modeled with linear capabilities so ownership and revocation are explicit.

### Context policy model

Compiler builtins define per-context allowed effects:

- `eff_allowed(boot)`
- `eff_allowed(thread)`
- `eff_allowed(irq)`
- `eff_allowed(nmi)`

MVP baseline includes:

- `eff_allowed(irq)` excludes `alloc`, `block`, and `yield`
- `eff_allowed(nmi)` is at least as strict as IRQ

### Call-edge enforcement rule

For any call edge `caller -> callee`, compilation requires:

- `ctx_ok(caller) subset_of ctx_ok(callee)`
- for all `c in ctx_ok(caller)`: `eff_used(callee) subset_of eff_allowed(c)`
- `caps_avail(caller) superset_of caps_req(callee)`

Violation is compile error, not lint.

### MMIO op model (in KRIR/OSIR)

MMIO is represented as explicit IR ops:

- `MmioRead(addr, width, order)` where `order in {Relaxed, Acquire}`
- `MmioWrite(addr, value, width, order)` where `order in {Release, SeqCst}`
- `Fence(domain, kind)` where `domain in {mmio, cpu}`

Verification policy for MVP:

- Illegal width or access mode is compile error
- Missing required fence is compile error (no auto-insert in MVP)

### Lock graph model

- Every lock instance must declare a lock class
- Lock edges are emitted per object (`lockgraph.json`)
- Final link step merges all lock graphs and rejects cycles

### Yield/preemption semantics

- `@noyield` regions forbid any yield op
- `yieldpoint()` is legal only when not in IRQ context and not under a spinlock
- `lock_budget(N)` uses call-count metric:
- budget unit is 1 per call to a non-`@leaf` function
- for every path from `Acquire` to matching `Release`, call-count must be `<= N`
- build report includes `max_lock_depth` and `no_yield_spans`

### Lowering invariants

The following must survive lowering unchanged in meaning:

- section placement attributes
- calling convention attributes
- visibility and symbol linkage attributes
- `@hotpath` / layout pinning hints
- MMIO ordering semantics
- lock-class metadata references

## Non-Negotiable Interop

- C ABI import/export
- Inline assembly with explicit clobbers/constraints
- Custom calling conventions where required
- Exact section placement for boot, ISR tables, per-cpu segments, and linker-defined symbols

## Near-Term Risk Controls

- Keep language surface small until KR2
- Prefer explicit effects over inferred magic in safety-critical paths
- Treat backend output shape as a compatibility contract for kernel entry/interrupt code
