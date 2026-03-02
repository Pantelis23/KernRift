# x86_64 Object Linear Subset v0.1

## Purpose

This document defines the first x86_64 ELF compatibility/export path emitted by KernRift:

- compiler-owned object format
- plus the `x86_64-sysv` backend target contract
- to a deterministic ELF64 relocatable object subset

This is intentionally downstream of the compiler-owned binary object format defined in `docs/spec/compiler-owned-object-linear-subset-v0.1.md`.
It remains intentionally tiny.
It is additionally smoke-checked against standard ELF inspection tools, relocatable linker flows, the smallest practical final-link flows, and narrow runtime execution flows for compatibility only; those tools do not become compiler truth.

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

- are derived from compiler-owned object fixups only
- currently use `R_X86_64_PLT32` for unresolved external direct calls
- target `.rela.text`
- are smoke-checked against external ELF tooling using the emitted bytes, not by re-deriving semantics from KRIR

Symbol ordering and indices are explicit for this subset:

- symbol table entry 0 is the null symbol
- symbol table entry 1 is the `.text` section symbol
- defined function symbols follow in deterministic name order
- undefined external function symbols follow in deterministic name order
- relocation symbol indices must refer to those emitted symbol-table entries directly
- defined function symbols must occupy non-overlapping `.text` ranges

## Determinism rules

- function order is canonical executable KRIR order
- direct call order is executable-op order
- relocation order follows compiler-owned fixup patch-offset order
- emitted ELF header, section order, symbol order, symbol indices, relocation order, and bytes are deterministic
- same executable KRIR input produces byte-identical object bytes

`.rela.text` rules for the current subset:

- `.rela.text` is absent when there are no unresolved external call relocations
- `.rela.text` is present when unresolved external call relocations exist
- `.rela.text.sh_link` points to `.symtab`
- `.rela.text.sh_info` points to `.text`
- `.rela.text.sh_entsize = 24`
- relocation patch offsets are unique in the current subset
- undefined external symbols are invalid unless at least one relocation references them

## Relationship to the compiler-owned object format

The compiler-owned object format is primary for internal backend work because it preserves:

- explicit symbols
- explicit fixups
- binary-first deterministic serialization

This ELF subset exists for downstream compatibility/export. It must not replace the compiler-owned object format as the internal backend boundary.
Compatibility smoke checks confirm that standard ELF inspection tools accept the emitted bytes for the supported subset, that the smallest relocatable linker flows accept the resulting objects, that the narrowest practical final-link flows accept them when paired with a tiny startup shim and resolver where needed, and that those final-linked artifacts execute successfully when the environment permits it. Those tools remain downstream observers only and do not become compiler truth.

Current user-facing export path:

- `kernriftc --surface stable --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=elfobj -o <output.o> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=elfobj -o <output.o> <file.kr>`

This writes the downstream ELF relocatable compatibility/export artifact directly. It participates in the same surface-aware CLI contract as other compiler flows while preserving stable-default behavior. Optional `--meta-out` writes deterministic header-level metadata for automation and CI convenience only, including repo-relative source provenance when the resolved input path lies under the Git repo root. It does not make ELF the internal backend truth.

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
