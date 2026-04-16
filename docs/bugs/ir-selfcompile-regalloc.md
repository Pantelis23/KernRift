# IR Self-Compile Register Allocator Bug

**Date**: 2026-04-16  
**Status**: Open  
**Severity**: Blocker for `--ir` default switch  
**Affects**: IR→IR bootstrap chain (stage 2 crash)

## Summary

The IR backend passes 237/237 small-program tests and 310/310 full test suite tests. Stage 1 (legacy-compiled krc2 → IR binary) works perfectly. But Stage 2 (IR binary → IR binary) produces a broken binary that crashes on startup.

The root cause is a **register allocator correctness bug** where chained arithmetic with 3+ variables produces wrong results. The interference graph doesn't properly track operand liveness across intermediate computations.

## Reproducer

Compiled by `krc_s1` (the stage-1 IR binary):

```kernrift
// Expected: exit(42).  Actual: exit(34)
fn main() {
    uint64 a = 10
    uint64 b = 20
    uint64 c = 12
    exit(a + b + c)
}
```

Simpler forms work:
- `exit(10 + 20 + 12)` → 42 (correct, single expression)
- `uint64 a = 10; uint64 b = 20; exit(a + b)` → 30 (correct, 2 vars)
- `uint64 a = 10; uint64 b = 20; uint64 c = 12; exit(a + b + c)` → **34** (wrong, 3 vars)

The error is `42 - 8 = 34`, suggesting `c` (12) is read as 4 — possibly a shifted or partially-clobbered register value.

## What Works

1. **310/310 test suite** passes (compiled by legacy krc2)
2. **237/237 IR-specific tests** pass
3. **Stage 1 binary** (legacy→IR) runs correctly:
   - `--version` works
   - Compiles hello.kr with both `--ir` and `--legacy`
   - Compiles programs up to ~35,000 lines (the full compiler minus main)
   - All arithmetic, control flow, function calls work for small programs
4. **factorial(5)=120**, **fibonacci(10)=55** — recursion works
5. The full compiler source **compiles** without errors via IR (1.2MB output, 980ms)

## What Fails

1. **Stage 2 binary** (IR→IR) crashes on startup (SIGSEGV before main prints anything)
2. The crash is NOT in the ELF structure — the entry point bytes are valid x86
3. The crash is specifically caused by **the main() function's code** being wrong
4. With a simplified main (just `write + exit`), the full-source binary works perfectly (34,748 lines)
5. Even simple function calls like `alloc(3145728)` crash when main has enough variables
6. The 3-variable addition test confirms the arithmetic codegen is wrong

## Root Cause Analysis

### Architecture

The IR register allocator is an interference-graph-based greedy coloring with 5 physical registers (rbx, r12-r15). Vregs 1-5 get physical registers if they don't interfere; the rest are spilled to `[rsp + offset]`. Spilled values are loaded via r10/r11 temporaries.

### The Bug

For `a + b + c` with 3 local variables:

```
v1 = const 10    (a)
v2 = const 20    (b)  
v3 = const 12    (c)
v4 = add v1, v2  (a + b = 30)
v5 = add v4, v3  (30 + c = should be 42)
```

The interference graph should show:
- v3 is live from its definition until the `add v4, v3` instruction
- v4 is defined at `add v1, v2` — at this point v3 IS live
- Therefore v3 ↔ v4 should interfere
- They should get different physical registers

**Hypothesis**: The interference builder doesn't mark v3 as live at v4's definition point. This allows the coloring to assign v3 and v4 the same register. Then `mov v4_reg, v1; add v4_reg, v2` overwrites v3's value (since v4_reg == v3's register), and the subsequent `add v5, v4, v3` reads the clobbered value.

### Why Small Programs Work

Small programs compiled directly by krc2 (legacy) use the legacy codegen, not IR. The IR tests (237 tests) are compiled by the legacy-compiled krc2 using `--ir`. The legacy krc2's `ir_emit_x86_function` is compiled by the legacy codegen, which uses a different register allocation strategy (stack-based, no graph coloring). So the IR codegen in the legacy binary works correctly.

When the IR binary (s1) compiles a small program, its own IR codegen (compiled via IR) is used. If the IR codegen functions themselves have wrong register allocation (due to this same bug), they produce wrong code for the target program. The 3-variable addition is the simplest manifestation.

## Bugs Found and Fixed So Far

### Critical fixes (all committed):

