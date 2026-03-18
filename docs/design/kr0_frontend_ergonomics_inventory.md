# KR0 Frontend Ergonomics Inventory

## Scope

This inventory covers only KR0 frontend and user-facing syntax ergonomics as implemented on current `main`.
It does not propose semantic, policy, JSON, artifact, or metadata changes.

Audit inputs:

- `crates/parser/src/lib.rs`
- `crates/hir/src/lib.rs`
- `docs/spec/krir-v0.1.md`
- `docs/KRIR_SPEC.md`
- `tests/must_pass/*`
- `tests/must_fail/*`
- `tests/living_compiler/*`

## Quick Read

Highest-friction areas today:

- duplicated spelling for the same facts (`@ctx(...)` vs `@irq` / `@noirq`, `@eff(...)` vs `@alloc` / `@block` / `@preempt_off`)
- split grammar documentation (core EBNF in one place, accepted forms hidden in notes and fixtures)
- extern declaration ceremony (`extern @ctx(...) @eff(...) @caps() fn ...;`)
- MMIO declaration and operand ceremony, which is intentionally explicit but still hard to memorize
- deterministic diagnostics that still lean on byte offsets and rarely suggest the canonical replacement

Current canonicalization direction for KR0 frontend syntax:

- prefer `@ctx(...)`, `@eff(...)`, `@caps(...)`, `@module_caps(...)`, and `critical { ... }`
- keep accepted aliases as compatibility spellings only
- make diagnostics explicitly steer users back to the canonical spelling

## Syntax Inventory

| Family | Current surface | Classification | Ergonomics read |
|---|---|---|---|
| Function item | `fn name() { ... }` | keep | Minimal and easy to remember. |
| Extern item | `extern @ctx(...) @eff(...) @caps() fn name();` | keep | Explicit and correct for dangerous boundaries, but high ceremony and easy to under-specify. |
| Module caps | `@module_caps(...)` | keep | Useful, but unusual because it looks like an attribute while behaving like a module directive. |
| Canonical context facts | `@ctx(...)` | keep | Best current canonical form; scales better than one-off aliases. |
| Canonical effect facts | `@eff(...)` | keep | Best current canonical form; scales better than one-off aliases. |
| Canonical capability facts | `@caps(...)` | keep | Explicit and appropriate for privileged boundaries; empty `@caps()` on externs is easy to forget. |
| Function attrs | `@noyield`, `@critical`, `@leaf`, `@hotpath`, `@lock_budget(N)` | keep | Mostly low-friction; `@critical` attribute vs `critical {}` block deserves clearer user guidance. |
| Context shorthands | `@irq`, `@noirq` | deprecate | They duplicate `@ctx(...)`, are under-documented, and increase memorization burden for little gain. |
| Effect shorthands | `@alloc`, `@block`, `@preempt_off` | deprecate | Same problem as context shorthands; canonical `@eff(...)` is clearer and more uniform. |
| Stable alias | `@thread_entry` | alias | Good ergonomic alias for `@ctx(thread)`; low-risk and already stable. |
| Experimental alias | `@irq_handler` | alias | Useful alias for `@ctx(irq)`, but should stay clearly gated because irq semantics are safety-sensitive. |
| Experimental alias | `@may_block` | alias | Useful alias for `@eff(block)`, but should stay secondary to canonical `@eff(...)`. |
| Deprecated alias | `@irq_legacy` | remove-later | Lifecycle already says deprecated; should stay unavailable and eventually disappear from user mental models. |
| Lock statements | `acquire(LockClass)`, `release(LockClass)` | keep | Explicit and short enough; no major ergonomics issue. |
| Control statements | `yieldpoint()`, `allocpoint()`, `blockpoint()` | keep | Clear synthetic effect markers; docs should better connect them to `@eff(...)` facts. |
| Critical region block | `critical { ... }` | keep | Good explicit syntax; currently easy to confuse with `@critical` whole-function behavior. |
| Structured MMIO decl | `mmio NAME = INT_LITERAL;` | keep | Narrow and explicit; acceptable ceremony for hardware-facing declarations. |
| Structured MMIO register decl | `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;` | keep | Verbose, but the verbosity pays for clarity; better docs/examples matter more than shorter syntax here. |
| Structured MMIO ops | `mmio_read<T>(addr)`, `mmio_write<T>(addr, value)` | keep | Reasonable explicitness; generic-call spelling is easy enough once learned. |
| Raw MMIO ops | `raw_mmio_read<T>(addr)`, `raw_mmio_write<T>(addr, value)` | keep | Intentionally noisy; should remain visually distinct. |
| Rejected legacy MMIO ops | zero-arg `mmio_read()` / `mmio_write()` / `raw_mmio_*()` | remove-later | Already rejected. Keep the rejection path and eventually stop documenting them except in migration notes. |

