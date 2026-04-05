# KernRift Compiler Improvements: Strictness, Fat Binary Fixes, Register Cache

**Date**: 2026-04-06
**Status**: Approved

## Problem

1. The compiler silently compiles invalid code (wrong arg counts, missing returns, undeclared identifiers in some paths), producing binaries that segfault at runtime instead of reporting errors at compile time.
2. Fat binary generation crashes on Windows (file not written) and segfaults on Pi 400 (after writing).
3. Runtime performance is 1.2-2.3x slower than gcc -O0 due to all values going through stack memory.

## Implementation Order

1. Semantic validation pass (highest user impact)
2. Fat binary crash fixes (blocks benchmarks)
3. Register cache (performance)

---

## 1. Semantic Validation Pass

### Architecture

Extend `src/analysis.kr` with a pre-codegen validation phase. Runs after parsing, before codegen. Two-pass design:

**Pass 1 — Symbol collection**: Walk top-level AST nodes and build:
- **Function signature table** (max 4096 entries): `[name_tok(4), param_count(4), has_return(4), flags(4)]`
  - Flags: is_exported, is_naked, is_noreturn
- **Global symbol table** (max 8192 entries): `[name_tok(4), kind(4)]`
  - Kinds: static variable, enum value, struct type, type alias

**Pass 2 — Validation**: Walk all function bodies and check:

### Checks

**1. Function call argument count** (highest value — catches the most common segfault source)
- At every call site, resolve the function name in the signature table
- Compare actual arg count (walk ast_child/ast_next chain) vs declared param count
- Error: `error: 'foo' expects 3 arguments, got 2`
- Skip builtins (exit, print, println, alloc, write, file_open, etc.) — these are validated by codegen
- Skip call_ptr (dynamic dispatch, can't validate statically)

**2. Missing return in non-void functions**
- For every function with `-> type` return declaration, walk the body
- Check that all control-flow paths end in a `return` statement
- Simplified version: check that the last statement in the body is a return, or every branch of the last if/else has a return. Don't do full path analysis.
- Error: `error: function 'foo' missing return on some paths`

**3. Undeclared identifier pre-check**
- Build a scope stack: function params + local variable declarations
- At every identifier reference in expressions, check it exists in: local scope, static globals, enum values, or function names
- Error: `error: use of undeclared identifier 'x'` (with line number)
- This supplements the existing codegen check (which catches some but not all cases)

**4. Duplicate function definitions**
- During Pass 1, check for duplicate function names
- Error: `error: redefinition of function 'foo'`

### Error Reporting

All errors print to stderr in the format:
```
error: <message>
```

Errors increment a counter. After the validation pass, if error_count > 0, the compiler prints the count and exits with code 1 before entering codegen. This means no partial/corrupt output is produced.

### Integration Point

In `src/main.kr`, after `parse()` and before `codegen_init()`:
```
uint64 errors = run_semantic_checks(ast_root)
if errors != 0 {
    // print error count
    exit(1)
}
```

### What We Don't Check (deliberately)

- Type compatibility (uint64 vs uint32 vs pointers) — the language is weakly typed by design
- Array bounds — would require constant propagation
- Null pointer dereference — would require data flow analysis
- Builtin argument types — codegen handles these directly

---

## 2. Fat Binary Crash Fixes

### Pi 400 Segfault

**Hypothesis**: The `compile_fat` function (4992-byte frame) completes compilation and writes output, then crashes during the summary print. The integer println digit buffer overflow (temp_slot_1 overwriting local variables) was fixed in the x86_64 codegen but the ARM64 codegen may have the same pattern in `compile_fat`'s code path, OR there's a buffer overflow in the compression/output assembly code.

**Debug approach**:
1. Run on Pi with visible stderr to see exactly where it crashes (the fat binary output line is partially printed: `fat binary: x86_64(`)
2. Check if the ARM64 integer print digit buffer fix applies to all code paths
3. Check the output buffer size — 6-slice compressed data might exceed the allocated buffer

### Windows No Output

**Hypothesis**: The default output path computation for `.krbo` files may produce a path that Windows can't write to, OR the `compile_fat` function crashes before reaching the write call.

**Debug approach**:
1. Test with an explicit `-o output.krbo` path on Windows
2. Check if the issue is in path handling (backslashes) or in the compilation itself
3. The stack commit fix (1MB) should cover the 4992-byte frame, so this is likely not a stack issue

### Fix

Both are likely small targeted fixes once the crash point is identified. Expected scope: 5-20 lines per fix.

---

## 3. Register Cache

### Current State

The codegen has a minimal RAX cache (`rax_cached_var`) that tracks when RAX holds a known variable value, avoiding redundant loads. But it's very limited — it's invalidated on almost every operation.

### Design

Extend the existing cache to track **RAX and one scratch register (R11)** as a 2-entry LRU cache:

**Cache entry**: `[var_offset(uint64), register(uint64), valid(uint64)]`

**Operations**:
- `reg_cache_load(var_offset)` — if var_offset is cached in RAX or R11, emit `mov rax, R11` (or nothing if already in RAX) instead of `mov rax, [rsp+off]`. Otherwise, load from stack and update cache.
- `reg_cache_store(var_offset)` — after storing RAX to a stack slot, mark that slot as cached in RAX.
- `reg_cache_invalidate_all()` — on function calls, branches, labels. Clear both entries.
- `reg_cache_invalidate(var_offset)` — when a variable is overwritten by something other than the cached value.

**Where it helps most**:
- Binary operations: `a + b` currently loads both from stack. With cache, the second operand load can be skipped if it's already in a register.
- Loop variables: `i = i + 1` in a while loop loads and stores `i` every iteration. With cache, the load is eliminated after the first iteration.
- Sequential variable access: `p.x = 10; p.y = 20` — the struct base pointer load is eliminated for the second access.

**Where we DON'T cache**:
- Across function calls (ABI clobbers everything)
- Across branch targets (values may differ between paths)
- Temp slots (short-lived, not worth tracking)

### Scope

Only x86_64 codegen initially. ARM64 can follow the same pattern later. Expected: ~200-300 lines of changes to `emit_load_rax_from_stack`, `emit_store_rax_to_stack`, and the invalidation points.

### Expected Impact

Eliminate 30-50% of stack loads in tight loops. Should close the gap with gcc -O0 for loop-heavy benchmarks (bubble sort, matrix multiply). Fibonacci (pure recursion, no loops) will see less improvement since function calls invalidate the cache.

---

## Success Criteria

1. **Strictness**: A test program with wrong arg count, missing return, and undeclared var produces 3 compile errors and no binary output. All 102 existing tests still pass.
2. **Fat binary**: `krc source.kr -o output.krbo` succeeds without segfault on Pi 400 and produces a file on Windows.
3. **Register cache**: Bubble sort benchmark drops from 348ms to <200ms (closer to gcc -O0's 149ms).

## Files Modified

- `src/analysis.kr` — semantic validation pass (~400-600 new lines)
- `src/main.kr` — integration point (5-10 lines)
- `src/codegen.kr` — register cache, fat binary fixes
- `src/codegen_aarch64.kr` — fat binary fixes if needed
