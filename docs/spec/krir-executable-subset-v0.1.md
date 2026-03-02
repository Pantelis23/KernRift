# Executable KRIR Subset v0.1

## Purpose

Executable KRIR v0.1 is the smallest backend-facing contract for KernRift.

It exists to make the executable boundary explicit without pretending the existing
analysis-first KRIR is already a machine-code IR.

Executable KRIR consumes canonical executable semantics. It does not lower directly from raw surface syntax or governed surface aliases.

For the supported subset, canonical executable semantics lowers deterministically to one explicit `entry` block per function.

Executable KRIR v0.1 is still target-independent. A separate backend target contract defines machine-facing facts such as register sets, ABI, stack alignment, symbol naming, and section naming. Only a tiny part of that target contract is exercised by the current executable subset.

For KR0.x, target-specific lowering from executable KRIR is defined separately in:

- `docs/spec/compiler-owned-object-linear-subset-v0.1.md`
- `docs/spec/x86_64-asm-linear-subset-v0.1.md`
- `docs/spec/x86_64-object-linear-subset-v0.1.md`

Those lowerings remain downstream of executable KRIR and do not make target assembly or object format the semantic authority. The compiler-owned binary object format is the primary internal machine-facing artifact; assembly remains a downstream reference form and ELF is exported from the compiler-owned object format as a downstream compatibility form.

This subset is intentionally narrow:

- linear function bodies only,
- direct calls only,
- unit return only,
- no target-machine details,
- no register allocation,
- no stack-frame lowering,
- no object or assembly emission in this phase.

## Relationship to analysis KRIR

`KrirModule` remains the analysis-first IR used for:

- context/effect/capability checking,
- lock and critical-region analysis,
- deterministic contract/report emission.

`ExecutableKrirModule` is a separate contract for future backend work.

The two representations may share facts, but they are not interchangeable:

- analysis facts are not executable ops,
- executable ops are not sufficient to replace semantic facts.

## Executable module model

Executable KRIR module fields:

- `module_caps: string[]`
- `functions: ExecutableFunction[]`
- `call_edges: CallEdge[]`

Determinism requirements:

- module caps are sorted and deduped,
- functions are sorted by name,
- function facts are sorted and deduped,
- signature parameter order is preserved exactly as constructed,
- block list order is preserved exactly as constructed,
- call edges are sorted and deduped.

In executable KRIR v0.1, canonicalization must not reorder executable interface or block structure. Lowering is responsible for emitting canonical parameter and block order for the supported subset.

## Executable function model

Each executable function contains:

- `name`
- `is_extern`
- `signature`
- `facts`
- `entry_block`
- `blocks`

### Executable signature v0.1

Executable signature is intentionally tiny:

- `params: []`
- `result: unit`

This means:

- executable KRIR v0.1 supports zero-argument functions only,
- all functions return `unit`,
- parameter lowering and non-unit return lowering are explicitly out of scope.

### Attached facts

Executable functions still carry semantic facts:

- `ctx_ok`
- `eff_used`
- `caps_req`
- `attrs`

These remain semantic metadata, not executable instructions.

## Executable block model

Each block contains:

- `label`
- `ops`
- `terminator`

Every executable function must:

- contain at least one block,
- contain exactly one block named by `entry_block`,
- have unique block labels.

## Executable ops v0.1

Supported executable ops:

- `Call { callee }`

Semantics:

- direct call only, and only to a defined non-extern executable function,
- no arguments yet,
- no bound result value yet,
- participates in `call_edges`.

## Executable terminators v0.1

Supported executable terminators:

- `Return { value: Unit }`

Semantics:

- explicit function exit,
- terminator result type must match the function signature result type,
- only `unit` return is supported in v0.1.

## Value model v0.1

Supported executable value kinds:

- `unit`

This is deliberately the smallest coherent result model.

It answers:

- what a result is in the subset,
- what a return means,
- what future lowering must preserve,

without pretending a general value system already exists.

## Out of scope

Executable KRIR v0.1 does not cover:

- general expressions,
- locals or SSA temporaries,
- memory loads/stores,
- branching or CFG joins,
- parameters,
- non-unit returns,
- target ISA details,
- calling convention lowering,
- register allocation,
- stack-frame layout,
- assembly or object emission in this contract itself.

## Relationship to backend target contracts

Backend target contracts define:

- target identity,
- register sets,
- caller/callee-saved partitions,
- stack alignment,
- symbol naming assumptions,
- section naming assumptions,
- freestanding toolchain assumptions.

Executable KRIR v0.1 lowers first to a compiler-owned binary object format. That object format preserves explicit symbols and fixups, including unresolved external call intent for downstream export. Target-specific assembly and ELF compatibility/export remain downstream of that internal object boundary and do not become semantic truth. In the current x86_64 compatibility subset, ELF symbol partitioning, symbol indices, relocation order, and `.rela.text` relationships are all derived from the compiler-owned object representation, and external inspection/linker smoke checks operate only on that emitted downstream artifact.

## Validation rules

Executable KRIR validation must reject:

- extern executable functions,
- functions with parameters,
- missing entry block,
- duplicate block labels,
- terminator result mismatch.

## Why this boundary exists

This subset keeps KernRift sovereign:

- KernRift source remains the real language,
- canonical KernRift semantics remain the semantic truth,
- KRIR remains the compiler-owned internal contract,
- backend work must start from explicit executable KRIR, not from Rust/C/LLVM as semantic truth.
