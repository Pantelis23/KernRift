# KRIR v0.1

## Purpose

KRIR v0.1 is the normalized, analyzable representation used by KernRift KR0.x to enforce lock/context/effect/capability constraints and emit deterministic contract artifacts.

KRIR v0.1 is intended to be:

- small and explicit,
- deterministic to serialize and diff,
- stable enough for regression locking in tests and CI.

## Non-Goals (v0.1)

KRIR v0.1 explicitly does not cover:

- modules/namespaces beyond a single source module,
- generics/templates,
- async/await/coroutines,
- LSP/editor protocol features,
- backend machine code generation,
- kernel integration/runtime loading.

KRIR v0.1 in this document remains the analysis-first contract used by `check`, `analyze`, and artifact emit. The executable subset contract is defined separately in `docs/spec/krir-executable-subset-v0.1.md` so backend work does not treat this full fact-oriented representation as already codegen-ready.

## Surface Grammar (KR0.x accepted language)

The parser currently accepts the following minimal surface form.

A frontend ergonomics audit and backlog for the current KR0 surface lives in `docs/design/kr0_frontend_ergonomics_inventory.md`.
A compact copyable authoring guide for the canonical KR0 surface lives in `docs/spec/kr0-canonical-authoring-reference.md`.

```ebnf
Module      := { Item }
Item        := ModuleCapsDirective | FunctionDecl

ModuleCapsDirective := "@module_caps" "(" [CsvIdent] ")" ";"

FunctionDecl := { QualifierOrAttr } "fn" Ident "(" ")" (";" | Block)
QualifierOrAttr := "extern" | Attribute

Attribute   := "@" Ident [ "(" [AttrArgs] ")" ]
AttrArgs    := RawTextBalancedParens

Block       := "{" { Statement ";" } "}"

Statement   := Invocation
Invocation  := Ident "(" [RawTextBalancedParens] ")"

CsvIdent    := Ident { "," Ident } [ "," ]
Ident       := /[_A-Za-z][_A-Za-z0-9]*/
```

Notes:

- Only empty parameter lists `()` are supported.
- Comments use `// ...` to end-of-line.
- Canonical fact lists accept one optional trailing comma:
  - `@ctx(thread, boot,)`
  - `@eff(block,)`
  - `@caps(PhysMap,)`
  - `@module_caps(PhysMap,)`
- Module-level MMIO base declarations are supported in KR0 typed MMIO slice:
  - `mmio NAME = INT_LITERAL;`
  - `NAME` must be unique across module MMIO declarations.
  - RHS must be an integer literal (decimal or hex).
- Module-level MMIO register declarations are supported in KR0 typed MMIO slice:
  - `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;`
  - `TYPE in {u8,u16,u32,u64}`
  - `ACCESS in {ro,wo,rw}`
  - `BASE` must resolve to a declared module MMIO base.
  - `REG` must be unique within `BASE`.
- Statement forms are lowered by callee name:
  - `acquire(LockClass)`
  - `release(LockClass)`
  - `yieldpoint()`
  - `mmio_read<T>(addr)` where `T in {u8,u16,u32,u64}`
  - `mmio_write<T>(addr, value)` where `T in {u8,u16,u32,u64}`
  - `raw_mmio_read<T>(addr)` where `T in {u8,u16,u32,u64}`
  - `raw_mmio_write<T>(addr, value)` where `T in {u8,u16,u32,u64}`
    - `addr` must be one of:
      - identifier
      - integer literal
      - identifier + integer literal
    - `value` must be one of:
      - identifier
      - integer literal
  - `mmio_read()` / `mmio_write()` / `raw_mmio_read()` / `raw_mmio_write()` are rejected as legacy non-addressful forms
  - otherwise `call(callee)`.

### Canonical KR0 Frontend Spellings

This spec treats the following spellings as canonical for frontend-facing KR0 code:

- `@ctx(...)`
- `@eff(...)`
- `@caps(...)`
- `@module_caps(...)`
- `critical { ... }`

This spec uses those spellings by default in examples and guidance, even when a compatibility alias is still accepted by the compiler.

Compatibility aliases that remain accepted today are non-canonical:

