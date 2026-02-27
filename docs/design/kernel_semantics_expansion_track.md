# Kernel Semantics Expansion Track

## Architecture Note

KernRift should treat kernel reasoning as one semantic layer, not scattered checks.
The layer is organized around four first-class dimensions:

1. Context semantics:
   - `ctx_ok` (declared)
   - `ctx_reachable` (call-graph closure)
2. Effect semantics:
   - `eff_used` (direct facts/sites)
   - `eff_transitive` (call-graph closure)
   - `eff_provenance` (why an effect is present: direct, via callee, via extern contract)
3. Region semantics:
   - explicit region boundaries (`critical { ... }`)
   - deterministic region findings (depth + violations with provenance)
4. Capability semantics:
   - `caps_req` (direct requirements)
   - `caps_transitive` (call-graph closure)
   - `caps_provenance` (direct / via-callee / via-extern)

Why this is better than incremental rules:
- policies consume stable contract facts instead of hidden compiler state
- each deny can explain *why* (provenance) and *where* (symbol/region)
- artifacts are deterministic and composable for CI, verification, and later codegen

Kernel invariants enforced by this layer must be encoded in contracts, then evaluated by policy.

## Staged PR Plan

### PR-A: Effect Semantics Core (current)
Proves:
- deterministic effect provenance is emitted in contracts v2
- provenance includes direct, via-callee, and via-extern paths
- extern `@eff(...)` contracts are propagated transitively
- report/facts split is normalized:
  - symbol semantics in `facts.symbols[*]`
  - aggregate/violation semantics in `report.*`

Files:
- `crates/emit/src/lib.rs`
- `docs/schemas/kernrift_contracts_v2.schema.json`
- `crates/kernriftc/tests/cli_contract.rs`
- `crates/kernriftc/tests/kernel_profile.rs`
- `tests/kernel_profile/*extern_stub*.kr`
- `tests/must_pass/transitive_alloc_*.kr`

Leaves out:
- region-local provenance unification (covered by PR-B)
- capability provenance (covered by PR-D)

### PR-B: Region Semantics Unification
Proves:
- region findings share the same provenance model as effects
- region violations are keyed by region type (`critical`, future `preempt_off`, `irq_off`)

Files:
- `crates/emit/src/lib.rs`
- `docs/schemas/kernrift_contracts_v2.schema.json` (or v3 if needed)
- `crates/kernriftc/tests/cli_contract.rs`
- `tests/kernel_profile/critical_region_*.kr`

### PR-C: Kernel API Contract Model
Proves:
- extern contracts can be grouped into deterministic API classes (allocator/scheduler/mmio/dma/lock)
- policy can consume API class facts without hardcoded OS names

Files:
- `crates/kernriftc/src/main.rs`
- `policies/kernel.toml`
- `docs/spec/kernel_profile.md`
- `examples/kernel/*`

### PR-D: Policy Engine Normalization
Proves:
- policy rules are grouped by semantic category (context/effect/region/lock/capability)
- each rule has a stable code, deterministic message shape, and deterministic order
- policy reads contracts semantics directly instead of reconstructing hidden state
  (for v2/kernel mode):
  - IRQ checks consume `facts.symbols[*].ctx_reachable`
  - effect checks consume `facts.symbols[*].eff_transitive` + `eff_provenance`
  - capability checks consume `facts.symbols[*].caps_transitive` + `caps_provenance`
  - region checks consume `report.critical.violations`

Files:
- `crates/kernriftc/src/main.rs`
- `crates/kernriftc/tests/*`
- `docs/spec/kernel_profile.md`

### PR-E: Kernel Author Ergonomics
Proves:
- semantic explanations include deterministic `why`/`via` guidance
- authoring improvements do not reduce analyzability

Files:
- `crates/kernriftc/src/main.rs`
- `examples/kernel/*`
- `docs/spec/krir-v0.1.md`
