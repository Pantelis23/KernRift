# Repo Organization, Docs & CLI Redesign

**Date:** 2026-03-21
**Status:** Approved
**Scope:** Onboarding layer, per-crate docs, installation instructions, CLI default command

---

## Goal

Make KernRift approachable for both external kernel/systems developers and internal Adaptive_OS collaborators. Add a complete onboarding path covering Linux, macOS, and Windows. Clean up the primary CLI interface.

---

## Deliverables

### 1. Rewritten `README.md`

Replace the current ~200-line CLI-heavy README with a lean landing page:

- **Header**: 1-paragraph pitch — what KernRift is, why it exists
- **Feature bullets**: 5–6 one-liners (context safety, lock ordering, MMIO correctness, capability gating, signed artifacts)
- **Install table**: Three-platform quick commands — `cargo install` and prebuilt binary download paths
- **Quickstart**: 5 lines — install, write a `.kr` file, run `kernriftc hello.kr`, see `.krbo` output
- **Links section**: Getting Started → Language Reference → Architecture → Contributing → Changelog

CLI reference is removed from README entirely; it lives in `docs/getting-started.md`.

---

### 2. `docs/getting-started.md`

Full guided onboarding document:

- **Prerequisites**: Rust 1.93.1 via rustup (auto-pinned by `rust-toolchain.toml`), Cargo
- **Install from source** (all platforms): `cargo install --path crates/kernriftc`
- **Install prebuilt binary**:
  - Linux/macOS: `gh release download` or `curl`, SHA256 verification, `chmod +x`
  - Windows: PowerShell `Invoke-WebRequest`, `Get-FileHash` SHA256 check, add to PATH
- **Your first program**: Annotated walkthrough of `hello.kr` — each annotation explained in plain English (`@ctx`, `@module_caps`, `raw_write`)
- **Running the compiler**: `kernriftc hello.kr` → produces `hello.krbo`; what success looks like; what an error looks like (context violation example)
- **Common commands**: one-sentence each for `kernriftc check`, `--emit krir`, `--emit lockgraph`, `verify`
- **Next steps**: links to `docs/LANGUAGE.md`, `docs/ARCHITECTURE.md`, `examples/`

---

### 3. `CONTRIBUTING.md`

- **Prerequisites**: Rust 1.93.1 (auto-pinned), Cargo
- **Build**: `cargo build --release -p kernriftc`
- **Test**: `cargo test --workspace`, `./scripts/local_gate.sh` (fmt + test + clippy, warnings-as-errors)
- **Crate map**: table with 6 rows — crate name, one-line role, key types
- **Adding tests**: when to use `tests/must_pass/` vs `tests/must_fail/` vs `tests/golden/`; file naming convention
- **Code style**: rustfmt + clippy enforced by gate; no `#[allow(warnings)]` in new code
- **PR checklist**: gate passes, new `.kr` test for new behaviour, `CHANGELOG.md` updated

---

### 4. Per-Crate `README.md` Files

One `README.md` per crate under `crates/*/README.md`, ~30 lines each, following this template:

```
# <crate-name>
One-line role in the pipeline.

## Inputs / Outputs
## Key Types
## Pipeline Position
  upstream → [this crate] → downstream
```

Crates: `parser`, `hir`, `krir`, `passes`, `emit`, `kernriftc`.

---

### 5. CLI Change: Default Compile Command

**Current behaviour:**
```
kernriftc --emit krbo <file.kr>   # produces .krbo
kernriftc check <file.kr>         # analysis only
```

**New behaviour:**
```
kernriftc <file.kr>               # compiles → <file>.krbo  (NEW default)
kernriftc check <file.kr>         # analysis only (unchanged)
kernriftc verify ...              # unchanged
kernriftc policy ...              # unchanged
kernriftc inspect-artifact ...    # unchanged
```

Implementation: add a positional argument branch in the CLI dispatcher (`crates/kernriftc/src/main.rs`) that, when a bare `.kr` path is given with no subcommand, invokes the existing KRBO emit path. No changes to emit logic.

---

## Out of Scope

- Package manager distribution (apt, winget, Homebrew) — deferred; requires stable versioning and per-ecosystem registry work
- Reorganizing `docs/` subdirectory structure
- Modifying existing spec docs (`KRIR_SPEC.md`, `ARCHITECTURE.md`, `LANGUAGE.md`)
