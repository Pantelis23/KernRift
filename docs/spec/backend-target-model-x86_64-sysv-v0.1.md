# Backend Target Model x86_64 SysV v0.1

## Purpose

This document defines the first explicit backend target contract for KernRift.

It exists to make future executable-KRIR lowering target a compiler-owned machine model rather than relying on Rust, C, LLVM, or any host-language/compiler semantics.

This branch defines the target contract only. It does not perform:

- instruction selection,
- register allocation,
- stack-frame lowering,
- assembly emission,
- object emission,
- linker integration.

## Layer boundary

The intended pipeline remains:

- surface KernRift
- canonical executable semantics
- executable KRIR
- backend target contract
- backend/codegen later

The target contract is downstream of executable KRIR. It is not semantic truth.

## Target identity

- `target_id = x86_64-sysv`
- `arch = x86_64`
- `abi = sysv`
- `endian = little`
- `pointer_bits = 64`

## Stack contract

- `stack_alignment_bytes = 16`

This is defined now for future stability even though the current executable subset does not yet lower stack frames.

## Integer register set

Deterministic register order:

- `rax`
- `rbx`
- `rcx`
- `rdx`
- `rsi`
- `rdi`
- `rbp`
- `rsp`
- `r8`
- `r9`
- `r10`
- `r11`
- `r12`
- `r13`
- `r14`
- `r15`

Special-purpose registers:

- stack pointer: `rsp`
- frame pointer: `rbp`
- instruction pointer: `rip`

## Saved-register partition

Caller-saved:

- `rax`
- `rcx`
- `rdx`
- `rsi`
- `rdi`
- `r8`
- `r9`
- `r10`
- `r11`

Callee-saved:

- `rbx`
- `rbp`
- `r12`
- `r13`
- `r14`
- `r15`

These sets are disjoint and both are subsets of the declared integer register set.

## Return convention

Current executable subset v0.1:

- unit return only
- no value register is consumed semantically

Future-facing scalar return convention, defined now for stability but not exercised in this branch:

- `integer_rax`
- this convention maps to `rax` for the first integer/scalar return value

## Argument convention

Current executable subset v0.1:

- zero parameters only
- argument registers are not exercised yet

Future-facing SysV integer argument register order, defined now for stability:

- `rdi`
- `rsi`
- `rdx`
- `rcx`
- `r8`
- `r9`

## Symbol and section assumptions

Function symbol naming:

- no implicit function-name prefix
- preserve KernRift source symbol names

Section naming assumptions:

- text: `.text`
- rodata: `.rodata`
- data: `.data`
- bss: `.bss`

## Freestanding assumptions

- no libc
- no host runtime
- assembler/linker bridge is future work and not yet exercised in v0.1

## Current subset vs future-facing fields

Exercised today by the current executable subset:

- target identity
- endianness
- pointer width
- stack alignment contract
- target freestanding stance
- symbol/section naming assumptions as backend planning inputs

Defined now for future stability but not yet exercised:

- caller/callee-saved partition
- scalar return convention and its `rax` mapping
- argument register order
- frame-pointer convention

## Non-goals

This target contract does not:

- define instruction selection,
- define prologue/epilogue rules,
- allocate registers,
- allocate stack slots,
- emit assembly,
- emit objects,
- define linker scripts,
- make LLVM or another host compiler the semantic authority.
