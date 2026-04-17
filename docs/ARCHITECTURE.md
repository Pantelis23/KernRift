# Architecture

The KernRift compiler (`krc`) is a self-hosting compiler written entirely in KernRift. It compiles itself to a bit-identical fixed point. No external assembler, linker, or C toolchain is involved — `krc` writes ELF, Mach-O, and PE headers plus native machine code directly to disk.

## Source Structure

```
src/
├── lexer.kr           Tokenizer (90+ token kinds)
├── ast.kr             Arena-based flat AST (32-byte nodes, 1-indexed)
├── parser.kr          Recursive descent + Pratt precedence climbing
├── analysis.kr        Safety passes (ctx, eff, lock, caps, critical)
├── ir.kr              SSA IR + x86_64 emitter (Linux/macOS/Windows/Android)
├── ir_aarch64.kr      AArch64 emitter from the same IR
├── codegen.kr         Legacy direct x86_64 codegen (SysV ABI)
├── codegen_aarch64.kr Legacy direct AArch64 codegen (AAPCS64)
├── format_macho.kr    macOS Mach-O header emission
├── format_pe.kr       Windows PE/COFF headers + import table
├── format_android.kr  Android ELF quirks (DT_FLAGS_1, soname)
├── format_archive.kr  AR archives, KRBO objects, KrboFat v2 (BCJ + LZ-Rift)
├── bcj.kr             Branch/call/jump filter for better compression
├── living.kr          Pattern detection + fitness scoring
├── formatter.kr       Source-level auto-formatter
├── runner.kr          `kr` — fat-binary slice extractor / launcher
├── runtime.kr         fmt_uint helper
└── main.kr            CLI, compile(), compile_fat()
```

## Compilation Pipeline

1. **Lex** — source text → flat token array (16 bytes per token)
2. **Parse** — tokens → arena AST (32 bytes per node, child/sibling links)
3. **Analyze** — effect/capability/locking passes over the AST
4. **Lower to IR** — AST → SSA IR instructions with virtual registers
5. **Liveness** — per-opcode live-in/live-out sets for all virtual registers
6. **Register allocation** — Chaitin-style graph coloring onto physical registers
7. **Emit** — per-target emitter (`ir.kr` for x86_64, `ir_aarch64.kr` for ARM64) writes raw machine bytes
8. **Fixup** — patch call displacements, RIP-relative / ADRP offsets, string addresses
9. **Write** — ELF / Mach-O / PE headers + code + data + strings straight to the output file

The `--legacy` flag bypasses steps 4–6 and uses the direct AST-walking codegen path instead. Legacy codegen remains available as a correctness oracle; IR is the default and the supported path forward.

## Key Design Decisions

- **Flat AST**: 32-byte nodes in a contiguous arena. No pointers, just indices.
- **SSA IR**: target-independent opcodes (90+), virtual registers, liveness, graph-coloring register allocator. Added in v2.8.2, replacing the "no IR" stance of earlier versions.
- **Per-target emitters, shared IR**: Linux/macOS/Windows/Android syscall conventions, Mach-O argc/argv in x0/x1, Windows IAT calls — all handled at emission time from the same abstract opcodes.
- **No external tools**: the compiler writes binaries directly; there is no assembler, linker, or libc in the build graph.
- **Variable dedup**: same-named variables in different if-branches share a slot.
- **Static access**: RIP-relative on x86_64, ADRP+ADD / LDR on AArch64.
- **Fat binary default**: `compile_fat()` runs the IR backend once per target, BCJ-filters the code, LZ-Rift-compresses each slice, and packs all eight into a KrboFat v2 `.krbo`.

## Bootstrap

```
released krc binary → krc (stage 1, from source)
krc → krc2 (stage 2, self-compiled)
krc2 → krc3 (stage 3)
krc3 → krc4 (stage 4)
krc3 == krc4 (bit-identical fixed point)
```

There is no Rust, no C, and no LLVM in the build. A released `krc` binary compiles the current source tree into the next `krc`. CI verifies the fixed point on every push across all eight platform targets.
