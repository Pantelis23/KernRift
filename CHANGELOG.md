# Changelog

All notable changes to `kernriftc` are documented in this file.

## v2.4.0 - 2026-04-08

### Added
- **uint16 pointer operations**: read/write through `u16` pointers in `unsafe`/`volatile` blocks (both x86_64 and ARM64).
- **ARM64 MSR/MRS system register access**: inline `asm` blocks can now read/write 20+ kernel-mode system registers via MRS/MSR instructions.
- **Volatile memory barriers**: `volatile { ... }` blocks now emit hardware memory barriers (`mfence` on x86_64, `DMB ISH` on ARM64).
- **Atomic builtins**: `atomic_load`, `atomic_store`, `atomic_cas`, `atomic_add` — emits `LOCK` prefix on x86_64 and `LDXR`/`STXR` sequences on ARM64.
- **Builtin argument count validation**: semantic analyzer now checks argument counts for all builtin functions at compile time.
- **std/color.kr**: `rgb`, `rgba`, `alpha_blend`, `color_lerp`, `darken`/`lighten`, 11 named color constants.
- **std/fixedpoint.kr**: 16.16 fixed-point math — `mul`, `div`, `sqrt`, `lerp`, `clamp`.
- **std/memfast.kr**: `memcpy32`/`memcpy64`, `memset32`/`memset64`.
- **std/fb.kr**: framebuffer primitives — `pixel`, `rect`, `line`, `blit`, `clear`, `rect_outline`.
- **std/font.kr**: complete 8x16 bitmap font covering ASCII 32–126 (95 characters).
- **std/widget.kr**: UI widgets — `panel`, `label`, `button`, `progress_bar`, `text_input`.

### Tested
- 13 new tests for uint16, atomics, volatile, and MSR/MRS (119 total).

## v2.3.0 - 2026-04-05

### Fixed
- **kr runner execution**: `kr` runner now executes extracted binaries via `set_executable` + `exec_process`.
- **exec_process Windows crash**: `exec_process` uses `lpApplicationName` to avoid read-only string crash.
- **kr runner Windows args**: `kr` runner correctly parses Windows command line via `win_init_args_r`.
- **Android kr temp path**: Android `kr` uses `/data/local/tmp/.kr-exec` (no `/tmp` on Android).
- **exec_process argv**: `exec_process` passes proper `argv` array to `execve`.

### Changed
- **Release workflow**: add macOS and Android binaries to release workflow (17 total assets).

## v2.2.0 - 2026-04-03

### Added
- **Android ARM64 support**: PIE ELF format for Android `aarch64` targets.
- **Semantic analysis pass**: argument count checking, missing return detection, duplicate function detection.

### Fixed
- **ARM64 codegen fixes**: SP encoding, large stack frames, and bit operation correctness.
- **Windows PE fixes**: stack alignment and digit buffer overflow.

## v2.1.1 - 2026-04-02

