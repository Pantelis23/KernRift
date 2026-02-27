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

- We observe `alloc`/`block` at function-fact level via `eff_used`.
- We now also have operation-level builtins in KRIR:
  - `allocpoint()` -> `KrirOp::AllocPoint`
  - `blockpoint()` -> `KrirOp::BlockPoint`

Current mapping:

- contracts v2 `report.effects.alloc_sites_count` is counted from `KrirOp::AllocPoint`.
- contracts v2 `report.effects.block_sites_count` is counted from `KrirOp::BlockPoint`.
- contracts v2 `report.effects.yield_sites_count` continues to count `KrirOp::YieldPoint`.
- contracts v2 `facts.symbols[*].eff_transitive` is derived by SCC-aware call-graph closure:
  - `eff_transitive(fn) = eff_used(fn) ∪ union(eff_transitive(callee))`
  - SCCs are collapsed first, then effects are propagated over the component DAG.

## Critical Context Marker

Critical marker is first-class via `@critical`:

- contracts v2 `report.contexts.critical_functions` is derived from `facts.symbols[*].attrs.critical`.
- kernel policy enforces `forbid_yield_in_critical` using transitive yield evidence from
  `facts.symbols[*].eff_transitive`.
