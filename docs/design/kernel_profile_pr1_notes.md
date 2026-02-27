# Kernel Profile PR1 Notes

## Current Pipeline (Source to Artifacts)

Current compiler/verifier flow:

1. Parse source `.kr` into module AST (`compile_file`).
2. Build HIR + semantic checks (`check_module` / `check_file`).
3. Analyze module (`analyze`) for interprocedural lock/yield/cap facts.
4. Emit deterministic artifacts:
   - `--emit krir` (KRIR JSON)
   - `--emit contracts` / `check --contracts-out` (contracts bundle JSON)
   - `verify --report` (verify report JSON)
5. Validate contracts and verify-report JSON against embedded schemas.

## Existing Artifacts

- KRIR JSON (`--emit krir`)
- Contracts JSON (`kernrift_contracts_v1`, canonical/minified via `check --contracts-out`)
- Verify report JSON (`kernrift_verify_report_v1`, `verify --report`)
- Optional hash/signature files (`--hash-out`, `--sig-out`)

## Kernel Constraints Already Modeled

Already modeled and enforced:

- lock acquisition ordering edge checks (policy forbid edge)
- max lock depth (`report.max_lock_depth` + policy limit)
- no-yield span bounds/unbounded state (`report.no_yield_spans` + policy limits)
- yield-under-lock detection (analysis + `check` errors)
- IRQ/effect checks from existing language facts in must-fail fixtures

## Missing for “Full Potential”

Not yet first-class in profile semantics:

- explicit kernel profile context model (`@irq`, `@critical`, `@no_alloc`) wired into policy diagnostics
- alloc/blocking observability surfaced as stable contract report fields
- explicit critical-section semantics and enforcement
- deterministic kernel profile ABI versioning in contracts v2

## Preconditions Command Log

Executed commands and outputs before PR1 coding:

```bash
$ cargo test --workspace
test result: ok. (all workspace tests passed)
```

```bash
$ cargo test -p kernriftc --test golden
test result: ok. 1 passed; 0 failed
```

```bash
$ ./tools/acceptance/krir_v0_1.sh
[1/5] cargo test --workspace
[2/5] cargo test -p kernriftc --test golden
[3/5] smoke: check emits contracts/hash
[4/5] smoke: verify pass writes report
[5/5] smoke: verify deny on hash mismatch (exit 1)
KRIR v0.1 acceptance: PASS
```

```bash
$ cargo run -q -p kernriftc -- --selftest
selftest: PASS
```