| Compatibility surface | Canonical spelling | Status |
|---|---|---|
| `@thread_entry` | `@ctx(thread)` | accepted stable alias |
| `@irq_handler` | `@ctx(irq)` | accepted experimental alias |
| `@may_block` | `@eff(block)` | accepted experimental alias |
| `@irq_legacy` | `@ctx(irq)` | deprecated alias; diagnostics should steer users back to the canonical spelling |

Additional frontend conventions:

- Compiler diagnostics and migration surfaces classify non-canonical spellings as `compatibility aliases` or `deprecated aliases`, and always point to the canonical replacement.
- `kernriftc check --canonical <file.kr>` is the opt-in authoring gate for accepted non-canonical frontend spellings; it reports deterministic canonicalization findings without changing lowering semantics.
- Accepted legacy unary fact shorthands remain non-canonical:
  - `@irq` -> `@ctx(irq)`
  - `@noirq` -> `@ctx(thread, boot)`
  - `@alloc` -> `@eff(alloc)`
  - `@block` -> `@eff(block)`
  - `@preempt_off` -> `@eff(preempt_off)`
- Rejected legacy control-point spellings should also point to the canonical replacement:
  - `@yieldpoint` -> `yieldpoint()`
- `@critical` is a whole-function attribute; `critical { ... }` is the canonical block-scoped critical-region form.
- `extern` declarations should use the canonical fact skeleton:
  - `extern @ctx(...) @eff(...) @caps() fn name();`
- Compatibility fixtures under `tests/living_compiler/*alias*.kr` exist to lock accepted alias behavior; they are not the recommended spelling for new code.

## KRIR v0.1 Semantics

### Module semantics

- `@module_caps(...)` defines module-wide available capabilities.
- `mmio NAME = INT_LITERAL;` defines module-level symbolic MMIO base declarations.
- `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;` defines module-level MMIO register metadata.
- All function cap checks are evaluated against module caps in KR0.x.

### Function fact semantics

- Facts come from attributes and defaults:
  - `ctx_ok`: defaults to `{boot, thread}` for non-extern functions.
  - `eff_used`: defaults to empty; `yieldpoint/mmio_*` add effects.
  - `caps_req`: defaults to empty.
- Extern functions must declare `@ctx(...)` and `@eff(...)` explicitly.

### Operation semantics

- `acquire/release` define lock stack transitions and lock ordering edges.
- `yieldpoint()` marks a scheduler yield point and contributes to yield analysis.
- `call()` adds call-graph edges and participates in interprocedural checks.
- `mmio_read/mmio_write/raw_mmio_read/raw_mmio_write` mark `mmio` effect usage.
- Typed MMIO scalar width and operands are preserved in KRIR ops as `ty`, structured `addr`, and structured `value` (write only).
- When an MMIO address uses an identifier base (`IDENT` or `IDENT + OFFSET`), that base must resolve to a declared module MMIO base.
- When an MMIO address uses `IDENT` or `IDENT + OFFSET`, the access resolves against declared MMIO registers for that base.
  - `IDENT` is interpreted as `IDENT + 0` for register validation.
  - Register offsets are matched by normalized numeric value (for example `4`, `0x04`, and `0X4` are equivalent).
  - Duplicate semantic offsets under a base are rejected even when literal spellings differ.
  - Duplicate absolute register addresses are rejected deterministically even across different bases when `BASE_ADDR + OFFSET` collides numerically.
  - `mmio_read<T>(IDENT + OFFSET)` and `mmio_read<T>(IDENT)` require register access `ro` or `rw`.
  - `mmio_write<T>(IDENT + OFFSET, value)` and `mmio_write<T>(IDENT, value)` require register access `wo` or `rw`.
  - `T` must match the declared register `TYPE`.
- Integer-literal MMIO addresses remain valid without declaration.
  - If an integer literal exactly matches a declared register absolute address (`BASE_ADDR + OFFSET`), access and width checks are enforced for that register.
  - If no exact absolute match exists and the module declares no MMIO structure (`mmio` / `mmio_reg`), integer-literal MMIO behavior remains unchanged.
  - If the module declares MMIO structure and no exact absolute match exists, unmatched raw integer-literal MMIO requires `@module_caps(MmioRaw)`.
  - Without `@module_caps(MmioRaw)`, unmatched raw integer-literal MMIO is rejected deterministically.
