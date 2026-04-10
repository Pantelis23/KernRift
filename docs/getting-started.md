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
krc hello.kr -o hello.krbo        # fat binary (7 slices, BCJ+LZ4-compressed)
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
| `krc fmt` | Auto-formatter — formats `.kr` source files |
| `krc lc` | Living compiler — pattern detection and fitness scoring |

### Supported platforms

| Platform | Compile | Run | Self-host |
|----------|---------|-----|-----------|
| Linux x86_64 | Yes | Yes | Yes |
| Linux ARM64 | Yes | Yes | Yes |
| Windows x86_64 | Yes | Yes | Yes |
| Windows ARM64 | Yes | Yes | Yes |
| Android ARM64 | Yes | Yes | Yes |
| macOS x86_64 | Yes | Yes | Yes |
| macOS ARM64 | Yes | Blocked | -- |

## Standard Library

KernRift ships with a standard library (`std/`) that provides common utilities. Import any module with `import "std/module.kr"`. The compiler automatically searches `~/.local/share/kernrift/` for stdlib files.

### Usage examples

```kr
import "std/string.kr"
import "std/vec.kr"
import "std/math.kr"
import "std/io.kr"

fn main() {
    // String operations
    uint64 s = int_to_str(42)
    println(s)

    // Dynamic arrays
    uint64 v = vec_new()
    vec_push(v, 10)
    vec_push(v, 20)
    vec_push(v, 30)
    uint64 val = vec_get(v, 1)  // 20

    // Math
    uint64 m = max(10, 20)  // 20
    uint64 g = gcd(48, 18)  // 6

    // Read input from stdin
    print("Enter a number: ")
    uint64 n = scan_int()
    println(n)

    exit(0)
}
```

### Available modules

| Module | Key functions |
|--------|---------------|
| `std/string.kr` | `str_cat`, `str_copy`, `str_starts`, `str_ends`, `str_find_byte`, `str_contains`, `str_sub`, `str_at`, `str_to_int`, `int_to_str`, `str_repeat`, `str_trim` |
| `std/io.kr` | `read_file`, `write_file`, `append_file`, `read_line`, `print_int`, `print_line`, `print_kv`, `print_indent`, `scan_int`, `scan_str` |
| `std/math.kr` | `min`, `max`, `abs`, `clamp`, `pow`, `sqrt_int`, `gcd`, `is_prime` |
| `std/fmt.kr` | `fmt_hex`, `fmt_bin`, `pad_left`, `pad_right` |
| `std/mem.kr` | `realloc`, `memcmp`, `memzero`, `arena_init`, `arena_alloc`, `arena_reset` |
| `std/vec.kr` | `vec_new`, `vec_push`, `vec_get`, `vec_set`, `vec_pop`, `vec_remove`, `vec_contains`, `vec_len`, `vec_cap`, `vec_last`, `vec_clear`, `vec_free` |
| `std/map.kr` | `map_new`, `map_set`, `map_get`, `map_has`, `map_len`, `map_keys`, `map_vals`, `map_free` |
| `std/color.kr` | `rgb`, `rgba`, `color_r`, `color_g`, `color_b`, `alpha_blend`, `color_lerp` |
| `std/fb.kr` | `fb_init(addr,w,h,stride,bpp)`, `fb_pixel`, `fb_rect`, `fb_fill`, `fb_clear`, `fb_hline`, `fb_vline`, `fb_line`, `fb_rect_outline`, `fb_blit` |
| `std/fixedpoint.kr` | `fp_from_int`, `fp_to_int`, `fp_add`, `fp_sub`, `fp_mul`, `fp_div`, `fp_sqrt` |
| `std/font.kr` | `font_init`, `fb_char`, `fb_text` |
| `std/memfast.kr` | `memcpy32`, `memcpy64`, `memset32`, `memset64` |
| `std/widget.kr` | `panel_new`, `panel_draw`, `label_new`, `button_new`, `progress_new`, `textfield_new` |
| `std/time.kr` | `time_now`, `time_sleep_ns`, `time_sleep_ms`, `time_elapsed` |
| `std/log.kr` | `log_set_level`, `log_debug`, `log_info`, `log_warn`, `log_error`, `log_info_kv`, `log_error_int` |
| `std/net.kr` | `net_socket`, `net_bind`, `net_listen`, `net_accept`, `net_connect`, `net_send`, `net_recv`, `net_close`, `net_htons`, `net_addr_ipv4` |

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
