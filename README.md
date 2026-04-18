# KernRift

**KernRift is a bare-metal systems programming language and compiler created by Pantelis Christou.**

A self-hosted systems language compiler for kernel-first development. KernRift compiles itself — no Rust, no C, no LLVM, no external toolchain. It produces native executables for x86_64 and AArch64 on Linux, Windows, macOS, and Android, with BCJ+LZ-Rift-compressed fat binaries as the default output (8 platform slices per `.krbo`). The `kr` runner executes `.krbo` fat binaries on any supported platform. The compiler self-hosts on all 8 targets and is verified via CI on every push. The compiler ships with an **SSA-based IR backend** with liveness analysis, graph-coloring register allocation, and a constant-folding / DCE / CSE optimizer, producing native machine code for all targets directly from the IR — no assembler in the loop.

**v2.8.8 highlights** (full details in [CHANGELOG.md](CHANGELOG.md)):

- **Android fat-binary runner** now prefers `memfd_create` + `execveat(AT_EMPTY_PATH)` so `kr program.krbo` runs on Termux without touching the filesystem — no `kr-exec` file, no SELinux file-label transition, no chmod. Falls back to the file-based path on pre-3.17 kernels.
- **IR `main()` now initializes `cli_envp`** — the IR backend previously left it at 0, forwarding `envp=NULL` to every child `exec_process()` spawned.
- **compile_fat memory regression fixed** — a `uint64[1]` slot declared inside LZ4's match loop was leaking ~8 B per iteration; a self-compile hit 18 GB peak RSS before this release. Now ~1 GB, safely inside GitHub's 16 GB runner budget.
- **Ten IR ARM64 miscompiles patched** — `str_eq` bytewise clobber, `atomic_*` OLD-vs-NEW return, `atomic_cas` branch offset, `memcmp`/struct-equality, `int_to_f32` / `f32_to_int` encoding, `println(0.0)` digit-count, `--debug` div-by-zero trap, and a handful of branch-offset imm19 `<<` bugs.
- **Shipped ARM64 krc binaries route through legacy codegen** (Makefile + release.yml), while user-invoked `--arch=arm64` still defaults to IR. A separate compile_fat-on-IR-ARM64 segfault is tracked for the next release.

## Features

- **Self-hosting** — the compiler compiles itself to a fixed point. No Rust, no C, no LLVM in the build.
- **SSA IR backend** — target-independent intermediate representation with liveness analysis and graph-coloring register allocation. Emits x86_64 and AArch64 machine code directly. `--legacy` falls back to the original direct codegen if needed.
- **Cross-platform** — Linux, Windows, macOS, Android on x86_64 and ARM64 from a single source tree.
- **Floating-point** — `f32` and `f64` types with full arithmetic, comparisons, conversions, and a math library (`sin`, `cos`, `exp`, `log`, `pow`, `sqrt`, `fmt_f64`). `f16` for storage. Hardware `sqrt`, software trig/exp/log.
- **Multi-return** — `return (a, b)` and `(u64 x, u64 y) = call()` for 2-tuple destructuring.
- **Inline asm I/O** — `asm { "rdtsc" } out(rax -> lo, rdx -> hi)` with in/out/clobbers clauses.
- **Fat binaries** — default output is a `.krbo` with 8 platform slices (BCJ+LZ-Rift compressed). The `kr` runner extracts and executes the right slice at startup.
- **Zero dependencies at runtime** — static executables, no libc, no dynamic linker.
- **Kernel-first primitives** — `device` blocks for typed MMIO, `load/store/vload/vstore` builtins for clean pointer access, inline assembly with a large instruction table, signed comparisons, bitfield ops, atomic operations, `--freestanding` mode.
- **Clean pointer syntax** — `store32(addr, val)` and `load64(addr)` instead of the verbose `unsafe { *(addr as uint32) = val }` form.
- **Slice parameters** — `fn foo([u8] data)` with `data.len` for buffer-processing functions.
- **Fixed arrays** — `u8[256] buf` locally, `static u8[4096] page` at module level, and `Point[10] pts` with `pts[i].field` syntax for struct arrays.
- **Volatile blocks** — `mfence` on x86_64, `DSB SY` on ARM64 — completion barrier, not just ordering.
- **ARM64 system registers** — MSR/MRS access in inline asm (20+ registers including SCTLR_EL1, VBAR_EL1, MPIDR_EL1).
- **Semantic analysis** — argument count checking, missing return detection, undeclared identifier detection.
- **`--emit=asm`** — disassembled listing with function labels.
- **Cross-compilation** — compile for any target from any host.