- `raw_mmio_read/raw_mmio_write` are explicit escape-hatch operations:
  - they require `@module_caps(MmioRaw)`,
  - they preserve typed/structured operand forms,
  - they bypass MMIO register lookup/access/width validation.

### Check/analyze semantics

- `check` enforces ctx/effect/cap/lockgraph constraints.
- Analysis computes:
  - lock ordering edges,
  - `max_lock_depth`,
  - per-function `no_yield_spans` in thread context.
- Recursion is rejected in KR0.1 analysis.
- KRIR module JSON includes deterministic `mmio_bases` metadata when non-empty:
  - `[{ "name": "<IDENT>", "addr": "<INT_LITERAL>" }, ...]`
- KRIR module JSON includes deterministic `mmio_registers` metadata when non-empty:
  - `[{ "base": "<IDENT>", "name": "<IDENT>", "offset": "<INT_LITERAL>", "ty": "u{8|16|32|64}", "access": "{ro|wo|rw}" }, ...]`

## Contracts ABI (v1)

Schema file: `docs/schemas/kernrift_contracts_v1.schema.json`

Top-level fields:

- `schema_version: "kernrift_contracts_v1"`
- `capabilities`
  - `module_caps: string[]`
  - `symbols: [{ name, caps_req[] }]`
- `facts`
  - `symbols: [{ name, is_extern, ctx_ok[], eff_used[], caps_req[], attrs{noyield,leaf,hotpath,lock_budget} }]`
- `lockgraph`
  - `edges: [{from,to}]`
  - `max_lock_depth: u64`
- `report`
  - `max_lock_depth: u64`
  - `no_yield_spans: map<fn_name, u64 | "unbounded">`

## Verifier Claims (`kernriftc verify` today)

`verify` validates artifact integrity and ABI conformance for a contracts bundle.

Guaranteed today:

- SHA-256 check: `contracts bytes` must match `--hash` value.
- UTF-8 check: contracts bytes must decode as UTF-8.
- Schema check: contracts JSON must validate against embedded `kernrift_contracts_v1` schema.
- Schema-version gate: `schema_version == kernrift_contracts_v1`.
- Optional signature check: if `--sig` + `--pubkey` are provided, Ed25519 signature must verify over exact contracts bytes.

Not guaranteed today:

- semantic re-analysis of source `.kr`,
- policy evaluation (that is `kernriftc policy` or `check --policy`),
- provenance/identity checks beyond explicit signature and hash inputs,
- execution/runtime safety guarantees.

## Verify Report ABI (v1)

Schema file: `docs/schemas/kernrift_verify_report_v1.schema.json`

`kernriftc verify --report <path>` emits a deterministic JSON report with:

- `schema_version: "kernrift_verify_report_v1"`
- `result: "pass" | "deny" | "invalid_input"`
- `inputs`: contracts/hash required, sig/pubkey optional
- `hash`: expected/computed SHA-256 and match flag
- `contracts`: UTF-8/schema validity and observed contracts schema version
- `signature`: whether signature checking was requested and whether it validated
- `diagnostics`: stable sorted diagnostic strings

Field contract (v1):

| Field | Type | Notes |
|---|---|---|
| `schema_version` | const string | must equal `kernrift_verify_report_v1` |
| `result` | enum string | one of `pass`, `deny`, `invalid_input` |
| `inputs.contracts` | string | basename when absolute path was passed |
| `inputs.hash` | string | basename when absolute path was passed |
| `inputs.sig` | string or null | present when `--sig` is provided |
| `inputs.pubkey` | string or null | present when `--pubkey` is provided |
| `hash.expected_sha256` | string or null | null when hash input was not parseable/readable |
| `hash.computed_sha256` | string or null | null when contracts bytes could not be read |
| `hash.matched` | bool | true only when expected==computed |
| `contracts.utf8_valid` | bool | contracts file decoded as UTF-8 |
| `contracts.schema_valid` | bool | contracts JSON validated against contracts schema |
| `contracts.schema_version` | string or null | observed contracts schema version |
| `signature.checked` | bool | true when signature verification was requested |
| `signature.valid` | bool or null | null when signature verification was not requested |
| `diagnostics[]` | string array | sorted + deduped diagnostics |

