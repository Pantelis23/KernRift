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

CsvIdent    := Ident { "," Ident }
Ident       := /[_A-Za-z][_A-Za-z0-9]*/
```

Notes:

- Only empty parameter lists `()` are supported.
- Comments use `// ...` to end-of-line.
- Statement forms are lowered by callee name:
  - `acquire(LockClass)`
  - `release(LockClass)`
  - `yieldpoint()`
  - `mmio_read()`
  - `mmio_write()`
  - otherwise `call(callee)`.

## KRIR v0.1 Semantics

### Module semantics

- `@module_caps(...)` defines module-wide available capabilities.
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
- `mmio_read/mmio_write` mark `mmio` effect usage.

### Check/analyze semantics

- `check` enforces ctx/effect/cap/lockgraph constraints.
- Analysis computes:
  - lock ordering edges,
  - `max_lock_depth`,
  - per-function `no_yield_spans` in thread context.
- Recursion is rejected in KR0.1 analysis.

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
