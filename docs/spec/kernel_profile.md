# Kernel Profile (KR0.x)

This profile defines a stricter `kernriftc check` mode for kernel/OS builds:

```bash
kernriftc check --profile kernel <file.kr>
```

When kernel profile is enabled, contracts emission uses `kernrift_contracts_v2`
internally (or explicitly via `--contracts-schema v2`).

The goal is to enforce kernel-facing invariants with existing analyzers while keeping default profile behavior unchanged.

## Kernel Subset Rules

Current `kernel` profile defaults (materialized from canonical policy rule definitions):

- no unbounded no-yield spans (`forbid_unbounded_no_yield = true`)
- bounded no-yield spans (`max_no_yield_span = 64`)
- shallow lock nesting (`max_lock_depth = 1`)
- forbidden lock ordering edge (`ConsoleLock -> SchedLock`)
- forbid `alloc` effects in IRQ-reachable functions
- forbid `block` effects in IRQ-reachable functions
- forbid `yield`/`alloc`/`block` effects inside `critical { ... }` regions
- optional capability deny in IRQ context via policy:
  - `[kernel] forbid_caps_in_irq = ["CapA", ...]`
  - `[kernel] allow_caps_in_irq = ["CapA", ...]`
  - precedence is explicit: allowlist > forbidlist > default allow

Planned kernel subset rules (next phases):

- no heap allocation by default
- no blocking operations in IRQ context
- no yield inside critical sections
- explicit IRQ-safe lock policies

## Core Terms

- `IRQ context`: code running in interrupt context.
- `yield point`: operation that may hand control back to scheduler (`yieldpoint()` or transitive yield through call graph).
- `blocking`: operation that may sleep or block scheduler progress.
- `allocation`: operation that requests dynamic memory.
- `critical section`: region where preemption/yield must not occur.
  - KR0.x marker: `critical { ... }` statement form.

## Compile-Time vs Runtime

Compile-time enforcement in KR0.x:

- lock depth limits
- lock edge ordering deny-lists
- bounded vs unbounded no-yield spans
- transitive effect checks from call graph (`alloc`, `block`, `yield`) in kernel policy rules
  - direct effects come from both operation sites (`*point()` builtins) and function facts (`@eff(...)`)
  - extern stubs (`extern fn`) contribute effects via declared `@eff(...)` and are included in transitive closure
  - contracts v2 expose deterministic `eff_provenance` per symbol/effect (`direct`, `via_callee[]`, `via_extern[]`)
- context reachability checks from call graph via `facts.symbols[*].ctx_reachable`
- critical region analysis:
  - max region nesting depth
  - deterministic per-function violation facts with normalized provenance (`direct`, `via_callee[]`, `via_extern[]`)
- deterministic diagnostics and artifacts

Contracts v2 semantic split:
- `facts.symbols[*]`: symbol semantics (`ctx_ok`, `ctx_reachable`, `eff_used`, `eff_transitive`, `eff_provenance`, caps, attrs)
- `report.*`: aggregate/violation summaries (`max_lock_depth`, `no_yield_spans`, effect site counts, critical findings)

Capability semantics in contracts v2:
- `caps_req`: direct declared capability requirements
- `caps_transitive`: call-graph propagated capability requirements
- `caps_provenance`: deterministic provenance per capability:
  - `direct`
  - `via_callee[]`
  - `via_extern[]`

Policy family structure (kernel policy evaluation):
- context rules
- lock rules
- effect rules
- region rules
- capability rules
- each family emits structured violations rendered via a family-specific deterministic formatter
  - external line shape remains: `policy: <CODE>: <message>`
- policy rule metadata is cataloged centrally (`code`, `family`, `sort_rank`, `requires_v2`)
  - definitions also carry `default_enabled_in_profile_kernel`, `diagnostic_template_id`, canonical profile `materialization_actions`, canonical `enablement_probes`, rule `trigger_kind` / `artifact_dependencies`, and canonical `condition_descriptors`
- `--profile kernel` defaults are materialized from canonical rule definitions (not a separate static default list)
  - violation ordering is deterministic by `sort_rank`, then `code`, then message

Policy consumes contracts artifacts directly (v2 facts/report) and does not reconstruct
hidden call-graph semantics when those facts are already present in the artifact.

Runtime responsibilities (outside KR0.x compiler checks):

- lock implementation correctness
- scheduler and interrupt controller behavior
- memory allocator and blocking primitive implementation semantics

## Kernel Invariants (Test Questions)

- Can an `@irq` function allocate? (target answer: no)
- Can an `@irq` function take locks? (policy-controlled; default no or allowlist only)
- Can a `critical { ... }` region contain yield? (target answer: no)
- Are lock acquisitions order-consistent across call graph? (target answer: yes)
- Are no-yield spans bounded in kernel profile? (target answer: yes)

Each invariant must map to deterministic diagnostics + regression tests.
