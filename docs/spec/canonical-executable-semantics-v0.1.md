# Canonical Executable Semantics v0.1

## Purpose

Canonical executable semantics v0.1 is the compiler-owned boundary between:

- surface KernRift syntax,
- governed/adaptive surface forms,
- executable KRIR.

It exists so backend work can lower from explicit KernRift semantics rather than from raw surface syntax or from analysis-first KRIR.

## Layer boundary

Executable pipeline for the supported subset:

- surface KernRift
- canonical executable semantics
- executable KRIR

This boundary is distinct from:

- governed feature metadata,
- analysis-first KRIR,
- target-specific backend lowering.

## Supported subset

Canonical executable semantics v0.1 supports:

- non-extern function definitions,
- explicit extern declarations,
- zero-parameter signatures,
- unit result,
- linear function bodies,
- direct named calls to defined non-extern executable functions,
- direct named calls to declared extern functions,
- explicit canonical `return unit`.

Extern declarations remain distinct from executable function bodies in canonical executable semantics v0.1. They are valid call targets, but they are preserved as declarations rather than lowered as executable bodies.

## Canonical function model

Each canonical executable function contains:

- `name`
- `signature`
- `facts`
- `body`

Canonical executable module also contains:

- `extern_declarations`

Facts remain semantic metadata:

- `ctx_ok`
- `eff_used`
- `caps_req`
- `attrs`

These facts are normalized from explicit source attributes and governed surface aliases before executable lowering.

## Canonical body model

Canonical executable body v0.1 contains:

- ordered direct-call ops,
- explicit `return unit` terminator.

Canonicalization may reorder only set-like metadata.

Canonicalization must preserve:

- parameter order,
- executable call order.

## Surface normalization

Governed surface aliases are resolved before canonical executable semantics is formed.

Example:

- `@thread_entry` normalizes to canonical `@ctx(thread)` facts.

## Rejected / out of scope

Canonical executable semantics v0.1 rejects the following in executable function bodies:

- `critical { ... }`
- `yieldpoint()`
- `allocpoint()`
- `blockpoint()`
- `acquire(...)`
- `release(...)`
- `mmio_read<T>(addr)` / `mmio_write<T>(addr, value)` where `T` is a typed MMIO scalar

Also out of scope:

- general expressions,
- locals / SSA values,
- parameters,
- non-unit returns,
- branching / CFG joins,
- memory loads/stores,
- target-machine details.

## Why this boundary exists

This boundary keeps KernRift sovereign:

- KernRift source remains the real language,
- canonical KernRift semantics remain the semantic truth,
- executable KRIR is a downstream compiler contract,
- backend work does not get to borrow semantics from Rust, C, or LLVM.

## Lowering contract to executable KRIR

For the supported subset, lowering to executable KRIR is deterministic:

- one canonical executable function lowers to one executable KRIR function,
- declared externs lower to explicit executable KRIR extern declarations,
- one executable body lowers to one explicit `entry` block,
- ordered canonical direct calls lower to ordered executable `Call` ops,
- canonical `return unit` lowers to executable `Return { value: Unit }`.