### Added
- **Universal fat binary**: `.krbo` now contains 7 slices — Linux ELF (x86_64 + arm64), Windows PE (x86_64 + arm64), macOS Mach-O (x86_64 + arm64). One binary runs everywhere.
- **BCJ compression filters**: x86_64 and AArch64 Branch/Call/Jump filters normalize instruction offsets before LZ4 compression, yielding ~9% better compression on real binaries.
- **Loader-resolved IAT**: Windows PE binaries have all 13 API imports (ExitProcess, GetStdHandle, WriteFile, CreateFileA, ReadFile, CloseHandle, VirtualAlloc, GetCommandLineA, GetFileSizeEx, CreateProcessA, WaitForSingleObject, GetExitCodeProcess, GetModuleFileNameA) resolved directly by the Windows PE loader at startup — no runtime GetProcAddress resolver needed.
- **Windows `kr` runner**: `kr.exe` extracts and executes Windows PE slices from `.krbo` fat binaries using CreateProcessA.
- **Windows stdlib support**: `install.ps1` downloads the standard library to `%LOCALAPPDATA%\KernRift\std\`. The compiler discovers stdlib via `GetModuleFileNameA` relative to its own path.
- **New built-in functions**: `get_target_os()`, `get_arch_id()`, `exec_process(path)`, `get_module_path(buf, size)` — compile-time platform detection and cross-platform process execution.
- **VS Code file icon**: `.kr` files show a minimal blue cracked K icon (auto-enabled via `languages[].icon`, no theme selection needed).
- **macOS Mach-O in fat binary**: Cross-compiled from Linux using existing `--emit=macho` support, tested via GitHub Actions macOS runners.

### Fixed
- **Undeclared identifier detection**: Using an undeclared variable (e.g., `cli_argv` without `static` declaration) now produces `error: use of undeclared identifier 'cli_argv'` at compile time instead of silently compiling to a null pointer dereference (SIGSEGV at runtime).
- **Fat binary buffer overflow**: Output buffer was 1MB but 6-slice compressed data exceeded it, causing silent truncation. Increased to 4MB.
- **Cross-platform CI tests**: `|| true` in test harness was swallowing exit codes, making all non-zero exit tests appear to fail on macOS.
- **Windows install script**: `install.ps1` now downloads from GitHub releases (was looking for local `dist/` binary that doesn't exist).
- **Release CI**: Bootstraps from previous release binary instead of outdated Rust `kernriftc`.

## Unreleased

### Added
- **Kernel-first features**: 10 features for bare-metal and kernel development:
  - **Inline assembly**: `asm("nop")` and `asm { "cli"; "sti" }` with x86_64 privileged instructions (cr0/cr3/cr4, lgdt, lidt, invlpg, cpuid, wrmsr, rdmsr, in/out, swapgs, iretq) and ARM64 system instructions (wfi, wfe, sev, isb, dsb, dmb, svc, eret). Raw hex byte emission for arbitrary encodings.
  - **@naked functions**: No prologue/epilogue — pure assembly bodies for ISR entry points.
  - **@noreturn functions**: Marks diverging functions (epilogue omitted).
  - **@packed structs**: No alignment padding between fields.
  - **@section attribute**: Place functions in specific linker sections (stored for future linker script support).
  - **Signed comparisons**: `signed_lt()`, `signed_gt()`, `signed_le()`, `signed_ge()` builtins using x86 setl/setg and ARM64 CSET LT/GT.
  - **Bitfield operations**: `bit_get()`, `bit_set()`, `bit_clear()`, `bit_range()`, `bit_insert()` builtins for hardware register manipulation.
  - **Volatile blocks**: `volatile { ... }` as explicit MMIO intent (same codegen as unsafe, forward-compatible with future optimizer).
  - **Stack size warnings**: Compile-time warning when a function's stack frame exceeds 4096 bytes.
  - **Freestanding mode**: `--freestanding` flag disables _start trampoline, auto-exit, and OS-specific syscall wrappers.
- **Self-contained toolchain**: `kernriftc` now produces native executables for all major platforms without any external tools.
  - Native ELF executable writer (`--emit=elfexe`) — no longer needs `ld`.
  - Native `.a` archive writer (`--emit=staticlib`) — no longer needs `ar`.
  - PE32+ executable writer for Windows x86_64 and AArch64 (with import directories).
  - Mach-O executable writer for macOS x86_64 and AArch64 (with dyld/libSystem linkage).
  - Native host executable emitter (`--emit=hostexe`) — no longer needs `cc`/`gcc`/`clang` on any platform.
- **6 platform runtime blobs**: hand-assembled machine code for Linux/macOS/Windows x86_64 and AArch64. Implements `_start`, `__kr_exit`, `__kr_write`, `__kr_mmap_alloc`, `__kr_alloc`, `__kr_dealloc`, `__kr_getenv`, `__kr_exec`, `__kr_str_copy`, `__kr_str_cat`, `__kr_str_len`.
- **Port I/O intrinsics**: `inb(port)`, `outb(port, val)`, `inw`, `outw`, `ind`, `outd` — x86_64 built-in intrinsics that emit native `IN`/`OUT` instructions. AArch64 produces a clear compile-time error (ARM has no port-mapped I/O).
- **`@syscall(nr, args...)` intrinsic**: generic syscall for Linux/macOS on both x86_64 and AArch64.
- **9 built-in host functions**: `write`, `alloc`, `dealloc`, `getenv`, `exec`, `exit`, `str_copy`, `str_cat`, `str_len` — available in `@ctx(host)` code without `extern fn` declarations, mapped to `__kr_*` runtime symbols.
- **Slice indexing**: `buf[i]` read and `buf[i] = val` write syntax for array element access.
- **KrboFat v2**: format version bumped with `runtime_offset`/`runtime_len` fields per entry.

### Changed
- **ApexRift drivers migrated**: all 7 driver `.kr` files now use built-in `inb`/`outb` intrinsics instead of `extern fn aos_inb/outb`.
- **`build.kr` migrated**: build script uses built-in host functions (`write`, `getenv`, `exec`, `str_len`, `exit`) instead of libc externs (`puts`, `system`, `getenv`, `exit`).
- Relaxed canonical-exec validation: general-purpose code with multiple variables and computed returns now compiles without rejection.

## v1.0.0 - 2026-03-24

### Added
- AArch64 (ARM64) backend: `aarch64-sysv` (Linux), `aarch64-macho` (macOS), `aarch64-win` (Windows).
- `KRBOFAT` fat binary container: 8-byte magic `KRBOFAT\0`, LZ4-compressed per-arch slices, fat-first detection (checked before single-arch `KRBO` magic).
- Default `kernriftc <file.kr>` output is now a fat binary (`.krbo`) containing x86_64 and ARM64 slices.
- `--arch x86_64|arm64|aarch64` flag: routes compilation to the specified target; output is still a fat binary. `aarch64` is an accepted alias for `arm64`.
- `--emit=krbofat` explicit fat binary emit mode (equivalent to default compile).
- `--emit=krboexe` for single-arch x86_64 KRBO (unchanged from prior behavior when requesting single-arch output explicitly).
- Dual-file output for `--emit=elfobj`, `--emit=asm`, `--emit=staticlib` without `--arch`.
- `kernrift` runner: fat-first detection reads 8-byte magic before the 4-byte single-arch check; extracts host-arch slice and executes it.
- ARM64 I-cache flush: `kernrift` flushes the instruction cache after writing ARM64 code to executable memory (required for AArch64 coherence on all Linux/macOS ARM64 hosts).
- New `krir` crate constants and APIs: `KRBO_ARCH_AARCH64` (`0x02`), `KRBO_FAT_MAGIC`, `KRBO_FAT_VERSION`, `KRBO_FAT_ARCH_X86_64`, `KRBO_FAT_ARCH_AARCH64`, `KRBO_FAT_COMPRESSION_NONE`, `KRBO_FAT_COMPRESSION_LZ4`, `emit_aarch64_executable_bytes()`, `emit_aarch64_elf_object_bytes()`, `emit_aarch64_macho_object_bytes()`, `emit_aarch64_coff_object_bytes()`, `emit_krbofat_bytes()`, `parse_krbofat_slice()`.
- New `krir` types: `TargetArch::AArch64`, `TargetAbi::Aapcs64`/`Aapcs64Win`, `BackendTargetId::Aarch64Sysv`/`Aarch64MachO`/`Aarch64Win`, `AArch64IntegerRegister` (X0–X15, X19–X30, Sp, Xzr; X16/X17/X18 excluded), `lower_executable_krir_to_aarch64_asm()`.
- New `kernriftc` CLI: `BackendArtifactKind::KrboFat` ("krbofat").
- New spec docs in `docs/spec/`: `aarch64-asm-linear-subset-v0.1.md`, `aarch64-object-linear-subset-v0.1.md`, `backend-target-model-aarch64-sysv-v0.1.md`, `backend-target-model-aarch64-macho-v0.1.md`, `backend-target-model-aarch64-win-v0.1.md`, `krbofat-container-v0.1.md`.

### Fixed
- `kernrift` runner: `map_uart_buffer` falls back to a kernel-chosen address when `mmap(MAP_FIXED)` at `0x10000000` is rejected (macOS CI ARM64 and Windows CI return `ENOMEM`/`null` for fixed mappings); programs with no MMIO (e.g. `examples/smoke_noop.kr`) are unaffected.
- `kernrift` runner (Unix): `map_executable` now maps `PROT_READ|PROT_WRITE` first, copies code, then `mprotect`s to `PROT_READ|PROT_EXEC`; avoids rejection of `PROT_WRITE|PROT_EXEC` on macOS CI (W^X enforcement).
- `kernrift` runner (Windows): `map_executable` calls `FlushInstructionCache` after writing JIT code; fixes SIGILL (exit 132) on Windows ARM64 where the I-cache and D-cache are incoherent.
- `krir` tests: 9 ELF-link tests now compile-gated with `#[cfg(all(unix, target_arch = "x86_64"))]`; `ld` on Windows emits PE (MZ magic), not ELF — those tests no longer run on Windows.

