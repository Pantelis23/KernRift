# The Living Compiler

> *Can a programming language remain stable enough for real engineering
> while still evolving its syntax, features, and ergonomics in response
> to actual usage?*

The Living Compiler is a governed, self-evolving compiler system: it
observes how the language is used, identifies friction points, proposes
improvements, and can adopt them under strict control — without
destroying compatibility, determinism, or long-term maintainability.

This document describes the vision and the current implementation. The
blueprint is ambitious; we build toward it in stages.

---

## The two-layer model

The core idea that makes the Living Compiler viable:

**Stable semantic core.** Type rules, IR meaning, safety model, ABI,
concurrency behavior. This layer changes slowly and carefully. Old code
compiled against a pinned core version continues to compile forever.

**Adaptive surface layer.** Syntax sugar, annotations, diagnostics,
common patterns, domain-specific constructs. This layer evolves in
response to observed usage. Multiple surface versions can coexist as
long as they lower into the same canonical IR.

This separation is what lets a language mature without fragmenting.
Beginners get ergonomic shortcuts. Long-lived kernels get a frozen
semantic contract. Migrations are mechanical because the lowered form
is stable.

---

## The pipeline

```
       Source code
           |
           v
    +-------------+
    |  Telemetry  |   Observe usage: call graph, pattern frequency,
    +-------------+   unsafe density, lock depth, extern ratio, etc.
           |
           v
    +-------------+
    |  Proposals  |   Generate candidate improvements. Each proposal
    +-------------+   names a pattern and a replacement form.
           |
           v
    +-------------+
    |   Fitness   |   Score each proposal: readability, safety,
    +-------------+   adoption cost, ambiguity, performance impact.
           |
           v
    +-------------+
    | Governance  |   Approve / reject / rollback / stabilize. Track
    +-------------+   the promotion lifecycle: experimental → stable
           |         → deprecated.
           v
    +-------------+
    |  Migration  |   Rewrite old code into the new form. Mechanical
    +-------------+   when the lowering is identity-preserving.
```

The five components are designed to be independent. You can run just
the telemetry pass and get a report. You can run telemetry + proposals
+ fitness and see suggestions. Governance and migration are the
heavier-weight stages that require human judgment or batch rewrites.

---

## Success criteria

From the original blueprint:

- Old projects continue to compile under pinned versions.
- New syntax or features provide measurable benefit.
- Changes are reversible and versioned.
- Migration can be automated.
- Semantic stability is preserved.
- Compile-time and runtime regressions stay controlled.
- The language becomes more useful over time without becoming fragmented.

---

## Current implementation

`krc lc <file.kr>` runs the telemetry and proposal passes and prints a
two-layer report.

```
=== KernRift Living Compiler Report ===
    stable semantic core + adaptive surface layer

Telemetry
  Functions:    13
  Calls:        21
  Unsafe ops:   22
  Total ops:    205
  Patterns:     2

Fitness: 73/100

--- Semantic Core (1) ---

  [1] unchecked_call  fitness: 43
      title: Unchecked call results
      signal: 95 occurrence(s)
      suggestion: Check return values of calls, especially for file I/O and allocation.

--- Adaptive Surface (1) ---

  [1] legacy_ptr_ops  fitness: 60  (auto-fix available)
      title: Legacy unsafe{} pointer syntax
      signal: 22 occurrence(s)
      suggestion: Migrate `unsafe { *(addr as T) -> dest }` to `dest = loadN(addr)` and `unsafe { *(addr as T) = val }` to `storeN(addr, val)`. Same codegen, much cleaner.
```

Patterns are classified as **semantic core** (correctness or
structural issues) or **adaptive surface** (ergonomic migrations).
Patterns tagged `(auto-fix available)` are candidates for the future
migration engine — the rewrite is identity-preserving at the IR level.

### Patterns currently detected

**Semantic core:**

| Pattern | What it catches |
|---|---|
| `unchecked_call` | Calls whose results are discarded (especially risky for I/O and allocation). |
| `large_unsafe_ratio` | Too many raw unsafe operations vs total operations. |
| `high_call_density` | Very complex call graph &mdash; hard to audit. |

**Adaptive surface:**

| Pattern | Migration | Auto-fix |
|---|---|---|
| `legacy_ptr_ops` | `unsafe { *(addr as T) -> dest }` → `dest = loadN(addr)` | Yes (planned) |
| `many_trivial_fns` | Inline 1-2 statement helper functions at call sites. | No |

### CLI

```sh
krc lc file.kr                     # full report
krc lc --min-fitness=30 file.kr    # filter to patterns with fitness >= 30
krc lc --ci file.kr                # fail exit code if any pattern fires
krc lc --ci --min-fitness=50 file.kr  # fail only on patterns >= 50
```

### Fitness scoring

The file-level fitness score starts at 100 and deducts for each
detected pattern:

- **Core-layer** pattern: deduct up to `fitness / 4` (bigger penalty).
- **Surface-layer** pattern: deduct up to `fitness / 8` (smaller penalty).

This reflects the blueprint's priority: correctness concerns weigh more
than stylistic ones.

---

## Roadmap

What's built and what's next, aligned with the blueprint:

| Stage | Status |
|---|---|
| Telemetry collection | Implemented |
| Pattern detection | Implemented (5 patterns, 2-layer classification) |
| Fitness scoring | Implemented (layer-weighted) |
| Two-layer report | Implemented |
| CI gating (`--ci`, `--min-fitness`) | Implemented |
| Auto-fix flag on patterns | Implemented (advisory) |
| **Migration engine** (actual source rewriting) | **Not yet** — needs a safe source-to-source rewriter. |
| **Proposal engine** (generate *new* language forms from usage) | **Not yet** — requires a DSL for describing candidate syntax. |
| **Governance layer** (approve / reject / rollback / stabilize) | **Not yet** — requires a proposal store and versioning. |
| **Versioned language profiles** (`#lang stable` / `#lang experimental`) | **Not yet** — requires pinned compiler semantics per version. |

The current implementation is a solid foundation for the vision: the
two-layer split is real, the telemetry is real, and the surface-layer
patterns map to real migrations. Each unimplemented stage is a future
chunk of focused work — none of them are blocked by the others.

---

## Why it matters

Traditional languages become outdated because real usage patterns
change faster than the language can. Engineers compensate with macros,
boilerplate, conventions, wrappers, and external tooling. The Living
Compiler internalizes that pressure and turns it into structured,
measurable language evolution.

The goal is not a chaotic language that changes unpredictably. The
goal is a compiler that can propose, evaluate, version, and adopt
language changes under strict control. The language becomes an
evolving engineering system: new syntax forms, domain-specific
constructs, and usability improvements can emerge when they are
justified by measurable benefit, while the underlying semantics remain
stable and reproducible.

Language design as an ongoing optimization process, rather than a
one-time decision.
