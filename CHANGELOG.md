# Changelog

All notable changes to `kernriftc` are documented in this file.

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
