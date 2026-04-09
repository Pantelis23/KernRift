# KernRift

**KernRift is a bare-metal systems programming language and compiler created by Pantelis Christou.**

A self-hosted systems language compiler for kernel-first development. KernRift compiles itself — no Rust, no C, no external toolchain. It produces native executables for x86_64 and AArch64 on Linux, Windows, macOS, and Android, with BCJ+LZ4-compressed fat binaries as the default output (7 platform slices per `.krbo`). The `kr` runner executes `.krbo` fat binaries on any supported platform. The compiler self-hosts on 5 platforms including phones (verified on Samsung Galaxy Z Fold 5 via Termux).

## Features

- **Self-hosting** — the compiler compiles itself to a fixed point
- **Cross-platform** — Linux, Windows, macOS, and Android from a single source (x86_64 + ARM64)
- **Fat binaries** — default output is KrboFat (7 platform slices, BCJ+LZ4-compressed)
- **Zero dependencies** — static executables, no libc, no linker
- **Kernel-first** — inline assembly, `@naked` functions, `@packed` structs, signed comparisons, volatile memory, bitfield ops, `--freestanding` mode
- **Kernel safety** — context checks, effect tracking, lock graphs, capabilities, undeclared identifier detection
- **Volatile blocks** — `volatile { ... }` emits memory barriers (`mfence` on x86_64, `DMB ISH` on ARM64)
- **Atomic operations** — `atomic_load`, `atomic_store`, `atomic_cas`, `atomic_add`, `atomic_sub`, `atomic_and`, `atomic_or`, `atomic_xor`
- **Pointer cast ops** — uint16/uint32/uint64 and int16/int32/int64 pointer operations in `unsafe`/`volatile` blocks
- **Assembly listing** — `--emit=asm` produces a disassembled listing with function labels
- **ARM64 system registers** — MSR/MRS access in inline asm (20+ registers including SCTLR_EL1, VBAR_EL1, etc.)
- **Builtin validation** — argument count validation for builtins in the semantic analyzer
- **Living compiler** — pattern detection, fitness scoring, auto-fix suggestions
- **Cross-compilation** — compile for any target from any host

## Quickstart

```bash
# Install (gets krc compiler, kr runner, and stdlib)
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh

# Compile to fat binary (default: 7 platform slices, BCJ+LZ4-compressed)
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

### Self-compilation times

| Platform | CPU | Time |
|----------|-----|------|
| Linux x86_64 | AMD Ryzen 9 7900X | 55ms |
| Linux ARM64 | ARM Cortex-A72 (Pi 400) | 635ms |
| Windows 11 x86_64 | Intel Core Ultra 9 275HX | 66ms |
| Windows 11 ARM64 | GitHub Actions runner | bootstrap verified |
| Android ARM64 | Snapdragon 8 Gen 2 (Z Fold 5) | self-compile verified |

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
curl -sSLO https://github.com/Pantelis23/KernRift/releases/latest/download/kernrift_2.5.0_amd64.deb
sudo dpkg -i kernrift_2.5.0_amd64.deb
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
    uint64 x
    uint64 y
}

fn Point.sum(Point self) -> uint64 {
    return self.x + self.y
}

type Size = uint64

fn fib(uint64 n) -> uint64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}

static uint64 counter = 0

fn main() {
    Point p
    p.x = fib(10)
    p.y = 42

    uint64 buf = alloc(64)
    uint64 s = int_to_str(p.sum(), buf)
    println(s)

    match p.x {
        55 => { exit(p.sum()) }
    }
    exit(0)
}
```

Types: `uint8/16/32/64`, `int8/16/32/64`, `bool` (`true`/`false`), `char`, structs, enums, arrays. Control: `if/else`, `while`, `for..in`, `break/continue`, `match`. Functions up to 8 args with method syntax (`fn Struct.method`). Imports (`import "file.kr"`) with recursive dependency resolution and stdlib search paths, type aliases (`type Size = uint64`), nested struct access (`a.b.c`). Unsafe and volatile pointer access for kernel memory operations.

## Kernel Features

KernRift is designed for kernel and driver development:

```kr
// Inline assembly — emit raw machine instructions
@naked fn isr_entry() {
    asm { "cli"; "0x48 0x89 0xE5" }   // raw hex bytes also supported
    asm("iretq")
}

// Signed comparisons for kernel arithmetic
if signed_lt(offset, 0) { panic() }

// Bitfield operations for hardware registers
uint64 flags = bit_range(cr0, 0, 16)    // extract bits 0-15
flags = bit_set(flags, 31)               // set bit 31
cr0 = bit_insert(cr0, 0, 16, flags)     // insert back

// Volatile memory access (same as unsafe, explicit intent)
volatile { *(mmio_addr as uint32) -> status }

// Freestanding mode — no _start, no auto-exit, no syscalls
// krc --freestanding kernel.kr -o kernel.elf
```

Annotations: `@noreturn`, `@naked` (no prologue/epilogue), `@packed` (no padding), `@section(".text.init")`. Stack frames >4KB emit a compile-time warning.

## Built-in Functions

These are compiler intrinsics — no import needed, available on all platforms:

