# Getting Started

**KernRift is a bare-metal systems programming language and compiler created by Pantelis Christou.**

## Install

### Linux / macOS

```bash
curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh

# Or download directly (x86_64)
curl -L -o krc https://github.com/Pantelis23/KernRift/releases/latest/download/krc-linux-x86_64
curl -L -o kr  https://github.com/Pantelis23/KernRift/releases/latest/download/kr
chmod +x krc kr
sudo mv krc kr /usr/local/bin/
```

This installs `krc` and `kr` to `~/.local/bin/` and the standard library to `~/.local/share/kernrift/`.

### Windows

```powershell
irm https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.ps1 | iex
```

This installs `krc.exe` and `kr.exe` to `%LOCALAPPDATA%\KernRift\bin\` and the standard library to `%LOCALAPPDATA%\KernRift\share\`. The installer adds the bin directory to your `PATH` automatically.

## Your First Program

```kr
fn main() {
    uint64 msg = "Hello, World!\n"
    write(1, msg, 14)
    exit(0)
}
```

Save as `hello.kr` and compile:

```bash
krc hello.kr -o hello.krbo        # fat binary (6 slices, LZ4-compressed)
kr hello.krbo                     # run on any platform

krc --arch=x86_64 hello.kr -o hello   # native x86_64 ELF
./hello                               # run directly
```

## Language Basics

```kr
// Types
uint64 x = 42
uint32 y = 10
uint8 byte = 0xFF
bool ready = true
char ch = 'A'

// Type aliases
type Size = uint64
type Offset = uint64

// Control flow
if x > 10 {
    exit(1)
} else {
    exit(0)
}

while x > 0 {
    x -= 1
}

for i in 0..10 {
    // i goes from 0 to 9
}

// Match statement
match opcode {
    1 => { do_add() }
    2 => { do_sub() }
    3 => { do_mul() }
}

// Functions
fn add(uint64 a, uint64 b) -> uint64 {
    return a + b
}

// Structs
struct Point {
    uint64 x
    uint64 y
}

// Method syntax
fn Point.sum(Point self) -> uint64 {
    return self.x + self.y
}

// Nested struct access: a.b.c

// Enums
enum Color {
    Red = 0
    Green = 1
    Blue = 2
}

// Arrays
uint8[256] buffer
buffer[0] = 42

// Imports
import "utils.kr"

// Pointer operations (kernel memory access)
unsafe { *(addr as uint32) = value }
unsafe { *(addr as uint32) -> result }

// Static variables
static uint64 counter = 0
```

## Safety Analysis

```bash
krc check module.kr     # context, effect, lock, capability checks
krc lc module.kr        # living compiler patterns + fitness score
```

## Toolchain

| Tool | Purpose |
|------|---------|
| `krc` | Compiler — compiles `.kr` source to native ELF, PE, Mach-O, or `.krbo` fat binary |
| `kr` | Runner — executes `.krbo` fat binaries on any supported platform |
| `krc check` | Safety analysis — context, effect, lock graph, capability checks |
| `krc lc` | Living compiler — pattern detection and fitness scoring |

### Supported platforms

| Platform | Compile | Run | Self-host |
|----------|---------|-----|-----------|
| Linux x86_64 | Yes | Yes | Yes |
| Linux ARM64 | Yes | Yes | Yes |
| Windows x86_64 | Yes | Yes | Yes |
| Windows ARM64 | Yes | Yes | -- |
| macOS x86_64 | Yes | WIP | -- |
| macOS ARM64 | Yes | WIP | -- |

## Standard Library

KernRift ships with a standard library (`std/`) that provides common utilities. Import any module with `import "std/module.kr"`. The compiler automatically searches `~/.local/share/kernrift/` for stdlib files.

### Usage examples

```kr
import "std/string.kr"
import "std/vec.kr"
import "std/math.kr"

fn main() {
    // String operations
    uint64 buf = alloc(256)
    uint64 s = int_to_str(42, buf)
    println(s)

    // Dynamic arrays
    uint64 v = vec_new(8)
    push(v, 10)
    push(v, 20)
    push(v, 30)
    uint64 val = get(v, 1)  // 20

    // Math
    uint64 m = max(10, 20)  // 20
    uint64 g = gcd(48, 18)  // 6

    exit(0)
}
```

### Available modules

| Module | Key functions |
|--------|---------------|
| `std/string.kr` | `str_cat`, `str_copy`, `str_starts`, `str_ends`, `str_find_byte`, `str_contains`, `str_sub`, `str_at`, `str_to_int`, `int_to_str`, `str_repeat`, `str_trim` |
| `std/io.kr` | `read_file`, `write_file`, `append_file`, `read_line`, `print_kv`, `print_indent` |
| `std/math.kr` | `min`, `max`, `abs`, `clamp`, `pow`, `sqrt_int`, `gcd`, `is_prime` |
| `std/fmt.kr` | `fmt_hex`, `fmt_bin`, `pad_left`, `pad_right` |
| `std/mem.kr` | `realloc`, `memcmp`, `memzero`, `arena_init`, `arena_alloc`, `arena_reset` |
| `std/vec.kr` | `vec_new`, `push`, `get`, `set`, `pop`, `remove`, `contains` |
| `std/map.kr` | `map_new`, `set`, `get`, `has` |

## Editor Setup

### VS Code

Install the **KernRift** extension from the VS Code Marketplace (v0.2.3). It provides:

- **Syntax highlighting** via TextMate grammar
- **LSP server** with:
  - Diagnostics (powered by `krc check`)
  - Completions for keywords, builtins, and imported symbols
  - Hover documentation
  - Go-to-definition

The extension activates automatically for `.kr` files.
