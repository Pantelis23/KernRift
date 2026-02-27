# Kernel Profile (KR0.x)

This profile defines a stricter `kernriftc check` mode for kernel/OS builds:

```bash
kernriftc check --profile kernel <file.kr>
```

When kernel profile is enabled, contracts emission uses `kernrift_contracts_v2`
internally (or explicitly via `--contracts-schema v2`).

The goal is to enforce kernel-facing invariants with existing analyzers while keeping default profile behavior unchanged.

## Kernel Subset Rules

Current `kernel` profile defaults (from `policies/kernel.toml`):

- no unbounded no-yield spans (`forbid_unbounded_no_yield = true`)
- bounded no-yield spans (`max_no_yield_span = 64`)
- shallow lock nesting (`max_lock_depth = 1`)
- forbidden lock ordering edge (`ConsoleLock -> SchedLock`)
- forbid `alloc` effects in IRQ-reachable functions
- forbid `block` effects in IRQ-reachable functions
- forbid `yield`/`alloc`/`block` effects inside `critical { ... }` regions

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
- context reachability checks from call graph via `facts.symbols[*].ctx_reachable`
- critical region analysis:
  - max region nesting depth
  - deterministic per-function violation facts (direct and transitive-through-call)
- deterministic diagnostics and artifacts

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
