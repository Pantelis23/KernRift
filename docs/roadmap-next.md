# KernRift Roadmap — Next Features

Post-v2.8.8. Items below are listed roughly in priority order; "Status" records
what's currently blocking each one.

---

## R1: Fix IR ARM64 `compile_fat` miscompile — ✅ RESOLVED 2026-04-19

**Root cause:** The IR ARM64 backend never implemented passing the 9th+
function argument on the caller's stack under AAPCS64. `compress_pair_best`
takes 9 parameters; the 9th (`out_pair_len_ptr`) landed on an
uninitialized caller-stack slot, and storing through it segfaulted.
The bug was silent on x86_64 (stack-spill is emitted there) and on
qemu-arm64 (different stack layout) — it only surfaced natively on
ARM64 hardware.

**Fix:** `src/ir_aarch64.kr` now reserves `IR_A64_OVERFLOW_RESERVE = 128`
bytes at the bottom of every function's frame. Spill slots, scratchpad
and saved-register regions shift up by that amount so spill offsets
stay stable across calls. IR_ARG for the 9th+ parameter STRs directly
to `[SP + (idx-8)*8]` — no SUB/ADD SP around the call, which was what
broke every earlier attempt (SUB SP would invalidate every spill slot
the same instruction needed to read from). Callee reads via the
unchanged `[SP + frame_size + (idx-8)*8]` and gets the right value.

**Validated:** Redmi Note 8 Pro, Android 11, arm64-v8a, via ADB.
Default 8-slice fat compile of `hello.kr` and full krc self-compile
both succeed (was SIGSEGV before). Deterministic 3/3. 433/433 tests
pass on x86_64; bootstrap fixed-point held at 1395040 bytes.

**Follow-up done:** Makefile and `.github/workflows/release.yml` no
longer pass `--legacy --arch=arm64` for any slice — all krc ARM64
variants now ship built through the IR backend. krc-linux-arm64
dropped from 996 KB → 879 KB (-12%).

---

## R2: IR x86_64 size regression _(closes the 10–34 % gap vs legacy)_

IR-compiled x86_64 krc is 10–34 % larger than the legacy direct-walking
codegen for the same source (see `benchmarks/BENCHMARKS.md`). Causes we
know of:

- Extra MOVs inserted between interference-graph colors when parameter
  pinning and spill coalescing don't pick the same register.
- Unnecessary `rip`-relative reloads where the value is still live in a
  callee-saved.
- No peephole after emission — no `mov rax, X ; mov rcx, rax` →
  `mov rcx, X`.

**Status:** Not started.

---

## R3: IR ARM64 runtime misfeatures

v2.8.8 closed str_eq, atomic_*, memcmp, f32 conversion, fmt_f64 padding,
and div-by-zero. Residual native-ARM64 CI failures suggest at least:

- `device_block_read_write` — uses `syscall_raw(mmap, 0x66666000, ...)`
  at an absolute VA that qemu-user can't honor; verify on real hardware.
- Compile-time fat-binary `custom_fat_smaller` — part of R1 above.

**Status:** waiting on hardware.

---

## R4: Compiler Diagnostics — self-aware error detection

Now that `cli_envp` / `--debug` div-by-zero / typed `bool` / typed
`char` / f-strings are in, the remaining diagnostics polish:

- **Type mismatch** — reject f64 assigned to u64 without explicit
  conversion (partial: strict `bool`/`char` already catch this class).
- **Uninitialized variables** — warning when a local is read before
  any write on all paths.
- **Array bounds checking** — opt-in runtime check for `arr[i]`
  under `--debug`.
- **Null pointer dereference** — opt-in runtime check for
  `load*`/`store*` under `--debug`.
- **Implicit truncation** — warning on `u8 b = some_u64`.
- **Unused variables** — warning for declared-but-never-read locals.

Existing diagnostics already in place: undeclared identifier,
missing-return path, unreachable code after return/break/exit,
argument count, stack-frame-over-4 KB warning.

**Status:** Not started (post-R1).

---

## R5-next: Mach-O / PE debug info

