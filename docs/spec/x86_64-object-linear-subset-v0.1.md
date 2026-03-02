# x86_64 Object Linear Subset v0.1

## Purpose

This document defines the first machine-facing binary artifact emitted by KernRift:

- executable KRIR
- plus the `x86_64-sysv` backend target contract
- to a deterministic ELF64 relocatable object subset

This is the first real object-emission step. It is still intentionally tiny.

## Layer boundary

The intended pipeline for the supported subset is:

- surface KernRift
- canonical executable semantics
- executable KRIR
- backend target contract
- x86_64 object linear subset

The emitted object is downstream of executable KRIR. It is not the semantic truth of the language.

## Supported lowering subset

Supported executable KRIR inputs:

- zero-parameter functions
- unit return
- exactly one explicit `entry` block per function
- ordered direct `Call` ops
- terminal `Return { value: Unit }`
- direct calls only to defined non-extern functions in the same executable KRIR module

Rejected at this lowering boundary:

- multiple blocks
- missing defined direct call targets
- any executable KRIR shape outside the current linear subset

## Emitted artifact kind

The emitted artifact is:

- ELF64 relocatable object (`ET_REL`)
- little-endian
- `EM_X86_64`
- one executable `.text` section
- one `.symtab`
- one `.strtab`
- one `.shstrtab`

This subset does not emit relocation sections in v0.1.

## Text section encoding

Per function:

- each direct call lowers to `call rel32`
- terminal unit return lowers to `ret`

For the current subset:

- all direct call targets are internal to the same object
- all `rel32` displacements are resolved directly during emission
- no stack frame is emitted
- no prologue/epilogue is emitted beyond terminal `ret`

## Symbol policy

- one function symbol per lowered executable KRIR function
- function symbol names preserve source symbol names
- symbol order is deterministic and follows canonical executable KRIR function order
- symbol offsets and sizes are recorded relative to `.text`

## Determinism rules

- function order is canonical executable KRIR order
- direct call order is executable-op order
- emitted ELF header, section order, symbol order, and bytes are deterministic
- same executable KRIR input produces byte-identical object bytes

## Explicit non-goals

This subset does not define:

- relocation sections for unresolved externs
- argument passing
- non-unit return lowering
- stack frames
- locals or stack slots
- branching / CFG lowering
- linker integration
- executable generation
- any host-compiler fallback
