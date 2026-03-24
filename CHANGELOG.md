# Changelog

All notable changes to `kernriftc` are documented in this file.

## v1.0.0 - 2026-03-24

### Added
- AArch64 (ARM64) backend: `aarch64-sysv` (Linux), `aarch64-macho` (macOS), `aarch64-win` (Windows).
- `KRBOFAT` fat binary container: 8-byte magic `KRBOFAT\0`, LZ4-compressed per-arch slices, fat-first detection (checked before single-arch `KRBO` magic).
- Default `kernriftc <file.kr>` output is now a fat binary (`.krbo`) containing x86_64 and ARM64 slices.
- `--arch x86_64|arm64|aarch64` flag: routes compilation to the specified target; output is still a fat binary. `aarch64` is an accepted alias for `arm64`.
- `--emit=krbofat` explicit fat binary emit mode (equivalent to default compile).
- `--emit=krboexe` for single-arch x86_64 KRBO (unchanged from prior behavior when requesting single-arch output explicitly).
- Dual-file output for `--emit=elfobj`, `--emit=asm`, `--emit=staticlib` without `--arch`.
- `kernrift` runner: fat-first detection reads 8-byte magic before the 4-byte single-arch check; extracts host-arch slice and executes it.
- ARM64 I-cache flush: `kernrift` flushes the instruction cache after writing ARM64 code to executable memory (required for AArch64 coherence on all Linux/macOS ARM64 hosts).
- New `krir` crate constants and APIs: `KRBO_ARCH_AARCH64` (`0x02`), `KRBO_FAT_MAGIC`, `KRBO_FAT_VERSION`, `KRBO_FAT_ARCH_X86_64`, `KRBO_FAT_ARCH_AARCH64`, `KRBO_FAT_COMPRESSION_NONE`, `KRBO_FAT_COMPRESSION_LZ4`, `emit_aarch64_executable_bytes()`, `emit_aarch64_elf_object_bytes()`, `emit_aarch64_macho_object_bytes()`, `emit_aarch64_coff_object_bytes()`, `emit_krbofat_bytes()`, `parse_krbofat_slice()`.
- New `krir` types: `TargetArch::AArch64`, `TargetAbi::Aapcs64`/`Aapcs64Win`, `BackendTargetId::Aarch64Sysv`/`Aarch64MachO`/`Aarch64Win`, `AArch64IntegerRegister` (X0–X15, X19–X30, Sp, Xzr; X16/X17/X18 excluded), `lower_executable_krir_to_aarch64_asm()`.
- New `kernriftc` CLI: `BackendArtifactKind::KrboFat` ("krbofat").
- New spec docs in `docs/spec/`: `aarch64-asm-linear-subset-v0.1.md`, `aarch64-object-linear-subset-v0.1.md`, `backend-target-model-aarch64-sysv-v0.1.md`, `backend-target-model-aarch64-macho-v0.1.md`, `backend-target-model-aarch64-win-v0.1.md`, `krbofat-container-v0.1.md`.

### Fixed
- `kernrift` runner: `map_uart_buffer` falls back to a kernel-chosen address when `mmap(MAP_FIXED)` at `0x10000000` is rejected (macOS CI ARM64 and Windows CI return `ENOMEM`/`null` for fixed mappings); programs with no MMIO (e.g. `examples/smoke_noop.kr`) are unaffected.
- `kernrift` runner (Unix): `map_executable` now maps `PROT_READ|PROT_WRITE` first, copies code, then `mprotect`s to `PROT_READ|PROT_EXEC`; avoids rejection of `PROT_WRITE|PROT_EXEC` on macOS CI (W^X enforcement).
- `kernrift` runner (Windows): `map_executable` calls `FlushInstructionCache` after writing JIT code; fixes SIGILL (exit 132) on Windows ARM64 where the I-cache and D-cache are incoherent.
- `krir` tests: 9 ELF-link tests now compile-gated with `#[cfg(all(unix, target_arch = "x86_64"))]`; `ld` on Windows emits PE (MZ magic), not ELF — those tests no longer run on Windows.

### Platform notes
- **macOS x86_64**: CI builds and ships the binary but execution on Intel Mac hardware has not been independently verified. Use with caution and report any issues.

### Tested
- `examples/smoke_noop.kr` compiled on Pi 400 (aarch64 Linux), fat binary pulled and run on x86_64 — exit 0.
- `examples/smoke_noop.kr` compiled on x86_64, fat binary run on Pi 400 — exit 0.

## v0.3.1 - 2026-03-23

### Added
- `kernrift` split into its own crate so `cargo install` tracks both binaries independently.
- `elfexe` emit target: `kernriftc --emit=elfexe` links an ELF ET_EXEC binary using `ld.lld`/`ld`.
- Dead function elimination pass: strips functions unreachable from `@export`/`@ctx(boot)`.
- Link-time lock graph merge: `kernriftc link` detects cross-module lock-order cycles.
- `kernriftc lc` alias: short form for `kernriftc living-compiler` (alias kept).
- Three new living-compiler patterns: `irq_raw_mmio`, `high_lock_depth`, `mmio_without_lock`.
- `lc --ci`: exit 1 if any suggestion fitness ≥ 50 (override with `--min-fitness N`).
- `lc --diff <file>`: show only new/worsened suggestions vs git HEAD.
- `lc --diff <before> <after>`: two-file local diff, no git dependency.
- `lc --fix --dry-run`: preview tail-call fixes as a unified diff.
- `lc --fix --write`: apply tail-call fixes in place, atomically.

