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

## IR vs legacy in the shipped binaries

Not every target defaults to IR. The release recipe is:

| Binary | Flags | Why |
|--------|-------|-----|
| `krc-linux-x86_64`        | default (IR)     | IR + optimizer handles real-world code well |
| `krc-windows-x86_64.exe`  | default (IR)     | same |
| `krc-macos-x86_64`        | default (IR)     | same |
| `krc-android-x86_64`      | default (IR)     | same |
| `krc-linux-arm64`         | **`--legacy`**   | IR ARM64 miscompiles `compile_fat`; legacy is 13 % larger but correct |
| `krc-windows-arm64.exe`   | **`--legacy`**   | same |
| `krc-macos-arm64`         | **`--legacy`**   | same |
| `krc-android-arm64`       | **`--legacy`**   | same |
| `kr-*` (runner, all 8)    | default (IR)     | simple program, no compile_fat — IR is fine |

Inside `compile_fat` itself (building the 8-slice `.krbo`), every ARM64 slice dispatches to `gen_function_a64` (legacy) regardless of `emit_ir_mode`, so the arm64 slice users pull out of `krc.krbo` is also legacy-built. `--ir` (emit_ir_mode ≥ 2) forces the IR path through those slices for backend testing.

User-invoked `krc --arch=arm64 myprog.kr -o myprog` still defaults to IR — the miscompile is specific to the `compile_fat` function's shape.

## Android fat-binary runner

`src/runner.kr` (the `kr` tool) on Android prefers a filesystem-free exec path:

1. `memfd_create("kr", MFD_CLOEXEC)` — anonymous in-kernel fd
2. `write(fd, slice, slice_size)` — copy the BCJ-decoded slice into it
3. `execveat(fd, "", argv, envp, AT_EMPTY_PATH)` — kernel ignores the pathname and execs the fd directly

This bypasses the SELinux file-label transition Termux uses to block execve of user-owned binaries, avoids touching any noexec mount, and leaves nothing behind in the user's cwd. On kernels older than Linux 3.17 (no `memfd_create`) it falls back to the file-based path (chmod + execve + exit-120 shell-wrapper trampoline) that earlier releases used.

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