## Prioritized Ergonomics Backlog

### P0: docs and diagnostics only

1. Publish one canonical frontend syntax reference.
   - Move accepted-but-currently-scattered forms into one user-facing section.
   - Include canonical forms, accepted aliases, deprecated forms, and examples.

2. Upgrade parser and HIR diagnostics from byte offsets to line/column plus source snippet.
   - Keep deterministic wording.
   - Add a single stable formatting convention before widening syntax.

3. Add canonical replacement hints for alias and lifecycle errors.
   - Example: `@irq_handler` stable-surface rejection should also point to `@ctx(irq)`.
   - Example: deprecated `@irq_legacy` should always point to `@ctx(irq)` and, when relevant, `@irq_handler`.

4. Improve extern-facts diagnostics.
   - Current errors are correct but repetitive.
   - The frontend should suggest the minimal valid extern template: `extern @ctx(...) @eff(...) @caps() fn name();`.

5. Clarify whole-function vs block-scoped critical syntax.
   - Document `@critical` vs `critical {}` side by side.
   - Add examples that show when each is appropriate.

### P1: low-risk syntax cleanup

6. Start deprecating duplicated unary fact attributes.
   - `@irq`, `@noirq`, `@alloc`, `@block`, `@preempt_off`
   - Keep parsing them initially, but steer users to `@ctx(...)` / `@eff(...)`.

7. Consider trailing-comma support in list-like attribute arguments.
   - Candidate targets: `@ctx(...)`, `@eff(...)`, `@caps(...)`, `@module_caps(...)`.
   - This is low semantic risk and improves editability, especially for multi-value lists.

8. Make the stable alias story explicit.
   - `@thread_entry` is already a good stable alias.
   - The repo should state clearly which aliases are permanent ergonomic sugar vs temporary lifecycle-gated forms.

### P2: syntax changes that should wait for a dedicated lane

9. Revisit MMIO declaration ergonomics only after diagnostics are stronger.
   - Current syntax is verbose, but it is also safety-relevant.
   - Any aliasing here should keep raw/unsafe operations visibly loud.

10. Revisit extern declaration boilerplate only after a coherent shorthand design exists.
   - Reducing ceremony is tempting, but this boundary carries the highest semantic risk in KR0.

## Candidate Parser and Diagnostic Improvements

1. Replace `at byte N` with `line:column` and a one-line source excerpt.
2. Add `did you mean ...` suggestions for common near-misses and canonical replacements.
3. Split `expected item boundary` into clearer cases.
   - Today one message covers `fn`, `mmio`, `mmio_reg`, and `@module_caps(...)`.
4. Add dedicated guidance for missing extern facts.
   - Missing `@ctx`, missing `@eff`, missing `@caps()` should point to one valid example.
5. Make MMIO operand errors more shape-aware.
   - Distinguish call expressions, parenthesized forms, and multi-operator arithmetic instead of one generic unsupported-operand message.
6. Add clearer diagnostics for `@critical` vs `critical {}` misuse.
7. If unary fact attributes remain accepted for compatibility, emit deprecation-oriented guidance toward `@ctx(...)` / `@eff(...)`.

## Recommended First Narrow PR

A narrow first PR should stay docs/tests only and avoid syntax changes:

- add this inventory document
- link it from `docs/spec/krir-v0.1.md`
- optionally add one tiny doc-presence test only if the repo wants this inventory kept as a locked contributor reference

That keeps the next syntax-facing PR honest:

- reviewable in isolation
- no semantic drift risk
- no policy / JSON / artifact noise
- creates a stable baseline for future alias, deprecation, and diagnostic work
