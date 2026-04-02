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
# Install (gets both krc compiler and kr runner)
bash install.sh

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
| Linux x86_64 | AMD Ryzen 9 7900X | 16ms |
| Windows 11 x86_64 | Intel Core Ultra 9 275HX | 44ms |
| Linux ARM64 | ARM Cortex-A72 (Pi 400) | 192ms |

## Install

**Linux / macOS** (installs both `krc` and `kr`):
```bash
bash install.sh
```

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
import "utils.kr"

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

    match p.x {
        55 => { exit(p.sum()) }
    }
    exit(0)
}
```

Types: `uint8/16/32/64`, `int8/16/32/64`, `bool` (`true`/`false`), `char`, structs, enums, arrays. Control: `if/else`, `while`, `for..in`, `break/continue`, `match`. Functions up to 8 args with method syntax (`fn Struct.method`). Imports (`import "file.kr"`), type aliases (`type Size = uint64`), nested struct access (`a.b.c`). Unsafe pointer access for kernel memory operations.

## Architecture

8,753 lines, 12 source files with imports. Self-compiles to 234KB in 16ms (AMD Ryzen 9 7900X). 53 tests, bootstrap fixed point verified on 3 platforms.

| File | Purpose |
|------|---------|
| `lexer.kr` | Tokenizer (90+ kinds) |
| `parser.kr` | Recursive descent + Pratt precedence |
| `codegen.kr` | x86_64 code generation |
| `codegen_aarch64.kr` | AArch64 code generation |
| `analysis.kr` | Safety passes |
| `living.kr` | Pattern detection + fitness |
| `format_*.kr` | ELF, Mach-O, PE, AR, KRBO, KrboFat |

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
