# x86_64 Object Linear Subset v0.1

## Purpose

This document defines the first x86_64 ELF compatibility/export path emitted by KernRift:

- compiler-owned object format
- plus the `x86_64-sysv` backend target contract
- to a deterministic ELF64 relocatable object subset

This is intentionally downstream of the compiler-owned binary object format defined in `docs/spec/compiler-owned-object-linear-subset-v0.1.md`.
It remains intentionally tiny.

## Layer boundary

The intended pipeline for the supported subset is:

- surface KernRift
- canonical executable semantics
- executable KRIR
- backend target contract
- compiler-owned object format
- x86_64 object linear subset

The emitted ELF object is downstream of executable KRIR and is exported from the compiler-owned object format. It is not the semantic truth of the language and is not the primary internal object contract.

## Supported lowering subset

Supported executable KRIR inputs:

- zero-parameter functions
- unit return
- exactly one explicit `entry` block per function
- ordered direct `Call` ops
- terminal `Return { value: Unit }`
- direct calls to:
  - defined non-extern functions in the same executable KRIR module, or
  - unresolved external function targets preserved in the compiler-owned object format

Rejected at this lowering boundary:

- multiple blocks
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
- `.rela.text` when unresolved external call relocations are present

Current implementation exports ELF from the compiler-owned object format for this compatibility path. ELF remains downstream of the compiler-owned object model rather than a peer lowering boundary.

## Text section encoding

Per function:

- each direct call lowers to `call rel32`
- terminal unit return lowers to `ret`

For the current subset:

- internal direct call targets are resolved directly during emission
- unresolved external direct call targets remain zero-displacement `call` sites plus explicit relocations derived from compiler-owned fixups
- no stack frame is emitted
- no prologue/epilogue is emitted beyond terminal `ret`

## Symbol policy

- one function symbol per lowered executable KRIR function
- zero or more undefined external function symbols derived from the compiler-owned object format
- function symbol names preserve source symbol names
- symbol order is deterministic and follows canonical executable KRIR function order
- symbol offsets and sizes are recorded relative to `.text`

Relocations:

- are derived from compiler-owned object fixups
- currently use `R_X86_64_PLT32` for unresolved external direct calls
- target `.rela.text`

## Determinism rules

- function order is canonical executable KRIR order
- direct call order is executable-op order
- emitted ELF header, section order, symbol order, and bytes are deterministic
- same executable KRIR input produces byte-identical object bytes

## Relationship to the compiler-owned object format

The compiler-owned object format is primary for internal backend work because it preserves:

- explicit symbols
- explicit fixups
- binary-first deterministic serialization

This ELF subset exists for downstream compatibility/export. It must not replace the compiler-owned object format as the internal backend boundary.

## Explicit non-goals

This subset does not define:

- argument passing
- non-unit return lowering
- stack frames
- locals or stack slots
- branching / CFG lowering
- linker integration
- executable generation
- any host-compiler fallback