### Improved
- **Syntax error messages** — all TokParser diagnostics now show human-readable token names
  instead of Rust debug format (e.g. `got '{'` instead of `got LBrace`).
  Specific improvements:
  - Missing return type after `->`: suggests valid types and `-> u64` example.
  - `if` without a condition: points at the `{` and suggests a boolean expression.
  - `let` keyword: directs to typed declaration syntax (`u64 x = ...`).
  - Undeclared variable assignment: names the variable and suggests declaration syntax.
  - Duplicate symbol: includes source location in the error.
  - Missing comma between call arguments: flags the unexpected token.
  - `mmio`/`mmio_reg` inside a function body: reports module-scope restriction.
  - `expect_kind` and all inner parser helpers use readable token names.
- `token_kind_to_str` is now exhaustive — every `TokenKind` variant maps to a display string.

## v0.2.10 - 2026-02-27

### Changed
- KRIR v0.1 acceptance script added: `tools/acceptance/krir_v0_1.sh`.
- Verify-report schema documentation tightened and strictness negative tests added for unknown keys/invalid enum values.
- KRIR spec updated with explicit verify-report ABI strictness table.

### Notes
- Product-only release.
- No infra/release workflow changes.
- `v0.2.9` remains frozen.

## v0.2.9 - 2026-02-27

### Changed
- KRIR v0.1: added schema-validated verify report ABI v1 (`docs/schemas/kernrift_verify_report_v1.schema.json`).
- `verify --report` now validates emitted report JSON against embedded schema with deterministic canonicalization.
- Expanded golden matrix for verify/report edge cases:
  - invalid UTF-8 contracts
  - schema-invalid contracts
  - signature mismatch
  - invalid signature/public key parsing
  - report overwrite refusal
- Aligned verify report output writing to guarded safe-write behavior (no overwrite + staged write flow).

### Notes
- User-visible product update: verify report format and coverage are now regression-locked in golden tests.

## v0.2.8 - 2026-02-23

### Changed
- Infra-only: release pipeline now signs/verifies archives only (`.tar.gz`, `.zip`).
- `.sha256` files remain unsigned convenience artifacts.

### Notes
- No compiler behavior changes vs v0.2.7.

## v0.2.7 - 2026-02-23

### Changed
- Fixed Windows cosign self-verification identity regex in release workflow.

### Notes
- `v0.2.6` introduced portable Linux checksums + signature self-verify, but release failed on Windows identity regex mismatch; use `v0.2.7`.

## v0.2.6 - 2026-02-23

### Changed
- Linux release checksum files now use archive basenames (portable `sha256sum -c` outside CI workspace layout).
- Release pipeline now self-verifies cosign signatures/certificates before uploading artifacts.

### Notes
- Infra-only release: no compiler behavior changes vs v0.2.5.

## v0.2.5 - 2026-02-23

### Changed
- Added `kernriftc --version` / `kernriftc -V` output (`kernriftc <semver>`) for release automation checks.

### Notes
- `v0.2.4` introduced release gating/signing workflow changes but failed release execution due missing CLI `--version`; use `v0.2.5`.

## v0.2.4 - 2026-02-23

### Changed
- Release pipeline now runs `fmt`/`clippy`/`test` gates before packaging artifacts.
- Release pipeline now signs artifacts with cosign keyless and publishes `.sig` + `.cert` files.
- Release build uses `--locked` and enforces tag/version match (`vX.Y.Z` == `kernriftc --version`).

### Notes
- This release is product-aligned and supersedes infra-only release tags (`v0.2.1`, `v0.2.2`).

## v0.2.3 - 2026-02-23

### Changed
- Infra: CI guards + release automation; no compiler behavior changes since v0.2.0.
- Versioning policy: tags/releases now track `kernriftc --version` (product-aligned).

### Notes
- v0.2.1 and v0.2.2 were infra-only tags; v0.2.3 is the aligned product tag.

## v0.2.0 - 2026-02-22

### Added
- Integrated policy gate in `check`:
  - `kernriftc check --policy <policy.toml> <file.kr>`
  - `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>`
- Policy evaluator command:
  - `kernriftc policy --policy <policy.toml> --contracts <contracts.json>`
- Canonical contracts artifact outputs from `check`:
  - `--contracts-out`, `--hash-out`, `--sign-ed25519`, `--sig-out`
- Artifact verification command:
  - `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> [--sig <contracts.sig> --pubkey <pubkey.hex>]`

### Changed
- Policy diagnostics are now deterministic and code-prefixed:
  - `policy: <CODE>: <message>`
- Policy `max_lock_depth` is evaluated from `report.max_lock_depth`.
- Exit code split is enforced:
  - `0` success
  - `1` policy/verification deny
  - `2` invalid input/config/schema/decode/tooling errors

### Safety Hardening
- Embedded contracts schema is used for validation (distro-safe, no repo path dependency).
- `check` refuses overwriting existing output files.
- Output writes use staged temp files before commit.
- `verify` now requires UTF-8 contracts content and schema/version-valid contracts payload (not only hash/signature match).