1. **argc/argv initialization** (commit 6514d9a): IR main() prologue reads [rsp]/[rsp+8] before pushes
2. **Opcode collision** (commit 6514d9a): IR_FSQRT32/IR_TIME_NS overlapped signed comparison range 116-119
3. **Implicit RET_VOID** (commit 6514d9a): Void functions without return fell through to next function
4. **Liveness LOAD/STORE exclusion** (commit 8b76564): IR_STORE's src2 IS a vreg (the value), was incorrectly excluded
5. **Interference-based allocator** (commit c98da57): Replaced trivial sequential assignment with proper graph coloring
6. **Binary op dst==src2 clobber** (commit c98da57): ADD/SUB/MUL/AND/OR/XOR/SHL/SHR swap or use temp when dst==src2
7. **While loop var map restore** (commit e9a3c21): Restore var map to header vregs after while body
8. **Branch fixup overflow** (commit b2a0be3): Dynamic sizing based on ir_bb_count for large functions (gen_expr needs 1673+)
9. **Liveness 256-insn limit** (commit b2a0be3): Replace fixed insn_list[256] with direct iteration
10. **Missing imm-as-vreg interference** (commit b2a0be3): MEMCPY/MEMSET/RET3/CAS/FMA length vregs
11. **IR_MEMSET literal vs vreg** (commit b2a0be3): Pass size as IR_CONST vreg
12. **Local var shadowed by static** (commit b2a0be3): Check ir_var_get before static_lookup
13. **COPY read-after-write hazard** (commit b2a0be3): Two-phase copy via temporaries
14. **Fixup buffer safety margin** (commit 442d8df): ir_bb_count*4 with 2048 minimum

### What these fixes achieved:

- Parse_module now correctly reads token kinds (was reading 0)
- Semantic checks pass
- Code generation loop runs
- The IR binary compiles programs, runs --version
- Stage 1 binary is fully functional for small programs
- Bootstrap chain compiles (stage 2 binary is produced) but output is broken

## Files

- `src/ir.kr`: IR backend (~8000+ lines) — all fixes are here
- `src/codegen.kr`: Legacy codegen — only `emit_ir_mode` default change
- `src/main.kr`: CLI flag handling, argc/argv init

## How to Reproduce

```bash
# Build from clean state
cp dist/krc-linux-x86_64 build/krc2 && chmod +x build/krc2
cat src/lexer.kr src/ast.kr src/parser.kr src/codegen.kr src/codegen_aarch64.kr \
    src/ir.kr src/format_macho.kr src/format_pe.kr src/format_archive.kr \
    src/format_android.kr src/bcj.kr src/analysis.kr src/living.kr src/runtime.kr \
    src/formatter.kr src/main.kr > build/krc.kr
./build/krc2 --arch=x86_64 build/krc.kr -o build/krc2.new
mv build/krc2.new build/krc2 && chmod +x build/krc2

# Stage 1: legacy → IR (works)
./build/krc2 --ir --arch=x86_64 build/krc.kr -o /tmp/krc_s1
chmod +x /tmp/krc_s1
/tmp/krc_s1 --version  # OK

# Arithmetic bug in s1's IR codegen:
echo 'fn main() { uint64 a = 10; uint64 b = 20; uint64 c = 12; exit(a + b + c) }' > /tmp/test.kr
/tmp/krc_s1 --ir --arch=x86_64 /tmp/test.kr -o /tmp/test_out
chmod +x /tmp/test_out
/tmp/test_out  # exits 34 instead of 42

# Stage 2: IR → IR (crashes)  
/tmp/krc_s1 --ir --arch=x86_64 build/krc.kr -o /tmp/krc_s2
chmod +x /tmp/krc_s2
/tmp/krc_s2 --version  # SIGSEGV
```

## Next Steps

1. **Dump the IR** for the 3-variable addition compiled by s1: `/tmp/krc_s1 --ir --emit=ir --arch=x86_64 /tmp/test.kr` — check if interference is computed correctly
2. **Disassemble** the output binary to see the actual x86 instructions for the addition
3. **Trace the interference graph builder** for this specific function — verify which vregs interfere
4. **Check ir_graph_color** output — which vregs get which colors
5. The fix will likely be in the interference builder's backwards instruction walk, specifically in how it adds uses to the live set for instructions that precede a definition

## Key Insight

The bug manifests as a **phase-ordering** issue: the IR codegen in the legacy-compiled binary works correctly, but the IR codegen in the IR-compiled binary (same source code, different compilation) produces wrong code. This means the bug is in how the IR backend compiles ITS OWN functions — specifically the register allocator and code emission functions in ir.kr itself. When these functions are compiled by a correct IR backend, they work. When compiled by themselves (bootstrap), the result has subtle arithmetic errors.
