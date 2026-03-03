# x86_64 Assembly Linear Subset v0.1

## Purpose

This document defines the first real target-specific lowering step for KernRift:

- executable KRIR
- plus the `x86_64-sysv` backend target contract
- to deterministic textual x86_64 SysV assembly

This is still a tiny subset. It does not add general codegen machinery, and it is not the primary internal backend artifact. The compiler-owned binary object format remains the primary internal machine-facing boundary.

`kernriftc` may export this downstream/debug/reference artifact directly:

- `kernriftc --surface stable --emit=asm -o <output.s> <file.kr>`
- `kernriftc --emit=asm -o <output.s> <file.kr>`

This does not change backend truth. Metadata sidecars remain limited to `krbo` and `elfobj` in this slice.

## Layer boundary

The intended pipeline for the supported subset is:

- surface KernRift
- canonical executable semantics
- executable KRIR
- backend target contract
- compiler-owned object format
- x86_64 linear assembly subset

The assembly model is downstream of executable KRIR and downstream of the compiler-owned object format. Current implementation exports this assembly from the compiler-owned object format rather than lowering from executable KRIR as a peer path. It is not the semantic truth of the language and is not the primary backend artifact.

## Supported lowering subset

Supported executable KRIR inputs:

- zero-parameter functions
- unit return
- exactly one explicit `entry` block per function
- ordered direct `Call` ops
- terminal `Return { value: Unit }`

Rejected at this export boundary:

- multiple blocks
- unresolved external direct call targets preserved in the compiler-owned object format
- any executable KRIR shape outside the linear subset

## Assembly syntax

The emitted syntax is deterministic GNU-style text assembly:

- section directive: `.text`
- function label: `<source_symbol>:`
- direct call: `call <callee_symbol>`
- return: `ret`

Indentation:

- labels are unindented
- instructions use four leading spaces

## Symbol and section behavior

- section is always `.text`
- function symbol names are preserved from the executable KRIR function names
- no implicit symbol prefix is added

## Prologue/epilogue policy

For this subset:

- no prologue
- no epilogue beyond terminal `ret`

This is deliberate. The current executable subset has:

- zero parameters
- no locals
- no stack slots
- no non-unit returns

So stack-frame machinery would be fake progress here.

## Determinism rules

- functions are emitted in deterministic compiler-owned object symbol order
- direct calls are emitted in compiler-owned fixup order within each function
- output text is byte-stable for the same executable KRIR input

## Explicit non-goals

This subset does not define:

- register allocation
- argument passing lowering
- non-unit return lowering
- stack-slot allocation
- branching / CFG lowering
- object emission
- linker integration
- external assembler invocation