### Platform notes
- **macOS x86_64**: CI builds and ships the binary but execution on Intel Mac hardware has not been independently verified. Use with caution and report any issues.

### Tested
- `examples/smoke_noop.kr` compiled on Pi 400 (aarch64 Linux), fat binary pulled and run on x86_64 — exit 0.
- `examples/smoke_noop.kr` compiled on x86_64, fat binary run on Pi 400 — exit 0.

## v0.3.1 - 2026-03-23

### Added
- `kernrift` split into its own crate so `cargo install` tracks both binaries independently.
- `elfexe` emit target: `kernriftc --emit=elfexe` links an ELF ET_EXEC binary using `ld.lld`/`ld`.
- Dead function elimination pass: strips functions unreachable from `@export`/`@ctx(boot)`.
- Link-time lock graph merge: `kernriftc link` detects cross-module lock-order cycles.
- `kernriftc lc` alias: short form for `kernriftc living-compiler` (alias kept).
- Three new living-compiler patterns: `irq_raw_mmio`, `high_lock_depth`, `mmio_without_lock`.
- `lc --ci`: exit 1 if any suggestion fitness ≥ 50 (override with `--min-fitness N`).
- `lc --diff <file>`: show only new/worsened suggestions vs git HEAD.
- `lc --diff <before> <after>`: two-file local diff, no git dependency.
- `lc --fix --dry-run`: preview tail-call fixes as a unified diff.
- `lc --fix --write`: apply tail-call fixes in place, atomically.