## Quickstart

```bash
# Install (gets krc compiler, kr runner, and stdlib)
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh

# Compile to fat binary (default: 8 platform slices, BCJ+LZ-Rift-compressed)
krc hello.kr -o hello.krbo

# Run on any platform
kr hello.krbo

# Single architecture — native ELF executable
krc --arch=x86_64 hello.kr -o hello
krc --arch=arm64 hello.kr -o hello

# Multi-file projects — imports resolved automatically
krc main.kr -o program    # main.kr can import "utils.kr", etc.

# Safety analysis
krc check module.kr

# Living compiler
krc lc program.kr
```

### Self-compilation (v2.8.8, ~204K tokens, ~128K AST nodes, ~1.5 MB source)

All 8 targets self-compile. CI verifies bootstrap fixed point (krc3 == krc4) and runs 335 tests on every push. Numbers below are on an AMD Ryzen 9 7900X — see [`benchmarks/BENCHMARKS.md`](benchmarks/BENCHMARKS.md) for the complete run including gcc / rustc comparisons.

| Target | Legacy codegen | IR codegen | IR is… |
|--------|---------------:|-----------:|-------:|
| linux   x86_64 ELF    |  249 ms / 1.12 MB | 1 600 ms / 1.50 MB | **+34 %** size |
| linux   arm64  ELF    |  244 ms / 0.97 MB | 1 595 ms / 0.85 MB | **−12 %** size |
| windows x86_64 PE     |  245 ms / 1.40 MB | 1 600 ms / 1.56 MB | +11 % size |
| windows arm64  PE     |  244 ms / 1.01 MB | 1 606 ms / 0.91 MB | −10 % size |
| macOS   x86_64 Mach-O |  249 ms / 1.13 MB | 1 605 ms / 1.51 MB | +33 % size |
| macOS   arm64  Mach-O |  251 ms / 1.02 MB | 1 602 ms / 0.92 MB | −10 % size |
| android x86_64 ELF    |  249 ms / 1.25 MB | 1 604 ms / 1.57 MB | +26 % size |
| android arm64  ELF    |  244 ms / 1.05 MB | 1 597 ms / 0.98 MB |  −6 % size |
| **Fat binary (all 8)**| **1 990 ms / 3.88 MB** | **1 982 ms / 3.87 MB** | same (hybrid default) |

**Current defaults**: `--arch=x86_64` stays on IR (optimizer wins matter more on real programs than the straight-line size regression); `--arch=arm64` uses IR (consistently 10 % smaller). Fat-binary builds (`krc program.kr`) route ARM64 slices through legacy because the IR ARM64 backend still miscompiles `compile_fat` itself — tracked as the next release milestone. `--legacy` and `--ir` flags force the respective backends.

## Install

**Linux / macOS** (installs `krc`, `kr`, and stdlib to `~/.local/`):
```bash
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh
```

**Homebrew** (macOS / Linux):
```bash
brew install kernrift
```

**Scoop** (Windows):
```powershell
scoop bucket add kernrift https://github.com/Pantelis23/KernRift
scoop install kernrift
```

**Winget** (Windows):
```powershell
winget install Pantelis23.KernRift
```

**Debian/Ubuntu** (.deb):
```bash
curl -sSLO https://github.com/Pantelis23/KernRift/releases/latest/download/kernrift_2.8.8_amd64.deb
sudo dpkg -i kernrift_2.8.8_amd64.deb
```

**AUR** (Arch Linux):
```bash
yay -S kernrift
```

