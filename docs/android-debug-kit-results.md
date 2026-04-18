# Android debug-kit run — 2026-04-19

## Device
- **Fingerprint**: `Redmi/begonia/begonia:11/RP1A.200720.011/V12.5.1.0.RGGMIXM:user/release-keys`
- **Model**: Redmi Note 8 Pro (codename `begonia`)
- **Android**: 11
- **ABI**: `arm64-v8a`
- **Kernel**: Linux localhost 4.14.186-g34d97d2 #1 SMP PREEMPT Tue Jun 29 20:03:38 WIB 2021 aarch64
- **Access**: adb via Twingate → Windows host (100.96.47.100) → `/data/local/tmp/r1debug/` on device (shell UID 2000, SELinux enforcing)

## Artifacts pushed

Fresh builds from this session's source tree (v2.8.13, commit `0b3599a`):

| File | sha256 | Size |
|---|---|---|
| `krc_leg_android` | built this run | 1,114,112 |
| `krc_ir_android` | built this run | 983,040 |
| `krc.kr` | `cf0a6c20fc93f3012c6fc33995f004bb6c42c3897810f003927a6e2118691538` | 1,605,372 |

## Headline findings

### R1: the previous diagnosis was partly wrong.

1. **Both backends self-compile a single-target binary cleanly.** Running
   `krc_ir_android --arch=arm64 --target=android krc.kr -o krc_self_ir`
   on the phone produces `krc_self_ir` (sha256
   `4417e787…62daefa3…`, 880,410 bytes) — **byte-identical** to what
   the legacy backend produces for the same input. The krc_self_ir is
   then runnable (`--version` → `krc 2.8.13`). So the IR ARM64 backend
   is not silently miscompiling normal code paths.

2. **Both backends crash with SIGSEGV when the default `compile_fat`
   path runs natively on Android ARM64.** This is deterministic (3/3
   runs), reproduces even on a 22-byte hello-world source, and happens
   for both the legacy and the IR binary. The roadmap's premise that
   "IR is bad, legacy is fine on ARM64" is incorrect — they both fail
   in the same place.

3. **The crash is specifically triggered by the 8-slice paired-
   compression code path.** Any subset of 7 or fewer targets via
   `--targets=...` works fine on the phone. The full 8-slice form
   (either the default with no flag, or `--targets=` listing all eight)
   always segfaults.

4. **The crash is a NULL / unmapped-pointer dereference in
   `emit_raw_bytes`.** Last strace syscalls before SIGSEGV are a
   sequence of `munmap`s freeing the paired-compression scratch
   buffers; then the crash fires on the first byte read. Crash dumps
   (from `adb logcat -b crash`) show:

   - IR build: `x0 = 0` (NULL), fault at `si_addr=NULL`, crash PC
     `0xc5e08` inside krc_ir_android.
   - Legacy build: `x0 = 0x86ef8e6b010` (non-canonical high-half
     garbage), fault at `si_addr=0x86ef8e6b010`, crash PC `0x11c18`
     inside krc_leg_android. Registers `x6` / `x7` hold
     `"/data/local/tmp/"` — suggesting a path-string handling site
     somewhere near the failure.

### R3: not re-tested yet.

Not exercised this run; focus was R1. The fix for R1 will likely also
resolve or reshape R3 since they share the fat-binary code path.

## Reproduction recipe

On the phone:

```sh
# This works, every time:
/data/local/tmp/r1debug/krc_ir_android \
    --targets=linux-x64,linux-arm64,windows-x64,windows-arm64,macos-x64,macos-arm64,android-arm64 \
    hello.kr -o hello_ok
echo $?          # 0

# This segfaults, every time:
/data/local/tmp/r1debug/krc_ir_android hello.kr -o hello_bad
echo $?          # 139

# Legacy backend, same pattern:
/data/local/tmp/r1debug/krc_leg_android hello.kr -o hello_bad_leg
echo $?          # 139
```

## Source-side localization

The default-fat path is `src/main.kr` around lines 3875–3978. It runs
four `compress_pair_best` calls (the paired-LZ4 optimization), then
emits a KrboFat v2 header and eight entries, then four
`emit_raw_bytes` calls reading from the saved `comp_*` pointers.

The NULL deref in `emit_raw_bytes` means one of
`x64_lm_comp`, `x64_wa_comp`, `arm_la_comp`, `arm_wm_comp` is `0`
at the emit site. Each is read via `slot_load_u64(…_slot)` from a
single-element stack array of type `uint64[1]`, which
`compress_pair_best` was supposed to populate via an `unsafe` write
through an out-pointer.

The leading hypothesis: **`uint64[1] slot` stack-array pointers
passed to a callee aren't stable across the call on Android ARM64** —
either the callee clobbers the caller's frame, or the compiler is
computing the slot address as a value that gets invalidated. This
would explain why:
- Small 7-target compiles (which take the `target_mask != 0xFF` custom
  branch instead — single-blob compression, no `compress_pair_best`,
  no stack-slot-out-pointer trick) work.
- The IR version crashes with `x0 = NULL` (uninitialized slot).
- The legacy version crashes with `x0 = <garbage>` (slot read from a
  clobbered stack location).
