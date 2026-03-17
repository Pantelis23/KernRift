# Kernel PR2 Inventory

## Existing Syntax We Reuse

### Context facts (`@ctx(...)`)

Parsed in `crates/hir/src/lib.rs`:

- `boot`
- `thread`
- `irq`
- `nmi`

`@ctx(...)` is lowered through HIR into KRIR `ctx_ok` and emitted in contracts `facts.symbols[*].ctx_ok`.
contracts v2 additionally emits `facts.symbols[*].ctx_reachable`, computed by call-graph
reachability closure from declared context roots.
report-level context lists were removed; policy consumes `facts.symbols[*].ctx_reachable` directly.

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
For `extern fn`, `@eff(...)` is mandatory and seeds `eff_used` directly from declared facts.

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
- contracts v2 `report.effects.raw_mmio_sites_count` counts `KrirOp::RawMmioRead` + `KrirOp::RawMmioWrite`.
- report remains aggregate-only; symbol-level semantics stay in `facts.symbols[*]`.
- contracts v2 `facts.symbols[*].raw_mmio_used` and `raw_mmio_sites_count` distinguish raw MMIO sites
  from ordinary structured `mmio_*` usage for governance/reporting consumers.
- kernel policy now consumes those raw-MMIO fields directly for:
  - deny-all raw MMIO
  - per-symbol raw-MMIO allowlists
  - aggregate raw-MMIO site caps
  - irq-context raw-MMIO deny by intersecting `facts.symbols[*].ctx_reachable` with raw-MMIO symbol facts
  - irq-context raw-MMIO site caps by summing `facts.symbols[*].raw_mmio_sites_count` only over
    irq-reachable symbols
- contracts v2 `facts.symbols[*].eff_transitive` is derived by SCC-aware call-graph closure:
  - `eff_transitive(fn) = eff_used(fn) ∪ union(eff_transitive(callee))`
  - SCCs are collapsed first, then effects are propagated over the component DAG.
  - this includes extern stubs, so caller transitive effects include effects declared on external APIs.
- contracts v2 `facts.symbols[*].eff_provenance[]` records deterministic origin for each transitive
  effect:
  - `direct` (function-local fact/site),
  - `via_callee[]` (non-extern propagation),
  - `via_extern[]` (extern contract propagation).
- contracts v2 `facts.symbols[*].caps_transitive` and `caps_provenance[]` follow the same model:
  - direct source from `caps_req`
  - propagation over call graph (including extern contracts)
  - deterministic provenance with `direct` / `via_callee[]` / `via_extern[]`

## Critical Regions

Critical sections are represented by statement blocks:

- Parser form: `critical { ... }`
- HIR lowers each block to `KrirOp::CriticalEnter` / `KrirOp::CriticalExit`.
- contracts v2 emits `report.critical`:
  - `depth_max`
  - `violations[]` with `{function,effect,provenance}`
- kernel policy enforces `forbid_effects_in_critical` using those deterministic violation facts.