**Windows** (PowerShell):
```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

**From source** (requires [bootstrap compiler](https://github.com/Pantelis23/KernRift-bootstrap)):
```bash
cargo install --git https://github.com/Pantelis23/KernRift-bootstrap kernriftc
make build && make install
```

This installs `krc` and `kr` to `~/.local/bin/` and the standard library to `~/.local/share/kernrift/`. On Windows, `install.ps1` installs `krc.exe` and `kr.exe` to `%LOCALAPPDATA%\KernRift\`.

## Language

```kr
import "std/string.kr"
import "std/io.kr"

struct Point {
    u64 x
    u64 y
}

fn Point.sum(Point self) -> u64 {
    return self.x + self.y
}

fn fib(u64 n) -> u64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    Point p
    p.x = fib(10)
    p.y = 42

    // int_to_str returns a pointer — use print_str, not println
    u64 s = int_to_str(p.sum())
    print_str("sum = ")
    println_str(s)

    exit(0)
}
```

```kr
import "std/math_float.kr"

fn main() {
    f64 x = int_to_f64(2)
    println_str(fmt_f64(sqrt(x), 6))  // "1.414213"

    (u64 q, u64 r) = divmod(17, 5)
    println(q)  // 3
    exit(0)
}

fn divmod(u64 a, u64 b) -> u64 {
    return (a / b, a % b)
}
```

Types: `u8/u16/u32/u64`, `i8/i16/i32/i64`, `f16/f32/f64` (long forms `uint8`..`int64` also work), structs, enums, fixed-size arrays, device blocks. Control: `if/else`, `while`, `for..in`, `break/continue`, `match`, recursion. Functions with method syntax (`fn Struct.method`), slice parameters (`fn foo([u8] data) { u64 n = data.len; ... }`), imports with recursive resolution.

## Kernel Features

KernRift is designed for kernel and driver development. The two most
important primitives:

```kr
// Typed MMIO register blocks — compile to volatile load/store with barriers
device UART0 at 0x3F201000 {
    Data at 0x00 : u32
    Flag at 0x18 : u32
    Ctrl at 0x30 : u32
}

fn putc(u8 c) {
    while (UART0.Flag & 0x20) != 0 { }
    UART0.Data = c
}

// Clean pointer builtins (no unsafe blocks required)
u32 status = vload32(0xFEE000B0)        // volatile load, with mfence / DSB SY
vstore32(0xFEE000B0, 0x1)                // volatile store, with barrier
store8(buf + offset, byte_value)         // plain store
u64 value = load64(addr)                 // plain load

// Inline assembly — raw instructions when you need them
@naked fn isr_entry() {
    asm { "cli"; "0x48 0x89 0xE5" }
    asm("iretq")
}

// Signed comparisons (default < > <= >= are unsigned)
if signed_lt(offset, 0) { panic() }

// Bitfield manipulation for hardware registers
u64 flags = bit_range(cr0, 0, 16)
cr0 = bit_insert(cr0, 0, 16, new_flags)