Strictness policy:

- JSON objects are closed (`additionalProperties: false`) at all levels.
- Unknown keys are rejected by schema validation.
- Unknown `result` enum values are rejected by schema validation.

Output write safety matches other guarded outputs:

- refuses overwrite when destination exists,
- stages writes through temp files and commits via rename.

## Structured Output Conventions

These transport rules apply to commands that already expose structured JSON output.

Non-canonical `migrate-preview` CLI note:

- `kernriftc migrate-preview <file.kr>` is valid for file input and defaults omitted
  `--surface` to `stable`.
- non-canonical `kernriftc migrate-preview --stdin` remains unsupported;
  `--stdin` is only accepted with `--canonical-edits`.

Current JSON-capable command surfaces:

- `kernriftc inspect-artifact <artifact> --format json`
- `kernriftc verify-artifact-meta --format json <artifact> <meta.json>`
- `kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>`
- `kernriftc check --format json --policy <policy.toml> <file.kr>`
  - today this emits structured JSON only for policy denials, reusing
    `kernrift_policy_violations_v1`
- `kernriftc check --canonical --format json <file.kr>`
  - this emits structured canonical findings under
    `kernrift_canonical_findings_v2`
- `kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>`
  - this emits a non-mutating canonical edit-plan preview under
    `kernrift_canonical_edit_plan_v2`
- `kernriftc fix --canonical --write --format json <file.kr>`
  - this emits canonical fix application results under
    `kernrift_canonical_fix_result_v1`
- `kernriftc fix --canonical --dry-run --format json <file.kr>`
  - this emits a non-mutating canonical fix preview under
    `kernrift_canonical_fix_preview_v1`

Artifact JSON consumer migration note:

- `kernriftc inspect-artifact --format json` now emits
  `kernrift_inspect_artifact_v2`
- `kernriftc verify-artifact-meta --format json` now emits
  `kernrift_verify_artifact_meta_v2`
- `kernrift_inspect_artifact_v1` and `kernrift_verify_artifact_meta_v1`
  remain preserved as historical contracts
- machine consumers must dispatch on `schema_version`, not command name alone
- both artifact v2 envelopes add required `file` using the exact artifact path
  seen by the CLI

Canonical JSON consumer migration note:

- `kernriftc check --canonical --format json` now emits
  `kernrift_canonical_findings_v2`
- `kernriftc migrate-preview --canonical-edits --format json` now emits
  `kernrift_canonical_edit_plan_v2`
- `kernrift_canonical_findings_v1` and `kernrift_canonical_edit_plan_v1`
  remain preserved as historical contracts
- machine consumers must dispatch on `schema_version`, not command name alone
- both canonical v2 envelopes add required `file` for file/stdin source labeling

Transport invariants:

- structured JSON payload is written to `stdout` only
- `stderr` remains empty when a structured payload is emitted
- structured JSON output ends with a trailing newline
- commands must not mix human text lines with JSON payload bytes in JSON mode
- exit codes remain command-specific and are authoritative alongside the payload
- when the payload has a versioned schema, `schema_version` must be present

Contributor lock for future JSON-capable commands:

- new JSON-capable commands must add `cli_contract` transport assertions
- prefer reusing `assert_json_transport` in
  `crates/kernriftc/tests/cli_contract.rs`
- new tests must lock `stdout`-only JSON transport, empty `stderr`, trailing
  newline termination, and `schema_version` presence when the payload is
  versioned

### Structured Output Command Matrix

