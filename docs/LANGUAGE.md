# KernRift Language Reference

**KernRift** is a bare-metal systems programming language and compiler created
by Pantelis Christou. It compiles itself. It runs on Linux, Windows, macOS,
and Android across x86_64 and ARM64 without any C toolchain, runtime, or libc.

This document describes what the language actually is. Every feature listed
here is implemented in the compiler you just installed — if you hit
something that doesn't work, it's a bug, not a typo in the docs.

---

## Table of Contents

1. [File structure and comments](#1-file-structure-and-comments)
2. [Types](#2-types)
3. [Variables and assignment](#3-variables-and-assignment)
4. [Operators](#4-operators)
5. [Control flow](#5-control-flow)
6. [Functions](#6-functions)
7. [Structs, methods, and enums](#7-structs-methods-and-enums)
8. [Arrays](#8-arrays)
9. [Slice parameters](#9-slice-parameters)
10. [Static variables and constants](#10-static-variables-and-constants)
11. [Pointer operations](#11-pointer-operations)
12. [Volatile and atomic](#12-volatile-and-atomic)
13. [Device blocks (MMIO)](#13-device-blocks-mmio)
14. [Inline assembly](#14-inline-assembly)
15. [Imports](#15-imports)
16. [Built-in functions](#16-built-in-functions)
17. [Annotations](#17-annotations)
18. [Compiler CLI](#18-compiler-cli)
19. [Living compiler](#19-living-compiler)
20. [Language profiles (#lang)](#20-language-profiles-lang)
21. [Freestanding mode](#21-freestanding-mode)
22. [Binary formats](#22-binary-formats)

---

## 1. File structure and comments

KernRift source files use the `.kr` extension. One file is one module. A
program starts execution at `fn main()` (unless you pass `--freestanding`).

```kr
// Line comment

/* Block comment.
   Can span multiple lines. */

fn main() {
    println("Hello, KernRift!")
    exit(0)
}
```

Statements do not require trailing semicolons. Semicolons are accepted and
ignored — useful when you want to write multiple statements on one line.

---

## 2. Types

### Scalar types

| Type      | Width | Alias | Notes                         |
|-----------|-------|-------|-------------------------------|
| `uint8`   | 1 B   | `u8`  | Unsigned byte                 |
| `uint16`  | 2 B   | `u16` | Unsigned 16-bit               |
| `uint32`  | 4 B   | `u32` | Unsigned 32-bit               |
| `uint64`  | 8 B   | `u64` | Unsigned 64-bit, pointer-sized |
| `int8`    | 1 B   | `i8`  | Signed byte                   |
| `int16`   | 2 B   | `i16` | Signed 16-bit                 |
| `int32`   | 4 B   | `i32` | Signed 32-bit                 |
| `int64`   | 8 B   | `i64` | Signed 64-bit                 |

All scalar values are stored as 64-bit words in variable slots. The specific
width matters for pointer load/store and for struct field layout. The short
aliases (`u8`, `u64`, `i32`, ...) are exact synonyms for the long form.

There is no `bool` keyword, and no `float` types. Use `0` / `1` for booleans
and integer math everywhere.

### Literals

- Decimal: `42`, `1000000`
- Hex: `0x1000`, `0xDEADBEEF`
- String: `"hello"` with `\n`, `\t`, `\\`, `\"`, `\0` escapes
- Character: `'A'`, `'\n'`, `'\t'`, `'\r'`, `'\0'`, `'\\'`, `'\''` — evaluates
  to the byte value of the character (e.g. `'A'` is 65, `'\n'` is 10).
  Use them directly in comparisons and arithmetic: `if c == 'a' { ... }`.

---

## 3. Variables and assignment

```kr
TYPE name = initializer
TYPE name                    // uninitialized — garbage contents
name = new_value
```

The type precedes the name (C-style, not Rust-style).

```kr
u32 status = 0
u64 base   = 0x3F000000
u8  byte   = 0xFF
```

### Compound assignment

| Op | Meaning        |
|----|----------------|
| `+=` | add            |
| `-=` | subtract       |
| `*=` | multiply       |
| `/=` | divide         |
| `%=` | remainder      |
| `&=` | bitwise AND    |
| `\|=` | bitwise OR     |
| `^=` | bitwise XOR    |
| `<<=` | left shift     |
| `>>=` | right shift    |

---

## 4. Operators

Expressions are parsed with a Pratt parser. Precedence from tightest to
loosest:

| Precedence | Operators                        | Notes                    |
|------------|----------------------------------|--------------------------|
| 110 (prefix) | `!`, `~`, `-`                  | Logical not, bitwise not, negation |
| 100        | `*`, `/`, `%`                    | Multiply, divide, remainder |
| 90         | `+`, `-`                         | Add, subtract            |
| 80         | `<<`, `>>`                       | Shift                    |
| 70         | `<`, `<=`, `>`, `>=`             | Unsigned comparison      |
| 60         | `==`, `!=`                       | Equality                 |
| 50         | `&`                              | Bitwise AND              |
| 40         | `^`                              | Bitwise XOR              |
| 30         | `\|`                             | Bitwise OR               |
| 20         | `&&`                             | Logical AND              |
| 10         | `\|\|`                           | Logical OR               |

`<`, `<=`, `>`, `>=` compare **unsigned**. For signed comparisons, use the
`signed_lt` / `signed_gt` / `signed_le` / `signed_ge` built-ins.

---

## 5. Control flow

### if / else

```kr
if x > 10 {
    println("big")
} else {
    println("small")
}
```

Parentheses around the condition are optional. `else if` works as a chain:

```kr
if n < 0 {
    println("negative")
} else if n == 0 {
    println("zero")
} else if n < 10 {
    println("small")
} else {
    println("big")
}
```

### while

```kr
u64 i = 0
while i < 10 {
    println(i)
    i = i + 1
}
```

### for (range)

```kr
for i in 0..n {
    println(i)
}
```

`0..n` is an **exclusive** range — `i` takes values `0, 1, ..., n-1`. There
is no inclusive `..=` form; use `0..n+1` when you need it.

### break and continue

```kr
while true {
    if done { break }
    if skip { continue }
    // ...
}
```

### match

```kr
match opcode {
    1 => { println("one") }
    2 => { println("two") }
    3 => { println("three") }
}
```

Arms are tested top-to-bottom. Each arm matches an integer literal or a
named integer constant. There is no default arm — if no arm matches, the
match is a no-op.

### return

```kr
fn get_value() -> u64 {
    return 42
}

fn do_thing() {
    return    // void return — also fine to just fall off the end
}
```

---

## 6. Functions

```kr
fn name(TYPE param1, TYPE param2) -> RETURN_TYPE {
    // body
    return value
}
```

The return type after `->` is optional; omitting it means the function
returns void. Parameters are `TYPE name` — type first.

```kr
fn add(u64 a, u64 b) -> u64 {
    return a + b
}

fn greet(u64 name) {
    print("Hello, ")
    print_str(name)
    println("!")
}
```

Recursion and mutual recursion work — function order within a file doesn't
matter.

### Calling functions

```kr
u64 r = add(2, 3)
greet("world")
```

Up to 8 arguments can be passed in registers (6 on Windows x64). Functions
with more arguments pass the overflow on the stack.

### Type parameters (generics)

A function may declare type parameters with `<T>` or `<T, U>`:

```kr
fn max_t<T>(T a, T b) -> T {
    if a > b { return a }
    return b
}

fn main() {
    exit(max_t(3, 42))   // 42
}
```

Type parameters are **syntactic only** in the current implementation:
every scalar is a 64-bit slot, so `T` is effectively `u64` at codegen
time. There is no monomorphization and no type checking across
instantiations — `max_t(3, 42)` and `max_t(struct_ptr_a, struct_ptr_b)`
compile to the same machine code. Use the syntax when it makes the
caller clearer; don't rely on it for type safety.

---

## 7. Structs, methods, and enums

### Structs

```kr
struct Point {
    u64 x
    u64 y
}
```

Field layout is packed — no alignment padding. Fields are stored in
declaration order at increasing offsets. Field sizes are determined by
their type (`u8` = 1 byte, `u32` = 4 bytes, `u64` = 8 bytes, etc.).

```kr
Point p            // stack-allocated struct value
p.x = 10
p.y = 20
println(p.x)
```

### Heap-allocated structs

A struct variable can also be initialized with an expression that
returns a pointer — typically `alloc(size)`. When written this way,
the variable holds the pointer and field access dereferences it:

```kr
struct Node {
    u64 value
    u64 next
}

fn main() {
    Node a = alloc(16)        // a holds the heap pointer
    Node b = alloc(16)
    a.value = 10
    a.next  = b               // field store on a pointer-backed struct
    b.value = 20
    b.next  = 0

    Node cur = a
    while cur != 0 {
        println(cur.value)
        cur = cur.next        // reassign pointer variable
    }
    exit(0)
}
```

This is the idiomatic form for linked lists, BSTs, graph nodes, and
any tree-shaped data. Field size is inferred from the struct
declaration just like stack structs; the only difference is that the
variable's slot holds a pointer to heap memory instead of stack
memory. Reassigning the pointer variable is allowed, so traversal
patterns like `cur = cur.next` work as expected.

### Methods

Attach a function to a struct with `fn StructName.method_name(StructName self, ...)`:

```kr
struct Point {
    u64 x
    u64 y
}

fn Point.sum(Point self) -> u64 {
    return self.x + self.y
}

fn main() {
    Point p
    p.x = 10
    p.y = 20
    u64 total = p.sum()   // 30
    println(total)
    exit(0)
}
```

The method receives `self` as a reference to the struct on the caller's
stack — `self.field` reads and writes work normally.

### Enums

```kr
enum Color {
    Red = 0
    Green = 1
    Blue = 2
}
```

`Color.Red`, `Color.Green`, `Color.Blue` are named integer constants usable
in any integer context (assignments, comparisons, match arms, switch bases,
etc.). Enums are a compile-time convenience; no runtime object is created.

> **Reminder**: all struct and array comparisons done with `<`, `<=`,
> `>`, `>=` are **unsigned** (see §4). If you need signed comparisons
> — for example when computing an AVL balance factor or a graph
> distance that can go negative — use the `signed_lt`/`signed_le`/
> `signed_gt`/`signed_ge` builtins. This trips people up in tree and
> heap code surprisingly often.

---

## 8. Arrays

### Local arrays

```kr
u8[256]  buffer         // byte buffer
u16[16]  samples        // 16 × 2-byte values
u32[10]  pixels         // 10 × 4-byte values
u64[10]  numbers        // 10 × 8-byte values

buffer[0]  = 0xAA
numbers[2] = 300
u64 first  = numbers[0]
```

Local arrays are allocated on the stack. The element size follows the
declared type — `u64[10]` reserves 80 bytes, `u32[10]` reserves 40, etc.
Indexing is scaled automatically (`numbers[2]` loads 8 bytes from offset
`2*8`). The variable holds a pointer to the first element, so `buffer`
alone evaluates to the base address. Indexing is unchecked.

### Static arrays

At module level, a static array gets storage in the data section:

```kr
static u8[1024]  message_buf      // 1024 bytes
static u16[16]   sensor_samples   // 32 bytes
static u32[10]   pixel_row        // 40 bytes
static u64[10]   counters         // 80 bytes

fn main() {
    message_buf[0] = 72   // 'H'
    message_buf[1] = 105  // 'i'
    message_buf[2] = 0
    counters[0] = 1000000
    counters[9] = 2000000
    print_str(message_buf)
    exit(0)
}
```

All integer element widths (`u8`/`u16`/`u32`/`u64`, `i8`/`i16`/`i32`/`i64`)
are supported and indexing is scaled automatically — `counters[5]` reads
8 bytes from offset `5*8`. (In compilers older than 2.6.3, wider element
types silently miscompiled; upgrade if you see garbage reads.)

Static arrays are zero-initialized by the loader.

### Struct arrays

Fixed-size arrays of struct instances work both locally and statically:

```kr
struct Point { u64 x; u64 y }

fn main() {
    Point[10] pts
    pts[0].x = 1
    pts[0].y = 2
    pts[5].x = 50
    println(pts[5].x)
    exit(0)
}
```

Element indexing uses the struct's full size as stride. `pts[i].field` is
a first-class syntax that reads and writes the `field` at the correct
offset within element `i`.

---

## 9. Slice parameters

A slice parameter `[TYPE] name` is sugar for a fat pointer: a `(ptr, len)`
pair passed as two separate arguments. Inside the function, `data.len`
reads the length, and `data` is a plain pointer for indexing.

```kr
fn sum_bytes([u8] data) -> u64 {
    u64 total = 0
    u64 i = 0
    u64 n = data.len
    while i < n {
        total = total + load8(data + i)
        i = i + 1
    }
    return total
}

fn main() {
    u8[6] buf
    buf[0] = 10
    buf[1] = 20
    buf[2] = 30
    // Caller passes (pointer, length) — two arguments
    u64 t = sum_bytes(buf, 3)
    println(t)
    exit(0)
}
```

The caller side explicitly passes the length as a normal second argument.
This is the classic C `(ptr, len)` pattern with a nicer symbolic name for
the length inside the callee.

---

## 10. Static variables and constants

### static

```kr
static u64 counter = 0
static u64 gpio_base = 0x3F200000

fn tick() {
    counter = counter + 1
}
```

Static variables live in the data section for the lifetime of the program.
They're initialized by the loader (BSS zero-fill; the `= value` initializer
is currently parsed but treated as zero — set the value at startup if you
need a non-zero default).

### const

```kr
const u64 BAUD = 115200
const u64 UART_BASE = 0x3F201000
```

`const` creates a compile-time integer constant. At use sites the value is
inlined — there is no runtime storage.

---

## 11. Pointer operations

KernRift has no dedicated pointer type. Addresses are just `u64` values. To
read or write memory at an address, use the pointer built-ins:

### The easy way

```kr
u64 v = load64(addr)          // read a 64-bit value
u32 x = load32(addr)          // read a 32-bit value
u16 h = load16(addr)          // read a 16-bit value
u8  b = load8(addr)           // read a single byte

store64(addr, 0xDEADBEEF)     // write 64 bits
store32(addr, 0x1234)         // write 32 bits
store16(addr, 0x5678)         // write 16 bits
store8(addr, 0xAA)            // write 1 byte
```

The load builtins zero-extend the read into a full `u64`. The store
builtins write exactly the specified width.

### The verbose way (unsafe blocks)

You can also write the raw pointer syntax:

```kr
u64 val = 0
unsafe { *(addr as u32) -> val }     // load
unsafe { *(addr as u8)  = some_byte } // store
```

The cast type determines access width. Supported cast types:
`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64` (plus the long
forms `uint8`..`int64`). `unsafe { ... }` is just a marker block — it
accepts one or more pointer statements.

The `load*` / `store*` builtins are equivalent and much easier to read —
prefer them unless you have a reason to use `unsafe` blocks.

---

## 12. Volatile and atomic

### Volatile: MMIO-safe loads and stores

For memory-mapped I/O, the compiler must not reorder, elide, or cache the
access, and the memory operation must complete before anything after it.

```kr
u32 v = vload32(mmio_addr)     // volatile load, barrier after
vstore32(mmio_addr, 0x01)      // volatile store, barrier before
```

All widths are available:
`vload8`, `vload16`, `vload32`, `vload64`, `vstore8`..`vstore64`.

The barrier emitted is:
- **x86_64**: `mfence` (full memory fence)
- **ARM64**: `DSB SY` (data synchronization barrier — waits for completion,
  not just ordering)

`volatile { *(addr as u32) = val }` is the equivalent block form and does
the same thing.

### Atomic operations

Lock-free atomic primitives are available as builtins:

```kr
u64 v = atomic_load(addr)
atomic_store(addr, v)
u64 old = atomic_cas(addr, expected, desired)   // compare-and-swap
u64 old = atomic_add(addr, delta)               // returns old value
u64 old = atomic_sub(addr, delta)
u64 old = atomic_and(addr, mask)
u64 old = atomic_or(addr, mask)
u64 old = atomic_xor(addr, mask)
```

These compile to `LOCK`-prefixed instructions on x86_64 and `LDXR`/`STXR`
exclusive pairs on ARM64. `atomic_cas` returns `1` on success, `0` on
failure.

---

## 13. Device blocks (MMIO)

For driver code, a `device` block describes a hardware register set at a
fixed base address. Field reads and writes compile directly to volatile
loads and stores of the right width — with the proper memory barriers.

```kr
device UART0 at 0x3F201000 {
    Data   at 0x00 : u32
    Flag   at 0x18 : u32
    IBRD   at 0x24 : u32
    FBRD   at 0x28 : u32
    LCRH   at 0x2C : u32
    Ctrl   at 0x30 : u32 rw
}

fn putc(u8 c) {
    // Spin until TX FIFO has room
    while (UART0.Flag & 0x20) != 0 { }
    UART0.Data = c
}
```

Syntax:

- `device NAME at ADDR { ... }` declares a device rooted at `ADDR`.
- `FIELD at OFFSET : TYPE [rw|ro|wo]` declares a register. The access
  specifier (`rw`, `ro`, `wo`) is currently optional and parsed-but-ignored
  — future versions will enforce it.
- Supported field types: `u8`, `u16`, `u32`, `u64` (and signed variants).

A read like `UART0.Data` emits a `vloadN` of the right width at
`0x3F201000 + 0x00`. A write like `UART0.Ctrl = 1` emits a `vstoreN` with
the appropriate barrier.

Device blocks sit on top of the volatile builtins — there is no hidden
mechanism, just a convenient named-register syntax.

---

## 14. Inline assembly

The `asm` keyword emits raw machine instructions at the call site.

### Single instruction

```kr
asm("nop")
asm("cli")
asm("sti")
```

### Multi-instruction block

```kr
asm {
    "cli";
    "mov rax, cr0";
    "sti"
}
```

### Raw hex bytes

When the assembler doesn't recognize a mnemonic, drop to hex:

```kr
asm("0x0F 0x01 0xD9")    // vmmcall (x86_64)
asm("0xD503201F")        // nop (ARM64)
```

### Supported instructions

**x86_64**: `nop`, `ret`, `hlt`, `int3`, `iretq`, `cli`, `sti`, `cpuid`,
`rdmsr`, `wrmsr`, `lgdt [rax]`, `lidt [rax]`, `invlpg [rax]`, `ltr ax`,
`swapgs`, control-register moves (`mov cr0, rax`, etc.), port I/O
(`in al, dx`, `out dx, al`, wide variants).

**ARM64**: `nop`, `ret`, `eret`, `wfi`, `wfe`, `sev`, barriers (`isb`,
`dsb sy/ish`, `dmb sy/ish`), `svc #N`, and `mrs` / `msr` for 20+ system
registers including `SCTLR_EL1`, `VBAR_EL1`, `TCR_EL1`, `MAIR_EL1`,
`MPIDR_EL1`, `CurrentEL`.

For anything not in the built-in table, use the raw hex form.

---

## 15. Imports

Bring functions and declarations from another file into the current
compilation unit:

```kr
import "std/io.kr"
import "std/string.kr"
import "utils.kr"
```

Import paths are resolved:

1. Relative to the importing file's directory
2. Then in the standard library location: `~/.local/share/kernrift/`
   (or `%LOCALAPPDATA%\KernRift\share\` on Windows)

Circular imports are detected and rejected. Each file is compiled at most
once regardless of how many files import it.

---

## 16. Built-in functions

All of these are compiler intrinsics — no runtime library, no imports
needed.

### I/O

| Function | Description |
|---|---|
| `print(literal_or_int)` | Print a string literal or format an integer as decimal. No newline. |
| `println(literal_or_int)` | Same, plus a newline. |
| `print_str(s)` | Print a null-terminated string from a pointer variable. |
| `println_str(s)` | Same, plus a newline. |
| `write(fd, buf, len)` | Write `len` bytes from `buf` to file descriptor `fd`. |
| `file_open(path, flags)` | Open a file. Returns a descriptor. |
| `file_read(fd, buf, len)` | Read up to `len` bytes. Returns bytes read. |
| `file_write(fd, buf, len)` | Write `len` bytes. Returns bytes written. |
| `file_close(fd)` | Close a descriptor. |
| `file_size(fd)` | Return the size of an open file. |

**Important:** `print(variable)` and `println(variable)` format the
variable as a decimal integer. If you want to print a string that lives
in a variable (e.g. the return value of `int_to_str`), use `print_str` /
`println_str` instead.

### Memory

| Function | Description |
|---|---|
| `alloc(size)` | Heap-allocate `size` bytes. Returns a pointer. |
| `dealloc(ptr)` | Free a previously allocated block. |
| `memcpy(dst, src, len)` | Copy `len` bytes. |
| `memset(dst, val, len)` | Fill `len` bytes with `val`. |
| `str_len(s)` | Length of a null-terminated string. |
| `str_eq(a, b)` | 1 if two null-terminated strings are equal, 0 otherwise. |

### Pointer load/store

| Function | Description |
|---|---|
| `load8/16/32/64(addr)` | Read a value of the given width, zero-extended to `u64`. |
| `store8/16/32/64(addr, val)` | Write a value of the given width. |
| `vload8/16/32/64(addr)` | Volatile load with barrier — for MMIO. |
| `vstore8/16/32/64(addr, val)` | Volatile store with barrier — for MMIO. |

### Atomic

| Function | Description |
|---|---|
| `atomic_load(ptr)` | Sequentially-consistent load. |
| `atomic_store(ptr, val)` | Sequentially-consistent store. |
| `atomic_cas(ptr, exp, new)` | Compare-and-swap. Returns 1 on success. |
| `atomic_add/sub/and/or/xor(ptr, val)` | RMW, returns old value. |

### Bit manipulation

| Function | Description |
|---|---|
| `bit_get(v, n)` | Bit `n` of `v` (0 or 1). |
| `bit_set(v, n)` | Return `v` with bit `n` set. |
| `bit_clear(v, n)` | Return `v` with bit `n` cleared. |
| `bit_range(v, start, width)` | Extract `width` bits starting at `start`. |
| `bit_insert(v, start, width, bits)` | Insert `bits` into `v` at position `start`. |

### Signed comparison

The normal `<`, `<=`, `>`, `>=` operators are unsigned. For signed
comparisons:

```kr
signed_lt(a, b)    signed_gt(a, b)
signed_le(a, b)    signed_ge(a, b)
```

### Platform and process

| Function | Description |
|---|---|
| `exit(code)` | Terminate the process with an exit code. |
| `get_target_os()` | Host OS: `0`=Linux, `1`=macOS, `2`=Windows, `3`=Android. |
| `get_arch_id()` | Host arch: `1`=x86_64, `2`=ARM64. |
| `exec_process(path)` | Spawn and wait for a process. Returns exit code. |
| `set_executable(path)` | `chmod +x` equivalent. |
| `get_module_path(buf, size)` | Write the current binary's path into `buf`. |
| `fmt_uint(buf, val)` | Format `val` as decimal into `buf`. Returns length. |
| `syscall_raw(nr, a1, a2, a3, a4, a5, a6)` | Raw syscall with up to 6 args. |

### Function pointers

| Function | Description |
|---|---|
| `fn_addr(name)` | Get the address of a named function. |
| `call_ptr(addr, ...)` | Call a function by address with any number of arguments. |

---

## 17. Annotations

Annotations appear immediately before a function or struct declaration.

### `@export`

Marks a function for inclusion in the output binary's symbol table (for
linking or ELF object introspection).

```kr
@export
fn my_entry() { }
```

### `@naked`

Emits a function with no prologue/epilogue. Useful for interrupt handlers
and low-level entry points that manage their own stack.

```kr
@naked fn isr() {
    asm { "cli"; "nop"; "iretq" }
}
```

### `@noreturn`

Marks a function that never returns (e.g. `panic`, infinite loops).
The compiler omits the epilogue.

```kr
@noreturn fn panic() {
    write(2, "panic\n", 6)
    while true { asm("hlt") }
}
```

### `@packed`

Accepted on struct declarations. KernRift structs are *already* packed
(no alignment padding), so this annotation is currently a no-op that
documents intent.

```kr
@packed struct Header {
    u8  kind
    u32 length
    u8  flags
}
```

### `@section("name")`

Parses and records a linker section name. Used with `--emit=obj` output.

```kr
@section(".text.init") fn early_start() { }
```

---

## 18. Compiler CLI

```sh
krc <file.kr>                        # compile to <stem>.krbo (fat binary)
krc <file.kr> -o out                 # specify output name
krc <file.kr> --arch=x86_64 -o out   # single-arch native ELF
krc <file.kr> --arch=arm64 -o out    # single-arch ARM64 ELF
krc <file.kr> --emit=asm -o out.s    # disassembled listing
krc <file.kr> --emit=obj -o out.o    # ELF relocatable object
krc <file.kr> --emit=pe -o out.exe   # Windows PE
krc <file.kr> --emit=macho -o out    # macOS Mach-O
krc <file.kr> --emit=android -o out  # Android ARM64 PIE ELF
krc --freestanding <file.kr> -o out  # no main trampoline, no auto-exit
krc check <file.kr>                  # run semantic checks only
krc fmt   <file.kr>                  # auto-format the file in place
krc lc <file.kr>                     # living compiler report (section 19)
krc lc --fix <file.kr>               # apply auto-fixes in place
krc lc --fix --dry-run <file.kr>     # preview auto-fixes without writing
krc lc --ci <file.kr>                # CI gate: exit non-zero if patterns fire
krc lc --min-fitness=N <file.kr>     # filter: only patterns with fitness >= N
krc lc --list-proposals              # print the proposal registry
krc lc --promote <name>              # promote a proposal to stable
krc lc --deprecate <name>            # mark a proposal as deprecated
krc lc --reject <name>               # revert a proposal to experimental
krc --version                        # print the compiler version
krc --help                           # usage info
```

### `kr` runner

```sh
kr program.krbo                      # run a fat binary on any platform
kr --version
kr --help
```

The `kr` runner extracts the slice matching the current host architecture
from a `.krbo` fat binary and executes it.

---

## 19. Living compiler

`krc lc` analyses KernRift source and produces a two-layer report. The
living compiler separates concerns into a **stable semantic core**
(correctness and structural issues) and an **adaptive surface layer**
(ergonomic migrations that lower to the same IR). This lets the language
evolve without destroying compatibility.

### Basic report

```sh
krc lc file.kr
```

Output has three sections: a telemetry summary, a fitness score
(layer-weighted, 0–100), and the patterns detected in each layer.
Patterns tagged `(auto-fix available)` can be rewritten mechanically.

### CI gating

```sh
krc lc --min-fitness=60 file.kr     # filter: only patterns with fitness >= 60
krc lc --ci file.kr                 # exit non-zero if any pattern fires
krc lc --ci --min-fitness=50 file.kr  # gate only on patterns >= 50
```

### Migration engine (auto-fix)

```sh
krc lc --fix file.kr                # rewrite in place
krc lc --fix --dry-run file.kr      # preview the rewritten source
```

The migration engine currently handles the `legacy_ptr_ops` pattern:

- `unsafe { *(addr as T) -> dest }`  →  `dest = loadN(addr)`
- `unsafe { *(addr as T) = val }`    →  `storeN(addr, val)`

Both forms lower to identical code at the codegen level, so the rewrite
is safe by construction.

### Proposal registry

The living compiler ships with a registry of candidate syntax evolutions,
each tagged with a lifecycle state (`experimental`, `stable`, or
`deprecated`):

```sh
krc lc --list-proposals
```

Proposals with triggers that match the current file fire inline in the
report. Under `#lang stable` (the default), only stable proposals fire.
Under `#lang experimental`, experimental proposals also fire as
"coming-soon" hints.

### Governance: persistent per-project state

Each project can override the compiler's baseline proposal states and
store them in a `.kernrift/proposals` file at the project root:

```sh
krc lc --promote <name>     # move a proposal to `stable`
krc lc --deprecate <name>   # move a proposal to `deprecated`
krc lc --reject <name>      # revert to `experimental`
```

The first invocation creates `.kernrift/proposals`. Subsequent runs of
`krc lc` in that directory automatically load the overrides. The format
is one line per proposal:

```
slice_for_buffer_params stable
tail_call_intrinsic experimental
extern_fn_decls deprecated
```

This is how the governance layer actually works — the compiler has a
baseline, each project can pin its own decisions, and everything is
version-controlled alongside the source.

See [`docs/LIVING_COMPILER.md`](LIVING_COMPILER.md) for the full
blueprint and the pipeline design.

---

## 20. Language profiles (`#lang`)

A source file may pin its required language profile on the first line:

```kr
#lang stable

fn main() {
    // only features promoted to the stable surface are allowed
    println("hello")
    exit(0)
}
```

```kr
#lang experimental

fn main() {
    // experimental features are also allowed
    exit(0)
}
```

Recognized profiles:

| Profile | Meaning |
|---|---|
| `stable` | Default. All stable features. Safe for production code. |
| `experimental` | Also allows features under active development. |

The directive must be the first non-empty line of the file. If absent,
the profile defaults to `stable`.

Profiles are part of the Living Compiler's two-layer model: the stable
semantic core doesn't change, but the adaptive surface layer may gate
certain features (like `tail_call()` or `extern fn` when those are added)
behind `#lang experimental`. This lets the language evolve without
breaking existing files — pin a file to `stable` and it keeps compiling
forever, even as new experimental features enter the language.

---

## 21. Freestanding mode

`krc --freestanding` produces a binary suitable for bare-metal:

- No automatic `exit(0)` at the end of `main`.
- No OS-specific syscall wrappers injected.
- The ELF entry point (`e_entry`) still points at `main` — you must
  provide `fn main()`. If you want a different name (e.g. `_start`),
  keep `fn main()` as the trampoline and have it call into your
  entry function.

```sh
krc --freestanding --arch=arm64 kernel.kr -o kernel.elf
```

Use this for kernel entry points, bootloaders, and embedded firmware.
The programmer is responsible for setting up the stack and handling
any return from `main`. Mark functions that never return with
`@noreturn` so the compiler skips the return-path check; annotate
interrupt handlers with `@naked` to suppress the prologue/epilogue.

Freestanding example:

```kr
@noreturn
fn main() {
    // kernel entry — set up your own state, never returns
    u64 vga = 0xB8000
    store16(vga + 0, 0x0F48)  // 'H' bright white
    store16(vga + 2, 0x0F69)  // 'i'
    while true { }
}
```

### Stack size warnings

The compiler prints a warning to stderr when a function's stack frame
exceeds 32768 bytes:

```
warning: large stack frame (49000 bytes) in function 'parse_module'
```

This catches accidental large local arrays that could overflow a kernel
stack. Big dispatch functions with many mutually exclusive branches
legitimately allocate slots across branches; the threshold is set high
enough to let those pass.

---

## 22. Extern functions

`extern fn` declares a function that is resolved by the platform linker at
link time. It has no body — the signature names an external symbol (typically
from libc or another static library):

```kr
extern fn strlen(u64 s) -> u64
extern fn write(u64 fd, u64 buf, u64 len) -> u64

fn main() {
    u64 msg = "hello from KernRift via libc!\n"
    write(1, msg, strlen(msg))
    exit(0)
}
```

Compile to a relocatable object and link with the platform toolchain:

```sh
# Linux
krc --emit=obj extern_libc.kr -o extern_libc.o
gcc extern_libc.o -o extern_libc -no-pie

# macOS
krc --target=macos --emit=obj extern_libc.kr -o extern_libc.o
clang extern_libc.o -o extern_libc

# Windows
krc --target=windows --emit=obj extern_libc.kr -o extern_libc.obj
link extern_libc.obj msvcrt.lib /ENTRY:main /SUBSYSTEM:console
```

The compiler emits relocations in the native format of each target:

| Target        | Format  | Relocation                |
|---------------|---------|---------------------------|
| Linux x86_64  | ELF     | `R_X86_64_PLT32`          |
| Linux ARM64   | ELF     | `R_AARCH64_CALL26`        |
| macOS x86_64  | Mach-O  | `X86_64_RELOC_BRANCH`     |
| macOS ARM64   | Mach-O  | `ARM64_RELOC_BRANCH26`    |
| Windows x64   | COFF    | `IMAGE_REL_AMD64_REL32`   |
| Windows ARM64 | COFF    | `IMAGE_REL_ARM64_BRANCH26`|

`extern fn` names shadow built-ins: if you declare `extern fn write(...)`,
calls to `write` resolve to the libc symbol instead of the `write` syscall
built-in. This lets you opt into the platform runtime on demand.

Note that programs that call buffered libc functions (like `printf` or
`puts`) from `main()` should exit via a libc `exit()` rather than the
built-in `exit()` — the built-in uses a raw syscall that bypasses libc's
stdio flush on exit. The safest pattern is to declare `extern fn exit`
and use that:

```kr
extern fn exit(u64 code)
extern fn puts(u64 s) -> u64

fn main() {
    puts("flushed through stdio")
    exit(0)
}
```

---

## 23. Binary formats

| Format | Produced by | Use |
|---|---|---|
| `.krbo` fat binary | default (no `--arch`) | Cross-platform distribution — `kr` picks the right slice |
| ELF executable | `--arch=x86_64` / `--arch=arm64` on Linux | Native Linux binary |
| ELF relocatable | `--emit=obj` | Link into an external object (`.o`) |
| Mach-O | `--emit=macho` | macOS executable (x86_64 or arm64) |
| PE | `--emit=pe` | Windows `.exe` |
| Android PIE ELF | `--emit=android` | Android ARM64 |
| Assembly listing | `--emit=asm` | Human-readable disassembly with labels |

A `.krbo` fat binary packs up to 7 platform slices (Linux x86_64, Linux
ARM64, Windows x86_64, Windows ARM64, macOS x86_64, macOS ARM64, Android
ARM64), each BCJ+LZ4 compressed. The `kr` runner extracts and executes
the slice matching the current host at startup.

---

## Appendix A. ABI reference

This is a quick reference for anyone reading the code `krc` generates or
linking it against other toolchains. It's the minimum you need to
reason about register allocation, interoperate with C, or write
`@naked` functions.

### x86_64

| Target  | Arg regs (1..6/8)                   | Return | Callee-saved                    | Stack align at CALL |
|---------|-------------------------------------|--------|---------------------------------|---------------------|
| Linux   | `rdi rsi rdx rcx r8 r9` (then stack) | `rax`  | `rbx rbp r12 r13 r14 r15 rsp`   | 16                  |
| macOS   | same (System V)                     | `rax`  | same                            | 16                  |
| Windows | `rcx rdx r8 r9` (then stack, +32 shadow) | `rax` | `rbx rbp rdi rsi rsp r12..r15 xmm6..xmm15` | 16 |

- KernRift currently allocates only GPRs — no XMM usage in generated
  code, so the caller-saved XMM registers are irrelevant to user code
  but matter when you link against C.
- On Windows, the first 32 bytes of the stack below `rsp` at call time
  are a **shadow** area owned by the callee. `krc` allocates it for you.
- `@naked` functions get no prologue/epilogue — you're responsible for
  stack alignment if you call into user code.

### arm64 (AArch64)

| Target       | Arg regs  | Return | Callee-saved       | Syscall nr in |
|--------------|-----------|--------|--------------------|---------------|
| Linux        | `x0..x7`  | `x0`   | `x19..x28 sp fp lr` | `x8`          |
| macOS        | `x0..x7`  | `x0`   | same               | `x16`         |
| Android      | `x0..x7`  | `x0`   | same               | `x8`          |
| Windows arm64| `x0..x7`  | `x0`   | same               | (no syscalls; uses kernel32 IAT) |

## Appendix B. Syscall numbers

`krc`'s builtins lower to real kernel syscalls. The table below is the
number used by each builtin on each supported (OS × arch) target.
Useful when reading `--emit=asm` output, stepping through with a
debugger, or writing portable code that uses `syscall_raw`.

### Linux x86_64

| Builtin    | nr  | C name          |
|------------|-----|-----------------|
| `write`    | 1   | `write`         |
| `read`     | 0   | `read`          |
| `exit`     | 231 | `exit_group`    |
| `alloc`    | 9   | `mmap`          |
| `dealloc`  | 11  | `munmap`        |
| `file_open`| 2   | `open`          |
| `file_read`| 0   | `read`          |
| `file_write`|1   | `write`         |
| `file_close`|3   | `close`         |
| `time_ns`  | 228 | `clock_gettime` |
| `set_executable` | 90 | `chmod` |

`syscall_raw(nr, a1, a2, a3, a4, a5, a6)` passes `nr` in `rax` and the
arguments in `rdi rsi rdx r10 r8 r9` (standard Linux x86_64 ABI).

### Linux arm64

| Builtin    | nr  |
|------------|-----|
| `write`    | 64  |
| `read`     | 63  |
| `exit`     | 93  |
| `alloc`    | 222 (`mmap`)  |
| `dealloc`  | 215 (`munmap`) |
| `file_open`| 56  (`openat`) |
| `time_ns`  | 113 (`clock_gettime`) |
| `set_executable` | 53 (`fchmodat`) |

`syscall_raw` passes nr in `x8` and args in `x0..x5`.

### macOS x86_64

macOS syscall numbers use the high nibble to encode the syscall class
(2 = Unix class). The numbers below are the full 32-bit values passed
in `rax`; arguments go in `rdi rsi rdx rcx r8 r9` like Linux.

| Builtin    | nr          | C name   |
|------------|-------------|----------|
| `exit`     | `0x2000001` | `exit`   |
| `write`    | `0x2000004` | `write`  |
| `read`     | `0x2000003` | `read`   |
| `alloc`    | `0x20000C5` | `mmap`   |

### macOS arm64

On arm64 macOS, the syscall number goes in **`x16`** (not `x8` as on
Linux). Numbers are the plain Darwin numbers, not the class-tagged form.

| Builtin    | nr  |
|------------|-----|
| `exit`     | 1   |
| `write`    | 4   |
| `read`     | 3   |
| `alloc`    | 197 |

### Windows

Windows x86_64 and arm64 do not use direct syscalls — every I/O and
process-control builtin lowers to a call through the binary's Import
Address Table (IAT) against `kernel32.dll`:

| Builtin            | kernel32 import            |
|--------------------|----------------------------|
| `exit`             | `ExitProcess`              |
| `write`            | `GetStdHandle` + `WriteFile` |
| `read`             | `GetStdHandle` + `ReadFile` |
| `alloc`            | `VirtualAlloc`             |
| `dealloc`          | `VirtualFree`              |
| `file_open`        | `CreateFileA`              |
| `file_read`        | `ReadFile`                 |
| `file_write`       | `WriteFile`                |
| `file_close`       | `CloseHandle`              |
| `exec_process`     | `CreateProcessA` + `WaitForSingleObject` + `GetExitCodeProcess` + `ExitProcess` |
| `set_executable`   | no-op (Windows has no executable bit) |

`syscall_raw` is **not supported** on Windows — the platform has no
stable syscall numbering. The `--target=windows` PE output uses IAT
imports exclusively.

## Appendix C. `--emit=obj` section layout

A relocatable object file (`.o` on Linux/macOS, `.obj` on Windows) produced
by `--emit=obj` contains the minimum set of sections the platform linker
needs. No `.rodata`, no `.bss`, no `.data` — string literals and static
scalars are placed at the end of `.text` and referenced with RIP-relative
addressing.

### Linux x86_64 / arm64 (ELF)

| Index | Name              | Type      | Purpose |
|-------|-------------------|-----------|---------|
| 0     | (null)            | NULL      | required by ELF |
| 1     | `.text`           | PROGBITS  | code + string literals + static scalars |
| 2     | `.data`           | PROGBITS  | (emitted empty — static data lives inside `.text`) |
| 3     | `.symtab`         | SYMTAB    | every `fn` is a symbol; `main` is `GLOBAL`, others `LOCAL` |
| 4     | `.strtab`         | STRTAB    | symbol name strings |
| 5     | `.shstrtab`       | STRTAB    | section header names |
| 6     | `.note.GNU-stack` | PROGBITS (flags=0) | marks the binary as non-exec-stack so `ld` doesn't warn |
| 7     | `.rela.text`      | RELA      | only present if the program uses `extern fn` |

Relocation types for `extern fn` call sites:
- **x86_64**: `R_X86_64_PLT32` (disp32 = -4 addend)
- **arm64**: `R_AARCH64_CALL26` (addend 0)

### macOS x86_64 / arm64 (Mach-O)

One `__TEXT,__text` section containing code + string literals. Symbol
names are prefixed with an underscore (`_main`, `_write`) as required
by the Darwin C ABI. `extern fn` call sites use relocations
`X86_64_RELOC_BRANCH` (x86_64) and `ARM64_RELOC_BRANCH26` (arm64).

### Windows x86_64 / arm64 (COFF `.obj`)

One `.text` section, one COFF symbol table. No underscore prefix on
x86_64. `extern fn` call sites use relocations
`IMAGE_REL_AMD64_REL32` (x86_64) and `IMAGE_REL_ARM64_BRANCH26` (arm64).

### Linking with gcc or clang

```sh
# Linux
krc --emit=obj prog.kr -o prog.o
gcc prog.o -o prog -no-pie

# No more "missing .note.GNU-stack" warning as of v2.6.3 — the compiler
# emits the section by default so linked binaries get a non-executable
# stack.
```

## Appendix D. `.krbo` fat-binary format (v2)

The runtime format for `.krbo` files — directly parseable without any
KernRift toolchain.

**Layout**:
```
offset  size  field
0x00    8     magic:        "KRBOFAT\0"
0x08    4     version:      u32 = 2
0x0C    4     arch_count:   u32 (≤ 7)
0x10    (arch_count × 48)   arch descriptor table
...     compressed slice blobs (per arch)
...     runtime blobs (per arch)
```

**Arch descriptor** (48 bytes each, one per slice):
```
offset  size  field
+0x00   4     arch_id:           u32 (see table below)
+0x04   4     compression:       u32 (1 = LZ4 frame, preceded by BCJ filter)
+0x08   8     slice_offset:      u64 (from start of file)
+0x10   8     slice_comp_size:   u64
+0x18   8     slice_uncomp_size: u64
+0x20   8     runtime_offset:    u64 (kr runner blob for this arch)
+0x28   8     runtime_len:       u64
```

**Arch IDs**:

| id | OS       | arch   |
|----|----------|--------|
| 1  | Linux    | x86_64 |
| 2  | Linux    | arm64  |
| 3  | Windows  | x86_64 |
| 4  | Windows  | arm64  |
| 5  | macOS    | x86_64 |
| 6  | macOS    | arm64  |
| 7  | Android  | arm64  |

**Decompression**: each slice is LZ4-compressed with a BCJ filter
applied *before* compression. On extraction the runner first LZ4-
decompresses, then runs the matching BCJ filter in reverse to restore
the original call/jmp offsets. BCJ filter selection:
- x86-family arch_ids (1, 3, 5): x86_64 BCJ filter (rewrites `E8`/`E9`
  disp32 offsets to absolute, for better compression).
- arm-family arch_ids (2, 4, 6, 7): AArch64 BCJ filter (rewrites `BL`
  imm26 fields).

**Minimal Python decoder**:
```python
import struct, lz4.frame

def parse_krbo(path):
    d = open(path, 'rb').read()
    assert d[:8] == b'KRBOFAT\0'
    ver, n = struct.unpack_from('<II', d, 8)
    assert ver == 2
    slices = []
    for i in range(n):
        off = 16 + i * 48
        (arch_id, compression,
         slice_off, comp_sz, uncomp_sz,
         rt_off, rt_len) = struct.unpack_from('<IIQQQQQ', d, off)
        raw_lz4 = d[slice_off:slice_off + comp_sz]
        # NOTE: you still need to reverse the BCJ filter after lz4 decode
        decompressed = lz4.frame.decompress(raw_lz4)
        slices.append((arch_id, decompressed))
    return slices
```

`arch_count` is capped at 7 by the current emitter; the format has
room for more if future targets are added.

---

*See the `examples/` directory for runnable programs demonstrating every
feature in this reference.*