// Freestanding mode — no main trampoline, no auto-exit
// krc --freestanding kernel.kr -o kernel.elf
```

Annotations: `@export`, `@noreturn`, `@naked` (no prologue/epilogue), `@packed` (structs are already packed), `@section(".text.init")`. Stack frames >4KB emit a compile-time warning.

## Built-in Functions

Compiler intrinsics — no imports needed.

| Category | Functions |
|----------|-----------|
| Core | `alloc(size)`, `dealloc(ptr)`, `exit(code)` |
| Output | `print(literal_or_int)`, `println(literal_or_int)`, `print_str(s)`, `println_str(s)` — use `*_str` for string pointers in variables |
| I/O | `write(fd, buf, len)`, `file_open(path, flags)`, `file_read(fd, buf, len)`, `file_write(fd, buf, len)`, `file_close(fd)`, `file_size(fd)` |
| Memory | `memcpy(dst, src, len)`, `memset(dst, val, len)`, `str_len(s)`, `str_eq(a, b)` |
| Pointer load | `load8(addr)`, `load16(addr)`, `load32(addr)`, `load64(addr)` — zero-extended to `u64` |
| Pointer store | `store8(addr, v)`, `store16(addr, v)`, `store32(addr, v)`, `store64(addr, v)` |
| Volatile (MMIO) | `vload8/16/32/64(addr)`, `vstore8/16/32/64(addr, v)` — with memory barrier |
| Atomic | `atomic_load(ptr)`, `atomic_store(ptr, v)`, `atomic_cas(ptr, exp, des)`, `atomic_add/sub/and/or/xor(ptr, v)` |
| Bitfield | `bit_get(v, n)`, `bit_set(v, n)`, `bit_clear(v, n)`, `bit_range(v, start, width)`, `bit_insert(v, start, width, bits)` |
| Signed cmp | `signed_lt(a, b)`, `signed_gt(a, b)`, `signed_le(a, b)`, `signed_ge(a, b)` |
| Float | `int_to_f64(v)`, `f64_to_int(v)`, `int_to_f32(v)`, `f32_to_int(v)`, `f32_to_f64(v)`, `f64_to_f32(v)`, `sqrt(v)`, `fma_f64(a,b,c)` |
| Syscall | `syscall_raw(nr, a1, a2, a3, a4, a5, a6)` |
| Platform | `get_target_os()`, `get_arch_id()`, `exec_process(path)`, `set_executable(path)`, `get_module_path(buf, size)`, `fmt_uint(buf, val)` |
| Function ptrs | `fn_addr(name)`, `call_ptr(addr, ...)` |

## Standard Library

17 modules (~2500+ lines) in `std/`:

| Module | Functions |
|--------|-----------|
| `std/string.kr` | `str_cat`, `str_copy`, `str_starts`, `str_ends`, `str_find_byte`, `str_contains`, `str_sub`, `str_at`, `str_to_int`, `int_to_str`, `str_repeat`, `str_trim`, `str_index_of`, `str_compare`, `str_lower`, `str_upper`, `str_replace`, `str_split`, `str_join`, `str_to_float`, `str_from_float`, `str_from_bool`, `str_from_codepoint`, `utf8_decode_at`, `utf8_encode`, `utf8_lower_codepoint`, `utf8_upper_codepoint`, `utf8_is_combining`, `str_lower_utf8`, `str_upper_utf8`, `str_codepoint_count`, `str_grapheme_count`, `sb_new`, `sb_append_{str,int,hex,float,bool,byte,codepoint}`, `sb_finish`, `sb_free` |
| `std/io.kr` | `read_file`, `write_file`, `append_file`, `read_line`, `print_int`, `print_line`, `print_kv`, `print_indent`, `scan_int`, `scan_str` |
| `std/math.kr` | `min`, `max`, `abs`, `clamp`, `pow`, `sqrt_int`, `gcd`, `is_prime` |
| `std/fmt.kr` | `fmt_hex`, `fmt_bin`, `pad_left`, `pad_right` |
| `std/mem.kr` | `realloc`, `memcmp`, `memzero`, `arena_init`, `arena_alloc`, `arena_reset` |
| `std/vec.kr` | `vec_new`, `vec_push`, `vec_get`, `vec_set`, `vec_pop`, `vec_remove`, `vec_contains`, `vec_len`, `vec_cap`, `vec_last`, `vec_clear`, `vec_free` |
| `std/map.kr` | `map_new`, `map_set`, `map_get`, `map_has`, `map_len`, `map_keys`, `map_vals`, `map_free` |
| `std/color.kr` | Color utilities: `rgb`, `rgba`, `alpha_blend` |
| `std/fixedpoint.kr` | 16.16 fixed-point math |
| `std/memfast.kr` | Fast block memory ops |
| `std/fb.kr` | Framebuffer primitives |
| `std/font.kr` | 8x16 bitmap font renderer |
| `std/widget.kr` | UI widgets: panel, label, button, progress bar, text field |
| `std/time.kr` | `time_now`, `time_sleep_ns`, `time_sleep_ms`, `time_elapsed` |
| `std/log.kr` | `log_set_level`, `log_debug`, `log_info`, `log_warn`, `log_error`, `log_info_kv`, `log_error_int` |
| `std/math_float.kr` | `sqrt`, `sin`, `cos`, `tan`, `exp`, `log`, `pow`, `floor`, `ceil`, `abs_f`, `fmt_f64`, `fmt_f32`, `f64_pi`, `f64_e` |
| `std/net.kr` | `net_socket`, `net_bind`, `net_listen`, `net_accept`, `net_connect`, `net_send`, `net_recv`, `net_close`, `net_htons`, `net_addr_ipv4` |

Import with `import "std/string.kr"` etc. The compiler searches `~/.local/share/kernrift/` automatically.

## Editor Support

A VS Code extension (v0.2.3) is available on the VS Code Marketplace:

- Syntax highlighting (TextMate grammar)
- LSP server with diagnostics (`krc check`), completions, hover docs, and go-to-definition

## Examples

See the [`examples/`](examples/) directory for runnable programs covering every feature — pointers, slices, struct arrays, device blocks, recursion, stdin input, and more.

## Architecture

~40 000 lines of KernRift across 17 source files + 17 stdlib modules (204 K tokens, 128 K AST nodes on self-compile). Self-compiles to a 1.1 MB x86_64 native binary in ~249 ms (legacy) / ~1.6 s (IR), or an 8-slice fat binary (BCJ + LZ-Rift compression) in ~2 s on an AMD Ryzen 9 7900X. **335 tests** pass, bootstrap fixed point verified on all 8 targets — Linux, macOS, Windows, and Android on both x86_64 and ARM64. See [`benchmarks/BENCHMARKS.md`](benchmarks/BENCHMARKS.md) for micro-benchmarks vs gcc / rustc and peak-memory numbers.

| File | Purpose |
|------|---------|
| `lexer.kr` | Tokenizer (90+ kinds) |
| `parser.kr` | Recursive descent + Pratt precedence |
| `ir.kr` | SSA IR + x86_64 emitter (Linux / macOS / Windows / Android) |
| `ir_aarch64.kr` | AArch64 emitter fed from the same IR |
| `codegen.kr` | Legacy direct x86_64 codegen (`--legacy` fallback) |
| `codegen_aarch64.kr` | Legacy direct AArch64 codegen |
| `analysis.kr` | Safety passes (incl. undeclared identifier detection) |
| `living.kr` | Pattern detection + fitness |
| `bcj.kr` | BCJ filters (x86_64 + AArch64) for compression |
| `format_*.kr` | ELF, Mach-O, PE, AR, KRBO, KrboFat |
| `runner.kr` | `kr` — fat-binary slice extractor / launcher |
| `std/*.kr` | Standard library (17 modules, ~2500+ lines) |

## Bootstrap

```
released krc binary → krc (stage 1, from source)
krc → krc2 → krc3 → krc4
krc3 == krc4 ✓ (bit-identical fixed point)
```

A released `krc` binary compiles the current source into the next `krc`. No Rust, no C, no LLVM involved. CI verifies the fixed point on every push across all 8 platform targets.

## Platforms

| Platform | Compile | Run | Self-host | File I/O | Bootstrap |
|----------|---------|-----|-----------|----------|-----------|
| Linux x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point |
| Linux ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point (legacy-compiled krc) |
| macOS ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point (legacy-compiled krc) |
| macOS x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ (Rosetta) |
| Windows x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point |
| Windows ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ krc3==krc4 (legacy-compiled krc) |
| Android ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ self-compiled on phone (legacy-compiled krc) |
| Android x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ verified |

> **Note on ARM64 krc binaries:** The shipped `krc-*-arm64` executables use the legacy codegen (13 % larger than IR) as a workaround for an IR ARM64 miscompile of `compile_fat` itself. User programs compiled by those binaries still go through IR on ARM64 and get the smaller, optimized output. The underlying codegen bug is the next milestone.

## License

MIT — see [LICENSE](LICENSE).