| Command surface | JSON mode flag | Schema / payload | Transport expectations | Exit code behavior | Structured output is emitted on |
|---|---|---|---|---|---|
| `kernriftc inspect-report --report <verify-report.json> --format json` | `--format json` | `kernrift_inspect_report_v1` | on success: `stdout` only, empty `stderr`, trailing newline, `schema_version` present; on invalid input/malformed report: no JSON payload, deterministic `stderr` | `0` on successful report inspection, `2` on invalid input or malformed report | success only |
| `kernriftc inspect-artifact <artifact> --format json` | `--format json` | `kernrift_inspect_artifact_v2` | on success: `stdout` only, empty `stderr`, trailing newline, `schema_version` present; on unsupported or malformed artifact bytes: no JSON payload, deterministic `stderr` | `0` on successful inspection, `1` on unsupported or malformed artifact bytes | success only |
| `kernriftc verify-artifact-meta --format json <artifact> <meta.json>` | `--format json` | `kernrift_verify_artifact_meta_v2` | on pass or mismatch: `stdout` only, empty `stderr`, trailing newline, `schema_version` present; on invalid input: no JSON payload, deterministic `stderr` | `0` pass, `1` mismatch / deny, `2` invalid input | pass and mismatch only |
| `kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>` | `--format json` | `kernrift_policy_violations_v1` | `stdout` only, empty `stderr`, trailing newline, `schema_version` present | `0` pass, `1` deny, `2` invalid input | pass and deny |
| `kernriftc check --format json --policy <policy.toml> <file.kr>` | `--format json` | `kernrift_policy_violations_v1` on policy denial | `stdout` only, empty `stderr`, trailing newline, `schema_version` present when JSON is emitted | `1` on policy deny with JSON payload; other check failures remain command-specific | policy deny only today |
| `kernriftc check --canonical --format json <file.kr>` | `--format json` | `kernrift_canonical_findings_v2` | `stdout` only, empty `stderr`, trailing newline, `schema_version` present | `0` when no canonical findings exist, `1` when findings are emitted, `2` invalid input | canonical pass and canonical-findings deny |
| `kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>` | `--format json` | `kernrift_canonical_edit_plan_v2` | `stdout` only, empty `stderr`, trailing newline, `schema_version` present | `0` on successful preview emission, `1` parse/frontend failure, `2` invalid input | canonical edit-plan preview only |
| `kernriftc fix --canonical --write --format json <file.kr>` | `--format json` | `kernrift_canonical_fix_result_v1` | `stdout` only, empty `stderr`, trailing newline, `schema_version` present | `0` on successful fix or no-op, `1` parse/frontend/write failure, `2` invalid input | changed and unchanged successful fix runs |
| `kernriftc fix --canonical --dry-run --format json <file.kr>` | `--format json` | `kernrift_canonical_fix_preview_v1` | `stdout` only, empty `stderr`, trailing newline, `schema_version` present | `0` on successful preview, `1` parse/frontend failure, `2` invalid input | changed and unchanged successful dry-run previews |

### Structured Output Test Coverage Matrix

Representative `cli_contract` coverage for the current JSON-capable commands:

| Command surface | Exact payload lock | Schema validation lock | Transport lock | Parity lock |
|---|---|---|---|---|
| `kernriftc inspect-report --report <verify-report.json> --format json` | yes: `inspect_report_json_is_stable_and_exact` | yes: same exact JSON test validates `kernrift_inspect_report_v1` | yes: `inspect_report_json_transport_is_stdout_only_and_newline_terminated` and `inspect_report_json_rejects_malformed_report_without_emitting_json` | n/a |
| `kernriftc inspect-artifact <artifact> --format json` | yes: `inspect_artifact_json_outputs_are_exact_for_fixture_matrix` | yes: `inspect_artifact_json_contract_shape_is_stable_across_krbo_elf_and_asm` | yes: `inspect_artifact_json_transport_is_stdout_only_and_newline_terminated` and `inspect_artifact_json_rejects_malformed_bytes_without_emitting_json` | n/a |
| `kernriftc verify-artifact-meta --format json <artifact> <meta.json>` | yes: `verify_artifact_meta_json_reports_success_with_schema_marker` and `verify_artifact_meta_json_reports_mismatch_with_schema_marker` | yes: same tests lock `schema_version` and envelope shape | yes: `verify_artifact_meta_json_transport_is_stdout_only_and_newline_terminated`, `verify_artifact_meta_json_rejects_invalid_input_without_emitting_json`, and `verify_artifact_meta_json_rejects_malformed_metadata_without_emitting_json` | n/a |
| `kernriftc policy --format json --policy <policy.toml> --contracts <contracts.json>` | yes: `policy_json_irq_raw_mmio_forbid_is_exact_and_structured`, `policy_json_irq_raw_mmio_allowlist_deep_path_is_exact_and_structured` | yes: `policy_json_schema_accepts_scalar_list_and_empty_list_evidence_variants` | yes: transport is asserted inside the exact JSON tests via `assert_json_transport` | n/a |
| `kernriftc check --format json --policy <policy.toml> <file.kr>` | yes: check-policy JSON is byte-compared against standalone policy JSON in `check_json_policy_irq_raw_mmio_forbid_matches_policy_json_contract_exactly` and `check_json_policy_irq_raw_mmio_allowlist_helper_path_matches_policy_json_contract_exactly` | yes: same parity tests validate against `kernrift_policy_violations_v1` schema | yes: same parity tests assert stdout-only transport via `assert_json_transport` | yes: exact parity to `kernriftc policy --format json` on policy denial |
| `kernriftc check --canonical --format json <file.kr>` | yes: `check_canonical_json_reports_legacy_unary_shorthands_exactly` and `check_canonical_json_reports_accepted_aliases_under_experimental_surface` | yes: `canonical_findings_json_schema_accepts_empty_and_nonempty_reports` plus the same exact JSON tests | yes: the exact JSON tests assert stdout-only transport via `assert_json_transport` | n/a |
| `kernriftc migrate-preview --canonical-edits --format json --surface stable <file.kr>` | yes: `migrate_preview_canonical_edits_json_reports_legacy_unary_exactly` and `migrate_preview_canonical_edits_json_reports_experimental_aliases_exactly` | yes: `canonical_edit_plan_json_schema_accepts_empty_and_nonempty_reports` plus the same exact JSON tests | yes: the exact JSON tests assert stdout-only transport via `assert_json_transport` | n/a |
| `kernriftc fix --canonical --write --format json <file.kr>` | yes: `fix_canonical_json_rewrites_legacy_unary_shorthands_exactly`, `fix_canonical_json_rewrites_accepted_aliases_under_experimental_surface_exactly`, and `fix_canonical_json_is_empty_for_canonical_source` | yes: `canonical_fix_result_json_schema_accepts_empty_and_nonempty_reports` plus the same exact JSON tests | yes: the exact JSON tests assert stdout-only transport via `assert_json_transport` | n/a |
| `kernriftc fix --canonical --dry-run --format json <file.kr>` | yes: `fix_canonical_dry_run_json_reports_legacy_unary_shorthands_exactly`, `fix_canonical_dry_run_json_reports_accepted_aliases_under_experimental_surface_exactly`, and `fix_canonical_dry_run_json_is_empty_for_canonical_source` | yes: `canonical_fix_preview_json_schema_accepts_empty_and_nonempty_reports` plus the same exact JSON tests | yes: the exact JSON tests assert stdout-only transport via `assert_json_transport` | n/a |

### New JSON Command Checklist

When introducing a new JSON-capable command:

- document the command surface in the structured-output command matrix
- document its coverage in the structured-output test coverage matrix
- add `cli_contract` transport assertions
- reuse `assert_json_transport` where applicable
- add or reference a schema when the payload is versioned
- preserve `stdout`-only, empty-`stderr`, trailing-newline transport behavior

These rules lock transport behavior only. They do not redefine command payload schemas.

## Verify Exit Codes

| Exit | Meaning |
|---|---|
| `0` | verify passed |
| `1` | deny (`HASH_MISMATCH` / `SIG_MISMATCH`) |
| `2` | invalid input/config (`UTF-8`, schema/decode, key/sig parsing, report write refusal) |

## Determinism Rules (KR0.x)

- Canonical contracts output (`check --contracts-out`) is minified JSON with stable field/array ordering.
- Dedupe + sorting rules:
  - module caps, symbol lists, lock edges, and policy diagnostics are sorted/deduped.
- Deterministic diagnostics:
  - checker and policy diagnostics are sorted by `(pass/code, message)`.
  - verify report diagnostics are sorted and deduped before emit.
- Path normalization:
  - verify report `inputs.*` uses basenames when absolute input paths are provided.
  - diagnostics in verify report are path-stripped to remove absolute-path instability.
- Stable JSON object key order:
  - report/contract/verify-report JSON is emitted from canonicalized object trees.
- No timestamps or host-specific nondeterministic values in governance artifacts.