ELF DWARF 5 is fully complete now. IR backend wires vregs into
DW_OP_reg<N> / DW_OP_fbreg locations; legacy backend uses
DW_OP_fbreg from named stack slots. Both backends produce the same
DWARF sections (line table, compile_unit DIE, base types,
subprograms, variables).

Remaining: the two non-ELF executable formats. Each is its own
multi-day project because neither writer was designed for dynamic
section addition — header offsets are hand-patched at fixed layout.

### Mach-O — estimated 2-3 days

The existing emit_macho_headers_{x64,a64} produces a tightly-packed
header with __PAGEZERO + __TEXT + __DATA + __LINKEDIT segments at
fixed LC offsets (the patch offsets in main.kr are 136/216/280/...).
Adding __DWARF requires either:

  1. A new LC_SEGMENT_64 after __LINKEDIT with __debug_{line,info,
     abbrev,str} sections. Involves re-packing the load-command
     table and re-patching every offset in main.kr's Mach-O branch.
  2. Writing a separate .dSYM bundle (a Mach-O with only the
     __DWARF segment) — preferred by the Apple toolchain.

Content bytes can reuse `emit_elf_debug_footer` exactly — just the
segment / load-command wrapping differs.

### PE — estimated 1-2 days

Windows native is PDB (multi-megabyte MSF container, multi-week).
Practical path: embed DWARF in PE sections named `.debug_info` etc.
with `IMAGE_SCN_LNK_INFO | IMAGE_SCN_MEM_DISCARDABLE` flags. GDB on
Wine and MinGW-side tooling read this fine; MSVC's debugger does
not — if MSVC support matters, PDB is the only path.

The existing emit_pe_headers_{x64,a64} layout also uses hand-patched
section-table offsets; adding 4 debug sections means resizing the
section table and re-offsetting .text / .data / .idata RVAs.

### IR-backend variable locations — DONE

Landed in 7c1d0d4. ir_snapshot_debug_vars walks ir_var_map after
regalloc; each var records a DW_OP_reg<N> location if it has a
physical register, or DW_OP_fbreg <spill-offset> if spilled. GDB
reads both forms correctly on the default backend.

## R5: Debug symbols (DWARF + PDB) — ELF MVP RESOLVED 2026-04-19

Landed in a450a76..c1accfa:
- `.debug_line` / `.debug_abbrev` / `.debug_str` / `.debug_info` for ELF
- IR backend wired to record source tokens per instruction
- `DW_TAG_subprogram` DIEs per function with low_pc/high_pc/decl_line
- `DW_LNS_set_prologue_end` flag on first real statement
- `DW_TAG_base_type` DIEs for u8/u16/u32/u64/i8/i16/i32/i64/f32/f64/bool
- `DW_TAG_variable` DIEs for legacy backend locals + params
- GDB `info functions`, `info locals`, `break <fn>` all work

Gaps tracked in "R5-next" above (Mach-O/PE, IR-backend variable
locations). ELF is the default target and fully debuggable now.

---

## R6: JIT / "living compiler" mode

Currently `krc lc` analyses source and reports patterns. A next step is
a REPL-ish mode that lowers and executes expressions immediately using
the IR backend's in-memory buffer.

**Status:** Not started.

---

## Completed since earlier drafts

- ~~F6: Custom fat binary pairs~~ — shipped in v2.8.x via `--targets=`
- ~~F4: IR Optimizer + Register Allocator~~ — shipped in v2.8.2;
  constant folding, DCE, CSE, graph-coloring regalloc, liveness
- ~~Strict `bool` type~~ — v2.8.3
- ~~Strict `char` type and `'x'` literals~~ — v2.8.3
- ~~Typed `print` / `println` pipeline (fmt_f64, fmt_bool)~~ — v2.8.3
- ~~Variadic `print` / `println`~~ — v2.8.3
- ~~f-string interpolation~~ — v2.8.3
- ~~`compile_fat` 18 GB RSS leak~~ — v2.8.6

---

## Known small issues

- ARM64 f16 conversions not implemented (test gated on x86_64 only).
- `naked_fn` / `asm_*` tests are x86-only (inline asm uses raw x86
  opcodes); already gated on `$ARCH != aarch64` in `tests/run_tests.sh`.