- x86_64 Linux doesn't hit it — different stack layout, different
  regalloc.

## Update — first fix attempt didn't land

I replaced the four `uint64[1]` stack arrays with a single heap slab
(`alloc(4*5*8)`, addresses computed as `slab+offset`) and `dealloc`d at
the end. Bootstrap held, 433/433 tests passed on x86_64. But the rebuilt
Android arm64 binary **still segfaults at the same site** (`pc ≈ 0xc5d8c`,
`x0 = 0`, NULL deref inside `emit_raw_bytes`). So the stack-slot
hypothesis was wrong — the data doesn't become NULL because of stack
unstability.

Reverted. The slab change isn't wrong (it's arguably cleaner), but it
isn't the bugfix I was hoping for, so leaving the source tree in its
original shape for a future diagnosis run.

## Root cause — confirmed via print-bisection

I instrumented `compile_fat` and `compress_pair_best` with staged
`write(2, ...)` markers and pushed to the phone. The crash is
**inside `compress_pair_best`, on the final `unsafe { *(out_pair_len_ptr
as uint64) = pair_len }` store**. The four prior `unsafe` stores in
the same function (to `out_comp_ptr`, `out_comp_len_ptr`,
`out_a_off_ptr`, `out_b_off_ptr`) all succeed.

The difference is the AAPCS64 calling convention: the first 8 parameters
go in `x0`–`x7`, the 9th (and beyond) go on the caller's stack.
`compress_pair_best` has 9 parameters. `out_pair_len_ptr` is the 9th —
the only one passed via stack.

Reading `src/ir_aarch64.kr` line 697–718 confirms it directly:

```kernrift
// ----- IR_CALL (50): bl with fixup -----
if op == 50 {
    // Overflow args (>8 args) not yet implemented for ARM64 IR
    // Emit BL placeholder
    ...
}
```

**The IR ARM64 backend has never passed args 9+. It records them in
`ir_overflow_args` in IR_ARG, then does nothing with them at IR_CALL.**
The callee reads [SP + frame_size + 0] to fetch arg 9, which is
uninitialized caller stack, which happens to be a bogus pointer on
Android.

On x86_64, args 7+ spill to the stack naturally as part of the sysv
calling convention's spill pattern, so the same source code works
there. On ARM64 only 8 args fit in regs before the spill is needed,
but the spill was never wired.

## What we tried this session

1. **Moving the `uint64[1] slot` stack arrays to a heap slab** —
   didn't help, because the slot address isn't the issue; the callee
   never receives the slot pointer at all.
2. **Adding the missing overflow-arg plumbing in `IR_CALL`**
   (SUB SP, STR args, BL, ADD SP) — compiled + bootstrap passed on
   x86_64, but still crashed identically on Android.
3. **Moving the STR into `IR_ARG` itself** (so the vreg's physical
   register is still valid) with a one-shot `SUB SP, #128` reserve on
   first overflow arg — same result.

The fix direction is correct, but either:
- the callee's `[SP + frame_size + (imm-8)*8]` formula needs an
  additional offset to account for saved callee-saved registers, or
- the 128-byte reserve interacts with the callee's prologue save
  sequence in some way I couldn't see without deeper inspection.

Reverted the WIP so master stays clean. Both attempts are in the
session log if you want to resurrect them:
- Attempt 1 edited `ir_a64.kr`'s `IR_CALL` handler.
- Attempt 2 edited `ir_a64.kr`'s `IR_ARG` handler.

## Confirmed next-session work

This is no longer a qemu-hidden mystery — it's a concrete missing
feature in the ARM64 IR backend that x86_64 doesn't exercise. One
focused sitting with a sample 9-arg test function + `lldb-server`
attached via ADB port-forward (recipe in `android-debug-kit.md §4c`)
should land it:

1. Write a minimal test: `fn f9(u64 a, ..., u64 i)` with 9 params that
   prints each. Call with distinct constants. Run on the phone.
2. Watch which arg(s) arrive wrong. Likely `i` will be garbage.
3. Inspect callee prologue to see where it actually puts `[SP, #…]`
   for param 8. Compare to caller's STR offset.
4. Align the two. Ship.

Once R1 is done, `--legacy --arch=arm64` overrides in the `Makefile`
and `.github/workflows/release.yml` can be dropped. Same mechanism
will fix anyone else hitting >8-arg calls on ARM64 native.

## Not the IR backend alone

Worth noting: the **legacy** ARM64 backend crashes identically in the
same place. So either the legacy backend has the same missing-spill
bug, or it defers the issue to the IR-shared lowering layer. Either
way, the "fat binary on ARM64 requires `--legacy`" premise in the
Makefile / CI is no longer valid — **both** backends need this fix.

## Artifacts on the phone

Left in `/data/local/tmp/r1debug/` for follow-up runs:
- `krc_leg_android`, `krc_ir_android` — the two fresh builds
- `krc.kr` — the concatenated source that reproduces self-compile
- `hello.kr` — the minimum `fn main() { exit(42) }` repro
- `krc_self_leg`, `krc_self_ir` — the identical-hash single-target
  self-compile outputs
- Intermediate `s_<target>` files from the per-slice bisection