### Improved
- **Syntax error messages** — all TokParser diagnostics now show human-readable token names
  instead of Rust debug format (e.g. `got '{'` instead of `got LBrace`).
  Specific improvements:
  - Missing return type after `->`: suggests valid types and `-> u64` example.
  - `if` without a condition: points at the `{` and suggests a boolean expression.
  - `let` keyword: directs to typed declaration syntax (`u64 x = ...`).
  - Undeclared variable assignment: names the variable and suggests declaration syntax.
  - Duplicate symbol: includes source location in the error.
  - Missing comma between call arguments: flags the unexpected token.
  - `mmio`/`mmio_reg` inside a function body: reports module-scope restriction.
  - `expect_kind` and all inner parser helpers use readable token names.
- `token_kind_to_str` is now exhaustive — every `TokenKind` variant maps to a display string.

## v0.2.10 - 2026-02-27

### Changed
- KRIR v0.1 acceptance script added: `tools/acceptance/krir_v0_1.sh`.
- Verify-report schema documentation tightened and strictness negative tests added for unknown keys/invalid enum values.
- KRIR spec updated with explicit verify-report ABI strictness table.

### Notes
- Product-only release.
- No infra/release workflow changes.
- `v0.2.9` remains frozen.

## v0.2.9 - 2026-02-27

