# KernRift

A self-hosted systems language compiler for kernel-first development. KernRift compiles itself — no Rust, no C, no external toolchain. It produces native executables for x86_64 and AArch64, with LZ4-compressed fat binaries as the default output. The `kr` runner executes `.krbo` fat binaries on any supported platform.

## Features

- **Self-hosting** — the compiler compiles itself to a fixed point
- **Dual architecture** — x86_64 and AArch64 from a single source
- **Fat binaries** — default output is KrboFat (both architectures, LZ4-compressed)
- **Zero dependencies** — static executables, no libc, no linker
- **Kernel safety** — context checks, effect tracking, lock graphs, capabilities
- **Living compiler** — pattern detection, fitness scoring, auto-fix suggestions
- **Cross-compilation** — compile for any target from any host

## Quickstart

```bash
# Install (gets krc compiler, kr runner, and stdlib)
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh

# Compile to fat binary (default: x86_64 + ARM64, LZ4-compressed)
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
| Linux x86_64 | AMD Ryzen 9 7900X | 20ms |
| Windows 11 x86_64 | Intel Core Ultra 9 275HX | 44ms |
| Linux ARM64 | ARM Cortex-A72 (Pi 400) | 192ms |

## Install

**Linux / macOS** (installs `krc`, `kr`, and stdlib to `~/.local/`):
```bash
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh
```

This installs `krc` and `kr` to `~/.local/bin/` and the standard library to `~/.local/share/kernrift/`.

**From source** (requires [bootstrap compiler](https://github.com/Pantelis23/KernRift-bootstrap)):
```bash
cargo install --git https://github.com/Pantelis23/KernRift-bootstrap kernriftc
make build && make install
```

**Windows:**
```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

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

Types: `uint8/16/32/64`, `int8/16/32/64`, `bool` (`true`/`false`), `char`, structs, enums, arrays. Control: `if/else`, `while`, `for..in`, `break/continue`, `match`. Functions up to 8 args with method syntax (`fn Struct.method`). Imports (`import "file.kr"`) with recursive dependency resolution and stdlib search paths, type aliases (`type Size = uint64`), nested struct access (`a.b.c`). Unsafe pointer access for kernel memory operations.

## Standard Library

7 modules (828 lines) in `std/`:

| Module | Functions |
|--------|-----------|
| `std/string.kr` | `str_cat`, `str_copy`, `str_starts`, `str_ends`, `str_find_byte`, `str_contains`, `str_sub`, `str_at`, `str_to_int`, `int_to_str`, `str_repeat`, `str_trim` |
| `std/io.kr` | `read_file`, `write_file`, `append_file`, `read_line`, `print_kv`, `print_indent` |
| `std/math.kr` | `min`, `max`, `abs`, `clamp`, `pow`, `sqrt_int`, `gcd`, `is_prime` |
| `std/fmt.kr` | `fmt_hex`, `fmt_bin`, `pad_left`, `pad_right` |
| `std/mem.kr` | `realloc`, `memcmp`, `memzero`, `arena_init`, `arena_alloc`, `arena_reset` |
| `std/vec.kr` | `vec_new`, `push`, `get`, `set`, `pop`, `remove`, `contains` |
| `std/map.kr` | `map_new`, `set`, `get`, `has` |

Import with `import "std/string.kr"` etc. The compiler searches `~/.local/share/kernrift/` automatically.

## Editor Support

A VS Code extension (v0.2.0) is available on the VS Code Marketplace:

- Syntax highlighting (TextMate grammar)
- LSP server with diagnostics (`krc check`), completions, hover docs, and go-to-definition

## Architecture

10,000+ lines of KernRift across 12 source files + 7 stdlib modules (828 lines). Self-compiles to a 255KB native binary in 20ms (AMD Ryzen 9 7900X). 70 tests, bootstrap fixed point verified on 3 platforms.

| File | Purpose |
|------|---------|
| `lexer.kr` | Tokenizer (90+ kinds) |
| `parser.kr` | Recursive descent + Pratt precedence |
| `codegen.kr` | x86_64 code generation |
| `codegen_aarch64.kr` | AArch64 code generation |
| `analysis.kr` | Safety passes |
| `living.kr` | Pattern detection + fitness |
| `format_*.kr` | ELF, Mach-O, PE, AR, KRBO, KrboFat |
| `std/*.kr` | Standard library (7 modules, 828 lines) |

## Bootstrap

```
kernriftc (bootstrap) → krc → krc2 → krc3 → krc4
                               krc3 == krc4 ✓ (fixed point)
```

The [bootstrap compiler](https://github.com/Pantelis23/KernRift-bootstrap) is only needed once.

## Platforms

| Platform | Compile | Run | Self-host |
|----------|---------|-----|-----------|
| Linux x86_64 | ✅ | ✅ | ✅ |
| Linux ARM64 | ✅ | ✅ | ✅ |
| Windows x86_64 | ✅ | ✅ | ✅ |
| Windows ARM64 | ✅ | ✅ | -- |
| macOS x86_64 | ✅ | WIP | -- |
| macOS ARM64 | ✅ | WIP | -- |

## License

MIT — see [LICENSE](LICENSE).
