# Kernel PR2 Inventory

## Existing Syntax We Reuse

### Context facts (`@ctx(...)`)

Parsed in `crates/hir/src/lib.rs`:

- `boot`
- `thread`
- `irq`
- `nmi`

`@ctx(...)` is lowered through HIR into KRIR `ctx_ok` and emitted in contracts `facts.symbols[*].ctx_ok`.

### Effect facts (`@eff(...)`)

Parsed in `crates/hir/src/lib.rs`:

- `alloc`
- `block`
- `preempt_off`
- `ioport`
- `mmio`
- `dma_map`
- `yield`

Also lowered into KRIR `eff_used` and emitted in contracts `facts.symbols[*].eff_used`.

## Yieldpoint Representation

- Parser maps `yieldpoint()` into `Stmt::YieldPoint` (`crates/parser/src/lib.rs`).
- HIR lowers it to `KrirOp::YieldPoint` and marks `Eff::Yield`.
- Interprocedural pass summaries propagate `has_yield` through call graph (`crates/passes/src/lib.rs`).
- `report.no_yield_spans` already captures bounded/unbounded behavior per thread-capable function.

## Alloc/Block Mapping Today

Current observability:

- We can observe `alloc`/`block` at function-fact level via `eff_used`.
- We do **not** currently have operation-level alloc/block site ops in KRIR.

PR2 mapping decision:

- contracts v2 `report.effects.alloc_sites_count` and `block_sites_count` are set to `0`.
- deterministic diagnostics are emitted in v2 mode:
  - `analysis: KERNEL_FEATURE_UNIMPLEMENTED: alloc_sites_count`
  - `analysis: KERNEL_FEATURE_UNIMPLEMENTED: block_sites_count`

This keeps outputs truthful and deterministic until op-level alloc/block sites are modeled.

## Critical Context Marker

For PR2, we reuse existing `@noyield` as the critical marker for kernel policy checks:

- contracts v2 `report.contexts.critical_functions` is derived from `facts.symbols[*].attrs.noyield`.
- kernel policy enforces `forbid_yield_in_critical` using transitive yield evidence from `report.no_yield_spans`.

