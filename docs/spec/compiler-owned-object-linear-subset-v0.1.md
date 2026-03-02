# Compiler-Owned Object Linear Subset v0.1

## Purpose

This document defines the first primary internal machine-facing artifact emitted by KernRift:

- executable KRIR
- plus the `x86_64-sysv` backend target contract
- to a deterministic compiler-owned binary object format

This format is intentionally small. It exists so backend work can preserve symbols and fixup intent without making assembly text or ELF the internal truth.

## Layer boundary

The intended pipeline for the supported subset is:

- surface KernRift
- canonical executable semantics
- executable KRIR
- backend target contract
- compiler-owned object linear subset

This object format is downstream of executable KRIR. It is machine-facing, but it is not semantic truth.

## Supported lowering subset

Supported executable KRIR inputs:

- zero-parameter functions
- unit return
- exactly one explicit `entry` block per function
- ordered direct `Call` ops
- terminal `Return { value: Unit }`
- direct calls to:
  - defined non-extern functions in the same executable KRIR module, or
  - unresolved external function targets preserved explicitly in the object format

Rejected at this lowering boundary:

- multiple blocks
- any executable KRIR shape outside the current linear subset

## Object identity

The compiler-owned object format v0.1 includes:

- magic: `KRBO`
- version: `0.1`
- object kind: `linear_relocatable`
- target id: `x86_64-sysv`
- endianness: little
- pointer width: 64
- deterministic format revision: `2`

## Code payload

The current subset uses one code payload corresponding to the target contract text section:

- section name: `.text`

Per function:

- each direct call lowers to `E8 00 00 00 00`
- terminal unit return lowers to `C3`

The call encoding uses zeroed displacement bytes in the primary object format because fixup intent is preserved explicitly rather than erased into fully resolved machine bytes.

## Symbol model

The format records one function symbol per lowered executable KRIR function:

- name
- kind
- offset
- size

Symbols also record whether they are:

- defined in the code payload, or
- unresolved external declarations carried for downstream export

Symbol names currently preserve executable KRIR function names directly.

Symbol ordering is deterministic and follows canonical executable KRIR function order.

## Fixup model

Direct calls produce explicit fixup entries. Fixups are mandatory in this format even when current call targets are internal and known.

Each fixup records:

- source symbol
- patch offset
- fixup kind
- target symbol
- width bytes

Current v0.1 fixup kind:

- `x86_64_call_rel32`

For this subset:

- `patch_offset` points at the first displacement byte after the `E8` opcode
- `width_bytes = 4`

This preserves compiler-owned relocation intent for later export or patching work.

Downstream export rules must consume these fixups directly. Later formats must not recover relocation intent by re-reading executable KRIR ops.

When the target symbol is unresolved external:

- the object still emits `E8 00 00 00 00`
- the fixup still records the target symbol
- the unresolved target is preserved as an explicit symbol entry rather than being erased

## Determinism rules

- function order is canonical executable KRIR order
- direct call order is executable-op order
- symbol order is deterministic
- fixup order preserves original call order
- emitted bytes are byte-stable for identical executable KRIR input

## Explicit non-goals

This subset does not define:

- linker integration
- executable generation
- argument passing
- non-unit return lowering
- stack frames
- locals or stack slots
- branching / CFG lowering
- assembly as the primary backend artifact
- ELF as the primary internal object model

## Relationship to ELF export

ELF is a downstream compatibility/export form, not the primary internal backend boundary.

The current repository exports ELF relocatable objects from this compiler-owned object format through a compatibility wrapper. That export step must not make ELF the semantic authority or the internal truth of the compiler.

For the current x86_64 linear subset, ELF export must derive:

- `.text` bytes from the compiler-owned code payload,
- defined function symbols from compiler-owned defined text symbols,
- undefined external function symbols from compiler-owned undefined external symbols,
- relocation entries from compiler-owned fixups in patch-offset order.

Compatibility smoke checks against standard ELF inspection tools and the smallest downstream linker flows, including the narrowest practical final-link and runtime checks, may validate the bytes emitted from this object format, but those tools do not replace the compiler-owned object representation as internal truth.

The compiler-owned object format remains the sole internal truth for:

- code bytes,
- symbol identity and definition state,
- fixup identity,
- relocation intent.

Current user-facing export path:

- `kernriftc --surface stable --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --surface stable --emit=krbo -o <output.krbo> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> --meta-out <output.json> <file.kr>`
- `kernriftc --emit=krbo -o <output.krbo> <file.kr>`

This writes the compiler-owned object bytes directly. It participates in the same surface-aware CLI contract as other compiler flows while preserving stable-default behavior. Optional `--meta-out` writes deterministic header-level metadata for automation and CI convenience only, including repo-relative source provenance when the resolved input path lies under the repo root. It does not perform linking or execution.
