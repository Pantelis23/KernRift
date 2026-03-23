# Changelog

All notable changes to `kernriftc` are documented in this file.

## [Unreleased]

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
