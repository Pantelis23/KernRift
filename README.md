# KernRift

Kernel-first systems language and compiler for OS architecture work: kernels, drivers, schedulers, memory, and IPC.

## Project Goal

KernRift keeps C-level control while making core kernel invariants compile-time enforceable:

- Interrupt and preemption context correctness
- Lock ordering and deadlock prevention metadata
- MMIO correctness (volatile semantics and ordering)
- Allocation and stack bounds in critical paths
- Capability-gated privileged operations

## Why This Exists

Generic systems languages and compilers do not model kernel reality directly. KernRift bakes OS-native semantics into the type system and compiler passes so invalid kernel behavior fails at compile time instead of in runtime boot/debug loops.

## Adaptive OS Alignment

This workspace includes `Adaptive_OS`, so KernRift is scoped to directly address issues observed there first.

Key baseline constraints observed in `Adaptive_OS`:

- No preemption in key paths (`STATUS.md:456`)
- Ring 3/user isolation still missing (`STATUS.md:419`)
- Polling-based interrupt transfers in USB path (`STATUS.md:889`)
- Frame allocator linear-scan limitations (`STATUS.md:177`)
- Existing policy intent already expressed as contracts (`README.md:241`, `README.md:242`)

Details are documented in `docs/ADAPTIVE_OS_CONTEXT.md`.

## MVP Path

- `KR0`: Freestanding static library output, pointers/slices, `unsafe`, typed `mmio<T>`
- `KR1`: Driver subset with effects (`@irq`, `@alloc`, `@block`, `@preempt`)
- `KR2`: Kernel module demo with spinlocks, scheduler hooks, per-cpu data
- `KR3`: Microkernel slice or Linux-side experimental subsystem

Execution criteria are in `docs/KR0_KR3_PLAN.md`.

## Initial Layout

- `docs/ARCHITECTURE.md` language and compiler architecture
- `docs/KRIR_SPEC.md` KRIR data model, ops, and mandatory verification passes
- `docs/ADAPTIVE_OS_CONTEXT.md` Adaptive OS problem mapping
- `docs/KR0_KR3_PLAN.md` staged delivery plan

## KR0 Status

Facts-only KR0 compiler pipeline is implemented as a Rust workspace:

- `crates/parser` minimal `.kr` parser
- `crates/hir` lowering + defaults + strict extern resolution
- `crates/krir` canonical KRIR structures
- `crates/passes` `ctx/effect/cap` checks
- `crates/emit` canonical JSON output
- `crates/kernriftc` CLI

CLI:

- `kernriftc check <file.kr>`
- `kernriftc check --policy <policy.toml> <file.kr>`
- `kernriftc check --contracts-out <contracts.json> <file.kr>` (canonical/minified contracts file for hashing/signing)
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>`
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> <file.kr>`
- `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> --hash-out <contracts.sha256> --sign-ed25519 <secret.hex> --sig-out <contracts.sig> <file.kr>`
- `kernriftc --emit krir <file.kr>`
- `kernriftc --emit lockgraph <file.kr>`
- `kernriftc --emit caps <file.kr>`
- `kernriftc --emit contracts <file.kr>`
- `kernriftc --report max_lock_depth,no_yield_spans <file.kr>`
- `kernriftc policy --policy <policy.toml> --contracts <contracts.json>`
- `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256>`
- `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> --sig <contracts.sig> --pubkey <pubkey.hex>`

Contracts ABI:

- Schema file: `docs/schemas/kernrift_contracts_v1.schema.json`

## Quality Gate

Run the local gate before merging:

- Linux/macOS: `./scripts/local_gate.sh`
- Windows PowerShell: `.\scripts\local_gate.ps1`
  - Script uses `cargo` from `PATH` or falls back to `%USERPROFILE%\.cargo\bin\cargo.exe`.

Gate contents:

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo test -p kernriftc --tests`
- `cargo test -p kernriftc --test kr0_contract`
- `cargo test -p kernriftc --test cli_contract`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo run -q -p kernriftc -- --emit lockgraph tests/must_pass/callee_acquires_lock.kr`

## Dev

Enable repo hooks:

```bash
git config core.hooksPath tools/hooks
```
