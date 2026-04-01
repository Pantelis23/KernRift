# Architecture

The KernRift compiler (`krc`) is a self-hosting compiler written entirely in KernRift. It compiles itself to a fixed point.

## Source Structure

```
src/
├── lexer.kr           Tokenizer (90+ token kinds)
├── ast.kr             Arena-based flat AST (32-byte nodes, 1-indexed)
├── parser.kr          Recursive descent + Pratt precedence climbing
├── codegen.kr         x86_64 code generation (SysV ABI)
├── codegen_aarch64.kr AArch64 code generation (AAPCS64)
├── format_macho.kr    macOS Mach-O header emission
├── format_pe.kr       Windows PE/COFF headers + import table
├── format_archive.kr  AR archives, KRBO objects, KrboFat v2 + LZ4
├── analysis.kr        Safety passes (ctx, eff, lock, caps, critical)
├── living.kr          Pattern detection + fitness scoring
├── runtime.kr         fmt_uint helper
└── main.kr            CLI, compile(), compile_fat()
```

## Compilation Pipeline

1. **Lex** — source text → flat token array (16 bytes per token)
2. **Parse** — tokens → arena AST (32 bytes per node, child/sibling links)
3. **Codegen** — AST walk → native machine code in output buffer
4. **Fixup** — patch call displacements, RIP-relative statics, string addresses
5. **Emit** — write ELF/Mach-O/PE headers + code + data + strings

## Key Design Decisions

- **Flat AST**: 32-byte nodes in a contiguous arena. No pointers, just indices.
- **No IR**: direct AST → machine code. No intermediate representation.
- **No optimization**: the compiler is a single-pass code generator.
- **Variable dedup**: same-named variables in different if-branches share a slot.
- **Dynamic temp slots**: BinOp/Compare allocate fresh slots to avoid nesting conflicts.
- **Static access**: RIP-relative on x86_64, MOVZ/MOVK+LDR/STR on AArch64.
- **Fat binary default**: compile_fat() runs both backends and LZ4-compresses into KrboFat.

## Bootstrap

```
kernriftc (Rust) → krc (stage 1)
krc → krc2 (stage 2, self-compiled)
krc2 → krc3 (stage 3)
krc3 → krc4 (stage 4)
krc3 == krc4 (fixed point)
```