| Category | Functions |
|----------|-----------|
| Core | `alloc(size)`, `dealloc(ptr, size)`, `exit(code)`, `print(arg)`, `println(arg)` |
| I/O | `write(fd, buf, len)`, `file_open(path, flags)`, `file_read(fd, buf, len)`, `file_write(fd, buf, len)`, `file_close(fd)`, `file_size(fd)` |
| Memory | `memcpy(dst, src, len)`, `memset(dst, val, len)`, `str_len(s)`, `str_eq(a, b)` |
| Signed cmp | `signed_lt(a, b)`, `signed_gt(a, b)`, `signed_le(a, b)`, `signed_ge(a, b)` |
| Bitfield | `bit_get(v, n)`, `bit_set(v, n)`, `bit_clear(v, n)`, `bit_range(v, lo, hi)`, `bit_insert(v, lo, hi, bits)` |
| Atomic | `atomic_load(ptr)`, `atomic_store(ptr, val)`, `atomic_cas(ptr, exp, des)`, `atomic_add(ptr, val)`, `atomic_sub(ptr, val)`, `atomic_and(ptr, val)`, `atomic_or(ptr, val)`, `atomic_xor(ptr, val)` |
| Meta | `fn_addr(name)`, `call_ptr(addr, ...)`, `get_module_path(buf, size)`, `exec_process(path)`, `set_executable(path)`, `get_target_os()`, `get_arch_id()`, `fmt_uint(buf, val)` |

## Standard Library

16 modules (~2500+ lines) in `std/`:

| Module | Functions |
|--------|-----------|
| `std/string.kr` | `str_cat`, `str_copy`, `str_starts`, `str_ends`, `str_find_byte`, `str_contains`, `str_sub`, `str_at`, `str_to_int`, `int_to_str`, `str_repeat`, `str_trim` |
| `std/io.kr` | `read_file`, `write_file`, `append_file`, `read_line`, `print_kv`, `print_indent` |
| `std/math.kr` | `min`, `max`, `abs`, `clamp`, `pow`, `sqrt_int`, `gcd`, `is_prime` |
| `std/fmt.kr` | `fmt_hex`, `fmt_bin`, `pad_left`, `pad_right` |
| `std/mem.kr` | `realloc`, `memcmp`, `memzero`, `arena_init`, `arena_alloc`, `arena_reset` |
| `std/vec.kr` | `vec_new`, `push`, `get`, `set`, `pop`, `remove`, `contains` |
| `std/map.kr` | `map_new`, `set`, `get`, `has` |
| `std/color.kr` | Color utilities: `rgb`, `rgba`, `alpha_blend` |
| `std/fixedpoint.kr` | 16.16 fixed-point math |
| `std/memfast.kr` | Fast block memory ops |
| `std/fb.kr` | Framebuffer primitives |
| `std/font.kr` | 8x16 bitmap font renderer |
| `std/widget.kr` | UI widgets: panel, label, button, progress bar, text field |
| `std/time.kr` | Clock access: `clock_gettime`, `nanosleep` |
| `std/log.kr` | Structured logging with levels |
| `std/net.kr` | Raw socket operations |

Import with `import "std/string.kr"` etc. The compiler searches `~/.local/share/kernrift/` automatically.

## Editor Support

A VS Code extension (v0.2.3) is available on the VS Code Marketplace:

- Syntax highlighting (TextMate grammar)
- LSP server with diagnostics (`krc check`), completions, hover docs, and go-to-definition

## Architecture

15,500+ lines of KernRift across 15 source files + 16 stdlib modules (~2500+ lines). Self-compiles to a 383 KB native binary in 55ms, or a 2.6 MB universal fat binary (7 slices) in ~280ms (AMD Ryzen 9 7900X). 125 tests, bootstrap fixed point verified on 5 platforms (Linux x86_64, Linux ARM64, Windows x86_64, Windows ARM64, Android ARM64).

| File | Purpose |
|------|---------|
| `lexer.kr` | Tokenizer (90+ kinds) |
| `parser.kr` | Recursive descent + Pratt precedence |
| `codegen.kr` | x86_64 code generation |
| `codegen_aarch64.kr` | AArch64 code generation |
| `analysis.kr` | Safety passes (incl. undeclared identifier detection) |
| `living.kr` | Pattern detection + fitness |
| `bcj.kr` | BCJ filters (x86_64 + AArch64) for compression |
| `format_*.kr` | ELF, Mach-O, PE, AR, KRBO, KrboFat |
| `std/*.kr` | Standard library (16 modules, ~2500+ lines) |

## Bootstrap

```
kernriftc (bootstrap) → krc → krc2 → krc3 → krc4
                               krc3 == krc4 ✓ (fixed point)
```

The [bootstrap compiler](https://github.com/Pantelis23/KernRift-bootstrap) is only needed once.

## Platforms

| Platform | Compile | Run | Self-host | File I/O | Bootstrap |
|----------|---------|-----|-----------|----------|-----------|
| Linux x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point |
| Linux ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ fixed point |
| Windows x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ chain verified |
| Windows ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ krc3==krc4 |
| Android ARM64 | ✅ | ✅ | ✅ | ✅ | ✅ self-compiled on phone |
| macOS x86_64 | ✅ | ✅ | ✅ | ✅ | — |
| macOS ARM64 | ✅ | WIP | — | — | — |

## License

MIT — see [LICENSE](LICENSE).