### Changed
- KRIR v0.1: added schema-validated verify report ABI v1 (`docs/schemas/kernrift_verify_report_v1.schema.json`).
- `verify --report` now validates emitted report JSON against embedded schema with deterministic canonicalization.
- Expanded golden matrix for verify/report edge cases:
  - invalid UTF-8 contracts
  - schema-invalid contracts
  - signature mismatch
  - invalid signature/public key parsing
  - report overwrite refusal
- Aligned verify report output writing to guarded safe-write behavior (no overwrite + staged write flow).

### Notes
- User-visible product update: verify report format and coverage are now regression-locked in golden tests.

## v0.2.8 - 2026-02-23

### Changed
- Infra-only: release pipeline now signs/verifies archives only (`.tar.gz`, `.zip`).
- `.sha256` files remain unsigned convenience artifacts.

### Notes
- No compiler behavior changes vs v0.2.7.

## v0.2.7 - 2026-02-23

### Changed
- Fixed Windows cosign self-verification identity regex in release workflow.

### Notes
- `v0.2.6` introduced portable Linux checksums + signature self-verify, but release failed on Windows identity regex mismatch; use `v0.2.7`.

## v0.2.6 - 2026-02-23

### Changed
- Linux release checksum files now use archive basenames (portable `sha256sum -c` outside CI workspace layout).
- Release pipeline now self-verifies cosign signatures/certificates before uploading artifacts.

### Notes
- Infra-only release: no compiler behavior changes vs v0.2.5.

## v0.2.5 - 2026-02-23

### Changed
- Added `kernriftc --version` / `kernriftc -V` output (`kernriftc <semver>`) for release automation checks.

### Notes
- `v0.2.4` introduced release gating/signing workflow changes but failed release execution due missing CLI `--version`; use `v0.2.5`.

## v0.2.4 - 2026-02-23

### Changed
- Release pipeline now runs `fmt`/`clippy`/`test` gates before packaging artifacts.
- Release pipeline now signs artifacts with cosign keyless and publishes `.sig` + `.cert` files.
- Release build uses `--locked` and enforces tag/version match (`vX.Y.Z` == `kernriftc --version`).

### Notes
- This release is product-aligned and supersedes infra-only release tags (`v0.2.1`, `v0.2.2`).

## v0.2.3 - 2026-02-23

### Changed
- Infra: CI guards + release automation; no compiler behavior changes since v0.2.0.
- Versioning policy: tags/releases now track `kernriftc --version` (product-aligned).

### Notes
- v0.2.1 and v0.2.2 were infra-only tags; v0.2.3 is the aligned product tag.

## v0.2.0 - 2026-02-22

### Added
- Integrated policy gate in `check`:
  - `kernriftc check --policy <policy.toml> <file.kr>`
  - `kernriftc check --policy <policy.toml> --contracts-out <contracts.json> <file.kr>`
- Policy evaluator command:
  - `kernriftc policy --policy <policy.toml> --contracts <contracts.json>`
- Canonical contracts artifact outputs from `check`:
  - `--contracts-out`, `--hash-out`, `--sign-ed25519`, `--sig-out`
- Artifact verification command:
  - `kernriftc verify --contracts <contracts.json> --hash <contracts.sha256> [--sig <contracts.sig> --pubkey <pubkey.hex>]`

### Changed
- Policy diagnostics are now deterministic and code-prefixed:
  - `policy: <CODE>: <message>`
- Policy `max_lock_depth` is evaluated from `report.max_lock_depth`.
- Exit code split is enforced:
  - `0` success
  - `1` policy/verification deny
  - `2` invalid input/config/schema/decode/tooling errors

### Safety Hardening
- Embedded contracts schema is used for validation (distro-safe, no repo path dependency).
- `check` refuses overwriting existing output files.
- Output writes use staged temp files before commit.
- `verify` now requires UTF-8 contracts content and schema/version-valid contracts payload (not only hash/signature match).
