# KernRift Language Reference

**KernRift is a bare-metal systems programming language and compiler created by Pantelis Christou.**

KernRift is a freestanding, ahead-of-time compiled systems language for kernel
and driver development.  It targets bare-metal x86-64 and ARM64 (AArch64) with
no runtime, no libc, and no implicit heap.  Safety properties — execution
context, MMIO capabilities, lock order, and yield restrictions — are checked at
compile time, not at runtime.  Every construct maps directly to machine
instructions; there are no hidden costs.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Quick Start](#2-quick-start)
3. [Types](#3-types)
4. [Variables and Assignment](#4-variables-and-assignment)
5. [Operators](#5-operators)
6. [Functions](#6-functions)
7. [Control Flow](#7-control-flow)
8. [Constants](#8-constants)
9. [Structs and Enums](#9-structs-and-enums)
10. [Hardware Access (Device Blocks)](#10-hardware-access-device-blocks)
11. [Kernel Safety Annotations](#11-kernel-safety-annotations)
12. [Locks](#12-locks)
13. [Per-CPU Variables](#13-per-cpu-variables)
14. [Extern Functions](#14-extern-functions)
15. [Tail Calls](#15-tail-calls)
16. [Critical Blocks](#16-critical-blocks)
17. [Unsafe Blocks](#17-unsafe-blocks)
18. [Inline Assembly](#18-inline-assembly)
19. [Kernel Annotations](#19-kernel-annotations)
20. [Signed Comparisons](#20-signed-comparisons)
21. [Bitfield Operations](#21-bitfield-operations)
22. [Volatile Blocks](#22-volatile-blocks)
23. [Freestanding Mode](#23-freestanding-mode)
24. [Port I/O Intrinsics](#24-port-io-intrinsics)
25. [Syscall Intrinsic](#25-syscall-intrinsic)
26. [Built-in Host Functions](#26-built-in-host-functions)
27. [Atomic Builtins](#27-atomic-builtins)
28. [Slice Indexing](#28-slice-indexing)
29. [Compiler CLI Reference](#29-compiler-cli-reference)
30. [Adaptive Surface & Living Compiler](#30-adaptive-surface--living-compiler)
31. [Binary Artifact Formats](#31-binary-artifact-formats)

---

## 1. Introduction

KernRift source files use the `.kr` extension.  One file is one module.
Comments are `//` (line) or `/* ... */` (block).  Statements do not require a
trailing semicolon in the new surface syntax, though one is accepted and
ignored.

---

## 2. Quick Start

### Hello, kernel world

The simplest valid program prints a string and exits:

```kr
@ctx(thread, boot)
fn entry() {
    print("Hello, kernel!\n")
}
```

Compile and run it:

```sh
krc hello.kr                # produces hello.krbo (native .krbo)
./hello                           # run the native binary

krc --emit=krbofat -o hello.krbo hello.kr   # fat binary (x86_64 + arm64)
./hello                           # run the native binary
```

### Minimal MMIO driver

```kr
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}

@ctx(thread, boot)
fn send_byte(uint8 b) {
    UART0.Data = b
}
```

`@module_caps(Mmio)` declares that this module uses the safe MMIO capability.
`device` declares the hardware block at a base address with named registers.
`@ctx(thread, boot)` restricts callers to thread and boot contexts.
The assignment `UART0.Data = b` is a volatile MMIO write enforced by the
compiler.

---

## 3. Types

### Scalar types

| Type      | Width | Notes                                      |
|-----------|-------|--------------------------------------------|
| `uint8`   | 1 B   | Unsigned byte. Aliases: `u8`, `byte`       |
| `uint16`  | 2 B   | Unsigned 16-bit. Alias: `u16`              |
| `uint32`  | 4 B   | Unsigned 32-bit. Alias: `u32`              |
| `uint64`  | 8 B   | Unsigned 64-bit, pointer-sized. Aliases: `u64`, `addr` |
| `int8`    | 1 B   | Signed byte. Alias: `i8`                   |
| `int16`   | 2 B   | Signed 16-bit. Alias: `i16`                |
| `int32`   | 4 B   | Signed 32-bit. Alias: `i32`                |
| `int64`   | 8 B   | Signed 64-bit. Alias: `i64`                |
| `float32` | 4 B   | IEEE-754 single. Alias: `f32`. *(V2 codegen)* |
| `float64` | 8 B   | IEEE-754 double. Alias: `f64`. *(V2 codegen)* |
| `float16` | 2 B   | IEEE-754 half, stored as raw 16-bit. *(V2)* |
| `bool`    | 1 B   | Boolean; `true` / `false` literals         |
| `char`    | 1 B   | ASCII character; `'x'` literals            |

### Slice type

`[T]` is a fat pointer — a `(ptr: uint64, len: uint64)` pair under SysV ABI.
Slices are views over caller-owned memory; no allocation occurs.

```kr
fn write_buf(uint8 first, [uint8] data) { ... }
```

---

## 4. Variables and Assignment

### Declaration

```kr
TYPE name = expr
TYPE name           // uninitialized
```

The type precedes the name (C style, not `name: TYPE`).

```kr
uint32 status = UART0.Status
uint8  byte   = 0xFF
bool   ready  = false
float32 scale = 1.0
```

### Simple assignment

```kr
name = expr
DEVICE.FIELD = expr
```

```kr
status = 0
UART0.Data = byte
```

### Compound assignment

```kr
name op= expr
```

All supported compound operators:

| Operator | Meaning            |
|----------|--------------------|
| `+=`     | add                |
| `-=`     | subtract           |
| `&=`     | bitwise AND        |
| `\|=`    | bitwise OR         |
| `^=`     | bitwise XOR        |
| `<<=`    | left shift         |
| `>>=`    | right shift        |
| `*=`     | multiply           |
| `/=`     | divide             |
| `%=`     | remainder          |

```kr
count += 1
flags &= 0xF0
mask |= 0x01
shift <<= 2
```

---

## 5. Operators

Expressions use a Pratt parser.  Precedence from highest (tightest) to lowest:

| Precedence | Operators            | Notes                     |
|------------|----------------------|---------------------------|
| 110 (prefix) | `!`, `~`, `-`      | Logical not, bitwise not, negation |
| 100        | `*`, `/`, `%`        | Multiply, divide, remainder        |
| 90         | `+`, `-`             | Add, subtract             |
| 80         | `<<`, `>>`           | Shift left, shift right   |
| 50         | `&`                  | Bitwise AND               |
| 40         | `^`                  | Bitwise XOR               |
| 30         | `\|`                  | Bitwise OR                |
| 60         | `==`, `!=`           | Equality                  |
| 70         | `<`, `>`, `<=`, `>=` | Comparison                |
| 20         | `&&`                 | Logical AND               |
| 10         | `\|\|`               | Logical OR                |

All binary operators are left-associative.  Use parentheses to override:

```kr
uint32 x = (a + b) & 0xFF
bool   ok = (flags & 0x01) != 0
```

Literals: decimal `42`, hexadecimal `0x1000`, binary `0b1010`, float `3.14`,
char `'A'`, bool `true` / `false`.

---

## 6. Functions

### Syntax

```kr
fn name(TYPE param, TYPE param, ...) -> TYPE {
    body
}
```

The return type after `->` is optional; omitting it means the function returns
void.  Parameters are written as `TYPE name` (type before name).

```kr
fn get_status() -> uint32 {
    uint32 s = UART0.Status
    return s
}

fn send(uint8 b) {
    UART0.Data = b
}
```

There is no implicit return.  Every code path that returns a value must end
with an explicit `return expr`.

### Parameters

Scalar and slice parameters:

```kr
fn compute(uint32 a, uint32 b) -> uint32 { ... }
fn dma_setup([uint8] buf) { ... }    // slice: (ptr, len) pair
```

### Slice length access

Inside a function, `slice.len` reads the element count of a slice parameter:

```kr
fn write_buf([uint8] data) {
    uint64 n = data.len
}
```

### Calling functions

A function defined anywhere in the same file can be called by name.  Arguments
are passed positionally.  The call result can be assigned to a variable whose
type matches the callee's return type.

```kr
fn double(uint32 n) -> uint32 {
    uint32 result = n
    result += n
    return result
}

fn quadruple(uint32 n) -> uint32 {
    uint32 r = double(n)
    r = double(r)
    return r
}

@ctx(thread, boot)
fn entry() {
    uint32 x = quadruple(3)   // x == 12
}
```

The compiler performs a topological sort of all call edges and rejects any
cycle, including trivial self-calls and mutual recursion (`a → b → a`).  Use
an explicit loop for iterative algorithms.

**KR0.1 restriction:** recursive calls are rejected at compile time.  A
function that calls itself directly or through a cycle will produce an error:

```
error: recursion unsupported in KR0.1: factorial -> factorial
```

Mutual recursion (`a → b → a`) is equally rejected.  Use an explicit loop or
an iterative stack to replace recursive algorithms.

### Built-in `print` statement

`print("literal")` writes a string literal to the UART output buffer.  The
argument must be a string literal — variables and expressions are not accepted.
Newlines are written as `\n`.

```kr
@ctx(thread, boot)
fn entry() {
    print("Booting...\n")
    print("Done.\n")
}
```

`print` is the only built-in I/O statement in the freestanding environment.
It writes to a memory-mapped UART buffer at `0x10000000`; the `kernrift`
runtime flushes the buffer to stdout after the program returns.

---

## 7. Control Flow

### if / else

```kr
if condition {
    // then
} else {
    // else (optional)
}
```

```kr
if status == 0 {
    uint32 x = 1
} else {
    uint32 x = 2
}
```

Conditions are full expressions.  There are no parentheses around the
condition (they are accepted but not required).

### while

```kr
while condition {
    body
}
```

```kr
uint32 i = 0
while i < n {
    i += 1
}
```

The type of the condition expression must be boolean or a comparison.  When
the loop variable is a typed integer (`uint32`, `uint64`, etc.), the compiler
uses that type for all arithmetic inside the loop body.

A `while` loop with a literal bound:

```kr
uint32 i = 0
while i < 10 {
    i = i + 1
}
```

### for (range loop)

Exclusive range `..`:

```kr
for i in 0..n {
    total += 1
}
```

Inclusive range `..=`:

```kr
for i in 0..=255 {
    sum += i
}
```

The loop variable `i` is an implicit `uint64` induction variable.  It is
read-only inside the body.

### break and continue

```kr
while true {
    if done { break }
    if skip { continue }
    total += 1
}
```

### return

```kr
return expr    // return a value
return         // void return
```

Return a variable:

```kr
fn get_val() -> uint32 {
    uint32 x = 42
    return x
}
```

Return a function parameter (parameters are fully accessible as local variables):

```kr
fn identity(uint32 n) -> uint32 {
    return n
}
```

Return after a conditional:

```kr
fn clamp_byte(uint32 x) -> uint32 {
    if x > 255 {
        uint32 max = 255
        return max
    }
    return x
}
```

---

## 8. Constants

```kr
const TYPE NAME = VALUE
```

Integer constants only.  Constants are inlined at every use site; no runtime
storage.

```kr
const uint32 UART_ENABLE = 0x0001
const uint16 BAUD_115200 = 0x001A
const uint64 BASE_ADDR   = 0xFEB00000
```

---

## 9. Structs and Enums

### Structs

```kr
struct NAME {
    field: TYPE,
    ...
}
```

Layout is C-style packed: no alignment padding, fields in declaration order.
Field offsets are compile-time constants available as `NAME::field`.

```kr
struct UartRegs {
    Control: uint32,   // offset 0
    Status:  uint32,   // offset 4
    Data:    uint8,    // offset 8
}
```

### Enums

```kr
enum NAME: TYPE {
    VARIANT = INTEGER,
    ...
}
```

Each variant is a named integer constant.  Referenced as `NAME::VARIANT`.

```kr
enum UartBase: uint64 {
    Uart0 = 0x40000000,
    Uart1 = 0x40001000,
}
```

Enums and structs are compile-time tools for address arithmetic.  No runtime
objects are created.

---

## 10. Hardware Access (Device Blocks)

### Device declaration

```kr
device NAME at BASE_ADDR {
    FIELD at OFFSET : TYPE ACCESS
    ...
}
```

`ACCESS` is one of `rw` (read-write), `ro` (read-only), `wo` (write-only).
Requires `@module_caps(Mmio)` at the top of the file.

```kr
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
    CR     at 0x08 : uint16 rw
}
```

### Reading a device register

Assign the field to a variable; the compiler emits a volatile load:

```kr
uint32 s = UART0.Status
uint16 c = UART0.CR
```

### Writing a device register

```kr
UART0.Data = byte
UART0.CR   = 0xFF
```

### Raw MMIO access

For hardware not described by a device block.  Requires `@module_caps(MmioRaw)`.

```kr
@module_caps(MmioRaw)

fn entry() {
    raw_write<uint32>(0x1014, x)
    raw_read<uint32>(0x1014, result)
}
```

`raw_write<T>(addr, value)` performs a volatile store to an arbitrary address.
`raw_read<T>(addr, slot)` performs a volatile load.

---

## 11. Kernel Safety Annotations

Annotations appear on the line(s) immediately before `fn`.  Multiple
annotations can share a line or occupy separate lines; order does not matter.

### `@ctx(context, ...)`

Declares which execution contexts the function may run in.

| Context  | Meaning                                       |
|----------|-----------------------------------------------|
| `thread` | Normal thread / task context                  |
| `boot`   | Early boot, before scheduling is active       |
| `irq`    | Hardware interrupt handler                    |
| `nmi`    | Non-maskable interrupt (strictest)            |

```kr
@ctx(thread, boot)
fn init() { ... }

@ctx(irq)
fn irq_handler() { ... }
```

For any call edge `caller → callee`, the compiler requires
`ctx(caller) ⊆ ctx(callee)`.

### `@eff(effect, ...)`

Declares side effects.  An empty `@eff()` declares no effects.

| Effect    | Meaning                                |
|-----------|----------------------------------------|
| `Mmio`    | MMIO reads / writes                    |
| `alloc`   | Memory allocation (forbidden in `irq`) |
| `block`   | Blocking / sleeping                    |
| `yield`   | Voluntary CPU yield                    |

### `@module_caps(cap, ...)`

Top-level module declaration listing which capability classes the module may
use.  The compiler rejects any operation that requires an unlisted capability.

| Capability | Meaning                                      |
|------------|----------------------------------------------|
| `Mmio`     | MMIO via declared device blocks              |
| `MmioRaw`  | Raw MMIO at arbitrary addresses              |
| `Ioport`   | x86 I/O port instructions                   |

```kr
@module_caps(Mmio)
@module_caps(MmioRaw)
@module_caps(Mmio, MmioRaw)
```

### `@leaf`

Marks a function as a leaf: it does not call any other KernRift functions.
Used by the lock-budget checker.

```kr
@leaf
fn tiny() { }
```

### `@lock_budget(N)`

The number of non-`@leaf` function calls on any path between an `acquire` and
its matching `release` must not exceed `N`.

```kr
@lock_budget(3)
@ctx(thread)
fn do_work() {
    acquire(DevLock)
    step_a()
    step_b()
    step_c()
    release(DevLock)
}
```

### `@noyield`

The function must not call any function that may yield.  Required on IRQ
handlers and scheduler hooks.

```kr
@noyield
@ctx(irq)
fn irq_handler() { ... }
```

### `@hotpath`

Aligns the function to a 16-byte boundary and marks it as a performance-critical
hot path.  Use on syscall fast paths and tight interrupt dispatch loops.

```kr
@hotpath
@ctx(irq)
fn fast_dispatch() { ... }
```

### `yieldpoint()`

Inserts a voluntary CPU yield point.  Legal only when not in IRQ context and
not inside a `critical { }` block or under `@noyield`.  The compiler rejects
uses that violate these constraints.

```kr
@ctx(thread)
fn pump() {
    while pending() {
        process_one()
        yieldpoint()    // cooperatively yield to the scheduler
    }
}
```

---

## 12. Locks

### Declaration

```kr
lock NAME
```

Declared at module level, no semicolon required (accepted and ignored).

```kr
lock ConsoleLock
lock DevLock
```

### Acquire and release

```kr
acquire(NAME)
release(NAME)
```

```kr
lock ConsoleLock

fn write_char(uint8 b) {
    acquire(ConsoleLock)
    UART0.Data = b
    release(ConsoleLock)
}
```

Acquire and release must be balanced on all paths.  The compiler builds a
lock graph across the module and fails the build if a deadlock cycle is
detected.  Always acquire locks in a consistent global order.

---

## 13. Per-CPU Variables

Declared at module level:

```kr
percpu NAME: TYPE
```

Accessed with the low-level intrinsics `percpu_read<T>` and `percpu_write<T>`
(old surface syntax, still supported):

```kr
percpu cpu_active: uint32

fn mark_active() {
    percpu_write<uint32>(cpu_active, 0x01)
}
```

Per-cpu variables have no address in the conventional sense; the compiler
generates `%gs`-relative accesses on x86-64.

---

## 14. Extern Functions

```kr
extern fn name(TYPE param, ...);
```

Declares a function implemented in another compilation unit (C, assembly).
Annotations must describe the function's context and capabilities so the
compiler can enforce call-edge rules.

```kr
extern @ctx(thread, boot) fn platform_barrier();
extern @ctx(irq) fn uart_irq_ack();
extern @ctx(thread, boot) fn memcpy([uint8] dst, [uint8] src, uint64 n);
```

---

## 15. Tail Calls

A tail call discards the current function's stack frame and jumps directly to the callee.  The result is zero stack growth per iteration, which is the correct way to implement unbounded poll loops and state machines in kernel code — especially in `@ctx(irq)` functions where stack space is fixed and never reclaimed.

### Intrinsic form

```kr
tail_call(callee[, arg, ...])
```

`tail_call` is an explicit intrinsic statement.  It transfers control to `callee` with zero stack growth.  Arguments are passed positionally.

```kr
@ctx(irq)
fn uart_poll(uint64 head) {
    uint64 next = (head + 1) & 7
    tail_call(uart_poll, next)   // jmp uart_poll — no frame growth
}
```

`tail_call` requires `--surface experimental`.  On stable surface, the living compiler emits the `try_tail_call` suggestion when plain calls are present but no tail calls are.

### Auto-fix

`krc lc --fix --dry-run <file.kr>` previews what a tail-call rewrite would look like as a unified diff.  `--fix --write` applies it atomically.  The rewrite prepends `tail ` to the last bare call statement in each function body.

### Constraints

- `tail_call` must be the last statement executed in the function.
- The callee's `@ctx` must be compatible with the caller's (same rules as any other call edge).
- Arguments are limited to six by the SysV ABI on x86-64.

---

## 16. Critical Blocks

A `critical { }` block declares that the enclosed statements form an atomic
critical section.  The kernel profile (`--profile kernel`) forbids `alloc`,
`block`, and `yield` effects anywhere inside a critical block.

```kr
lock ConsoleLock

fn write_char(uint8 b) {
    acquire(ConsoleLock)
    critical {
        UART0.Data = b     // no yield, no alloc inside this block
    }
    release(ConsoleLock)
}
```

Critical blocks may be nested.  The effect restriction applies to the entire
dynamic extent — including any function called from inside the block.

---

## 17. Unsafe Blocks

An `unsafe { }` block enables operations that bypass the compiler's normal
safety checks, such as inline assembly via `asm!`.  The compiler emits
`UnsafeEnter`/`UnsafeExit` markers in KRIR so the unsafe region is visible
to downstream analysis passes.

```kr
fn flush_tlb() {
    unsafe {
        asm!(invlpg)
    }
}
```

Use `unsafe` only for unavoidable hardware interactions that have no safe
surface equivalent.  All capability and context rules still apply inside an
unsafe block.

---

## 18. Inline Assembly

The `asm` statement emits raw machine instructions at the call site.

### Single instruction

```kr
asm("nop")
asm("cli")
asm("sti")
```

### Multi-line block

```kr
asm {
    "cli";
    "mov cr0, rax";
    "sti"
}
```

### Raw hex bytes

When the assembler doesn't recognize a mnemonic, use raw hex:

```kr
asm("0x0F 0x01 0xD9")   // VMMCALL — x86 only
asm("0xD503201F")        // NOP — ARM64
```

### Supported x86_64 instructions

| Category | Instructions |
|----------|-------------|
| Basic | `nop`, `ret`, `hlt`, `int3`, `iretq` |
| Interrupts | `cli`, `sti` |
| System | `cpuid`, `rdmsr`, `wrmsr`, `lgdt [rax]`, `lidt [rax]`, `invlpg [rax]`, `ltr ax`, `swapgs` |
| Control regs | `mov cr0, rax`, `mov cr3, rax`, `mov cr4, rax`, `mov rax, cr0`, `mov rax, cr3`, `mov rax, cr4` |
| I/O | `in al, dx`, `out dx, al`, `inw`, `outw`, `ind`, `outd` |

### Supported ARM64 instructions

| Category | Instructions |
|----------|-------------|
| Basic | `nop`, `ret`, `eret` |
| Wait | `wfi`, `wfe`, `sev` |
| Barriers | `isb`, `dsb sy`, `dsb ish`, `dmb sy`, `dmb ish` |
| System regs | `mrs x0, <sysreg>`, `msr <sysreg>, x0` (e.g. `SCTLR_EL1`, `VBAR_EL1`, `MPIDR_EL1`) |
| Supervisor | `svc #N` (16-bit immediate) |

---

## 19. Kernel Annotations

### `@naked`

Naked functions have no compiler-generated prologue or epilogue. Use for
interrupt service routines and other entry points that manage their own stack:

```kr
@naked fn isr_handler() {
    asm { "cli"; "nop"; "iretq" }
}
```

### `@noreturn`

Marks functions that never return (diverging functions). The compiler omits
the epilogue:

```kr
@noreturn fn panic() {
    write(2, "kernel panic\n", 13)
    asm("cli")
    while true { asm("hlt") }
}
```

### `@packed`

Packed structs have no alignment padding between fields:

```kr
@packed struct DeviceRegs {
    uint8 status
    uint32 data
    uint8 control
}
```

### `@section`

Places a function in a specific linker section (parsed and stored for future
linker script support):

```kr
@section(".text.init") fn early_init() { ... }
```

---

## 20. Signed Comparisons

All standard comparison operators (`<`, `<=`, `>`, `>=`) use unsigned
comparison. For signed arithmetic (needed in kernel offset calculations,
temperature readings, etc.), use the signed comparison builtins:

```kr
uint64 a = 0xFFFFFFFFFFFFFFFF   // -1 in two's complement
uint64 b = 1

if signed_lt(a, b) { ... }     // true: -1 < 1
if signed_gt(b, a) { ... }     // true: 1 > -1
if signed_le(a, a) { ... }     // true: -1 <= -1
if signed_ge(b, a) { ... }     // true: 1 >= -1
```

These emit `setl`/`setg`/`setle`/`setge` on x86_64 and `CSET LT`/`GT`/`LE`/`GE`
on ARM64.

---

## 21. Bitfield Operations

Hardware registers often pack multiple fields into a single word. KernRift
provides builtins for bit-level manipulation:

| Builtin | Signature | Operation |
|---------|-----------|-----------|
| `bit_get(val, bit)` | `(uint64, uint64) -> uint64` | Extract single bit: `(val >> bit) & 1` |
| `bit_set(val, bit)` | `(uint64, uint64) -> uint64` | Set single bit: `val \| (1 << bit)` |
| `bit_clear(val, bit)` | `(uint64, uint64) -> uint64` | Clear single bit: `val & ~(1 << bit)` |
| `bit_range(val, start, width)` | `(uint64, uint64, uint64) -> uint64` | Extract bit range: `(val >> start) & ((1 << width) - 1)` |
| `bit_insert(val, start, width, bits)` | `(uint64, uint64, uint64, uint64) -> uint64` | Insert bit range: `(val & ~(mask << start)) \| ((bits & mask) << start)` |

Example:

```kr
uint64 cr0 = 0
cr0 = bit_set(cr0, 0)          // set PE (Protection Enable)
cr0 = bit_set(cr0, 31)         // set PG (Paging)
uint64 pe = bit_get(cr0, 0)    // 1

uint64 flags = bit_range(status_reg, 4, 4)   // extract bits 7:4
status_reg = bit_insert(status_reg, 4, 4, 0xF) // set bits 7:4 to 0xF
```

---

## 22. Volatile Blocks

`volatile { ... }` is syntactically and semantically identical to `unsafe { ... }`
in the current compiler (no optimizer to defeat). It serves as documentation of
intent — the programmer is performing a memory-mapped I/O access that must not
be elided or reordered by future optimization passes:

```kr
volatile { *(mmio_base as uint32) -> status }
volatile { *(mmio_base as uint32) = 0x01 }
```

The cast type determines the access width.  Supported types are `uint8`,
`uint16`, `uint32`, and `uint64`:

```kr
volatile { *(port_base as uint16) -> reg16 }
volatile { *(port_base as uint16) = 0x1234 }
```

### Volatile barriers

`volatile_barrier()` emits a compiler-level ordering fence.  It prevents the
compiler from reordering memory accesses across the barrier without emitting a
hardware fence instruction:

```kr
volatile { *(mmio_base as uint32) = 0x01 }
volatile_barrier()
volatile { *(mmio_base as uint32) -> status }
```

For hardware-level ordering, use inline assembly barriers (`dmb`, `dsb` on
ARM64, or `mfence` on x86_64) instead.

---

## 23. Freestanding Mode

`krc --freestanding` produces a binary suitable for bare-metal environments:

- No `_start` trampoline (no argc/argv setup)
- No automatic `exit(0)` at end of `main`
- No OS-specific syscall wrappers

```sh
krc --freestanding --arch=x86_64 kernel.kr -o kernel.elf
```

Use this for kernel entry points, bootloaders, and embedded firmware where the
runtime environment is set up by the programmer.

### Stack size warnings

The compiler emits a warning to stderr when a function's stack frame exceeds
4096 bytes. This catches accidental large local allocations that could overflow
a kernel stack:

```
warning: large stack frame (13696 bytes) in function 'gen_expr'
```

---

## 24. Port I/O Intrinsics

KernRift provides built-in intrinsics for x86 port-mapped I/O.  These emit
native `IN` and `OUT` instructions directly — no `extern fn` declarations,
no C FFI, and no runtime overhead.

### Available intrinsics

| Intrinsic             | Width | x86 instruction        |
|-----------------------|-------|------------------------|
| `inb(port) -> uint8`  | 8-bit | `IN AL, DX`           |
| `outb(port, val)`     | 8-bit | `OUT DX, AL`          |
| `inw(port) -> uint16` | 16-bit | `IN AX, DX`          |
| `outw(port, val)`     | 16-bit | `OUT DX, AX`         |
| `ind(port) -> uint32` | 32-bit | `IN EAX, DX`         |
| `outd(port, val)`     | 32-bit | `OUT DX, EAX`        |

The `port` argument is a `uint16`.  The `val` argument must match the
intrinsic width.

### Example

```kr
@module_caps(Ioport)

@ctx(thread, boot) @eff(ioport)
fn serial_write(uint8 b) {
    // Wait for transmit-holding register empty
    uint8 lsr = inb(0x3FD)
    while (lsr & 0x20) == 0 {
        lsr = inb(0x3FD)
    }
    outb(0x3F8, b)
}
```

### Platform restriction

Port I/O intrinsics are **x86_64 only**.  Compiling a file that uses `inb`,
`outb`, `inw`, `outw`, `ind`, or `outd` with `--arch arm64` produces a
compile-time error:

```
error: port I/O intrinsics are x86_64-only (ARM has no port-mapped I/O)
```

---

## 25. Syscall Intrinsic

The `@syscall` intrinsic issues a raw system call to the host kernel.  It is
available in `@ctx(host)` functions and maps to the platform-appropriate
instruction (`syscall` on Linux/macOS x86_64, `svc #0` on AArch64).

### Syntax

```kr
@syscall(nr, arg0, arg1, ...) -> uint64
```

`nr` is the syscall number.  Up to 6 arguments are supported (matching the
SysV and AArch64 calling conventions).

### Platform differences

| Platform          | Instruction | Syscall number source |
|-------------------|-------------|----------------------|
| Linux x86_64      | `syscall`   | `nr` in RAX          |
| Linux AArch64     | `svc #0`    | `nr` in X8           |
| macOS x86_64      | `syscall`   | `nr + 0x2000000`     |
| macOS AArch64     | `svc #0x80` | `nr` in X16          |

### Example

```kr
@ctx(host)
fn host_write(uint32 fd, uint64 buf, uint64 len) -> uint64 {
    // Linux x86_64: write = syscall 1
    uint64 result = @syscall(1, fd, buf, len)
    return result
}
```

In practice, prefer the built-in host functions (`write`, `exec`, etc.)
over raw `@syscall` unless you need a syscall not covered by the built-in
set.

---

## 26. Built-in Host Functions

When a function is annotated with `@ctx(host)`, thirteen built-in functions are
available without any `extern fn` declaration.  The compiler maps these to
`__kr_*` symbols provided by the KernRift host runtime.

### Available functions

| Function                                     | Description                                        |
|----------------------------------------------|----------------------------------------------------|
| `write(fd, buf, len)`                        | Write `len` bytes from `buf` to file descriptor `fd`. |
| `alloc(size) -> uint64`                      | Allocate `size` bytes of memory. Returns a pointer. |
| `dealloc(ptr, size)`                         | Free memory at `ptr` of `size` bytes.              |
| `getenv(name) -> uint64`                     | Look up an environment variable. Returns pointer or 0. |
| `exec(cmd) -> uint32`                        | Execute a shell command. Returns the exit code.    |
| `exit(code)`                                 | Terminate the process with exit code `code`.       |
| `str_copy(dst, src)`                         | Copy a null-terminated string from `src` to `dst`. |
| `str_cat(dst, src)`                          | Append `src` to the end of `dst`.                  |
| `str_len(s) -> uint64`                       | Return the length of null-terminated string `s`.   |
| `get_target_os() -> uint64`                  | Return the target OS identifier (e.g. `1` = Linux, `2` = Windows, `3` = macOS). |
| `get_arch_id() -> uint64`                    | Return the target architecture (`1` = x86-64, `2` = AArch64). |
| `exec_process(path) -> uint32`               | Spawn and wait on an executable at `path`. Returns the exit code. |
| `get_module_path(buf, size) -> uint64`        | Write the path of the current module into `buf` (up to `size` bytes). Returns bytes written. |

### Example

```kr
@module_caps(Env, Process, Stdout)

@export
@ctx(host) @eff(env, process, stdout)
fn main() {
    uint64 msg = "Hello from KernRift host mode\n"
    write(1, msg, str_len(msg))

    uint64 home = getenv("HOME")
    if home != 0 {
        write(1, home, str_len(home))
    }

    uint32 rc = exec("ls -la")
    exit(rc)
}
```

These functions are only available in `@ctx(host)` code.  Using them in
kernel contexts (`boot`, `thread`, `irq`, `nmi`) is a compile-time error.

> **Note:** The compiler performs undeclared identifier detection during
> semantic analysis.  Any call to a name that is not a declared function,
> a built-in host function, or an imported symbol is reported as an error
> before code generation begins.

### Module capabilities

Host functions require the appropriate module capabilities:

| Capability | Required for                        |
|------------|-------------------------------------|
| `Stdout`   | `write`                             |
| `Env`      | `getenv`, `get_target_os`, `get_arch_id` |
| `Process`  | `exec`, `exec_process`, `exit`      |
| `Fs`       | `get_module_path`                   |

`alloc`, `dealloc`, `str_copy`, `str_cat`, and `str_len` do not require
additional capabilities beyond `@ctx(host)`.

---

## 27. Atomic Builtins

KernRift provides built-in functions for lock-free atomic operations on memory.
These map directly to hardware atomic instructions (`LOCK` prefix on x86_64,
`LDXR`/`STXR` sequences on ARM64).

### Compare-and-swap

```kr
uint64 ok = atomic_cas(addr, expected, desired)
```

Atomically compares the value at `addr` with `expected`.  If they match,
stores `desired` and returns `1`.  Otherwise returns `0` and leaves memory
unchanged.

### Atomic load and store

```kr
uint64 val = atomic_load(addr)
atomic_store(addr, value)
```

`atomic_load` performs a sequentially-consistent load.
`atomic_store` performs a sequentially-consistent store.

### Atomic arithmetic

```kr
uint64 old = atomic_add(addr, delta)
uint64 old = atomic_sub(addr, delta)
```

Returns the previous value at `addr` before the operation.

### Atomic bitwise

```kr
uint64 old = atomic_and(addr, mask)
uint64 old = atomic_or(addr, mask)
uint64 old = atomic_xor(addr, mask)
```

Returns the previous value at `addr` before the operation.

### Example: spinlock

```kr
static uint64 lock_val = 0

fn spin_lock() {
    while atomic_cas(lock_val, 0, 1) == 0 {
        asm("pause")   // x86_64 hint
    }
}

fn spin_unlock() {
    atomic_store(lock_val, 0)
}
```

---

## 28. Slice Indexing

Slices support element access via bracket notation.  Both reads and writes
are supported.

### Read

```kr
fn first_byte([uint8] data) -> uint8 {
    uint8 b = data[0]
    return b
}
```

`data[i]` loads the element at byte offset `i * sizeof(T)` from the slice
base pointer.  No bounds check is performed at runtime (freestanding
environment — the caller is responsible for ensuring `i < data.len`).

### Write

```kr
fn zero_first([uint8] buf) {
    buf[0] = 0
}
```

`buf[i] = val` stores `val` at the computed offset.  The slice must refer
to writable memory.

### Indexing with variables

The index expression can be any integer expression:

```kr
fn fill([uint8] buf, uint64 n, uint8 val) {
    uint64 i = 0
    while i < n {
        buf[i] = val
        i += 1
    }
}
```

---

## 29. Compiler CLI Reference

### Default compilation

```sh
krc <file.kr>                        # compile to <stem>.krbo (native arch)
krc --version
```

### Check and analyse

```sh
krc check <file.kr>
krc check --surface stable <file.kr>
krc check --surface experimental <file.kr>
krc check --profile kernel <file.kr>
krc check --policy <policy.toml> <file.kr>
krc check --contracts-out <contracts.json> <file.kr>
krc link <file1.kr> [file2.kr ...]   # cross-file lock-cycle check
```

The `--surface` flag controls which language features are accepted:

| Surface       | Meaning                                       |
|---------------|-----------------------------------------------|
| `stable`      | Default. All stable features.                 |
| `experimental`| Enables proposals under active development.   |

### Emit binary artifacts

```sh
krc --emit=krbofat -o out.krbo <file.kr>        # fat binary (x86_64 + arm64)
krc --emit=krbo    -o out.krbo <file.kr>        # native-arch .krbo
krc --emit=krboexe -o out.krbo <file.kr>        # self-contained .krbo (single-arch)
krc --emit=elfobj  -o out.o    <file.kr>        # ELF relocatable object
krc --emit=elfobj  --arch arm64 -o out.o <file.kr>
krc --emit=asm     -o out.s    <file.kr>        # textual assembly
krc --emit=asm     --arch arm64 -o out.s <file.kr>
krc --emit=staticlib -o out.a  <file.kr>        # static library archive (no ar needed)
krc --emit=elfexe  -o out     <file.kr>        # native ELF executable (no ld needed)
krc --emit=hostexe -o build   <file.kr>        # native host executable (no cc needed)
```

Optionally emit a sidecar metadata JSON alongside the binary:

```sh
krc --emit=krbo -o out.krbo --meta-out out.json <file.kr>
```

### Introspection

```sh
krc --emit krir      <file.kr>   # dump KRIR IR as JSON
krc --emit lockgraph <file.kr>   # dump lock graph as JSON
krc --emit caps      <file.kr>   # dump capability manifest as JSON
krc --emit contracts <file.kr>   # dump semantic contracts as JSON
krc inspect --contracts <contracts.json>
krc inspect-artifact <artifact>
krc inspect-artifact <artifact> --format json
```

### Contracts and verification

```sh
krc check --contracts-out contracts.json \
    --hash-out contracts.sha256 \
    --sign-ed25519 secret.hex \
    --sig-out contracts.sig \
    driver.kr

krc verify --contracts contracts.json \
    --hash contracts.sha256 \
    --sig contracts.sig \
    --pubkey pubkey.hex
```

### Living compiler (automated style fixes)

The living compiler (`lc`) analyses a source file for non-canonical annotation
spellings and can rewrite them automatically:

```sh
krc lc <file.kr>             # report non-canonical annotations
krc lc --fix --dry-run <file.kr>   # preview changes as a diff
krc lc --fix --write <file.kr>     # apply changes in-place
krc lc --ci --min-fitness 70 <file.kr>   # CI mode: fail below fitness score
```

### Canonical form

```sh
krc check --canonical <file.kr>
krc migrate <file.kr>        # rewrite to canonical annotation form
krc migrate <file.kr> --dry-run
```

### Running compiled programs

```sh
./output                          # run the compiled binary
```

The `kernrift` runtime maps the UART buffer, copies the code to executable
memory, runs the entry function, and flushes `print()` output to stdout.

---

## 30. Adaptive Surface & Living Compiler

KernRift's compiler has a built-in mechanism for evolving the language without
breaking existing code.  Every attribute alias or shorthand goes through a
lifecycle tracked by the compiler itself: `experimental` → `stable` →
`deprecated`.  This section documents how to interact with that system.

### The `#lang` directive

A source file may declare its required surface profile at the very first line:

```kr
#lang stable
```

```kr
#lang experimental
```

This overrides the `--surface` flag passed to the compiler for that file.  Use
it to pin production kernel code to `stable` while keeping experimental
feature work in separate files.

You can also pin a specific language version:

```kr
#lang 1.0
```

The compiler rejects files that require a version higher than its own
(`CURRENT_LANG_VERSION = 1.0`).  This prevents silently compiling code with a
toolchain that does not fully understand it.

### Surface profiles

| Profile        | Meaning                                                          |
|----------------|------------------------------------------------------------------|
| `stable`       | Default. All promoted features. Suitable for production code.    |
| `experimental` | Also allows features tagged `Experimental`. Not for shipping.    |

Pass `--surface experimental` on the command line, or write `#lang experimental`
in the source file.

### Adaptive feature aliases

Adaptive features are ergonomic aliases for verbose canonical forms.  Each has
a lifecycle status and a safe mechanical replacement.

| Alias            | Expands to          | Status       | Profile gate   |
|------------------|---------------------|--------------|----------------|
| `@irq_handler`   | `@ctx(irq)`         | Experimental | experimental   |
| `@thread_entry`  | `@ctx(thread)`      | Stable       | stable         |
| `@may_block`     | `@eff(block)`       | Experimental | experimental   |
| `@irq_legacy`    | `@ctx(irq)`         | Deprecated   | —              |

Using an alias outside its gate produces a compiler error.  Using a deprecated
alias always produces an error regardless of surface profile.

The compiler also accepts legacy shorthand attributes inherited from earlier
syntax revisions (`@irq`, `@noirq`, `@alloc`, `@block`, `@preempt_off`).
These are always accepted and lower to their canonical equivalents.

### Inspecting available features

List the active feature aliases for a profile:

```sh
krc features --surface stable
krc features --surface experimental
krc features --surface stable --json
```

### The `proposals` subcommand

Proposals track the promotion lifecycle of every adaptive feature.  Use them
to audit the compiler's own roadmap:

```sh
# List all proposals with their current status.
krc proposals

# Validate internal consistency of the proposal table.
krc proposals --validate

# Show which proposals are ready to promote to the next lifecycle stage.
krc proposals --promotion-readiness

# Preview what promoting a feature would change (dry run + diff).
krc proposals --promote irq_handler_alias --dry-run --diff

# Perform the promotion (updates the compiler's internal table).
krc proposals --promote irq_handler_alias
```

Promoting a feature means advancing it from `Experimental` → `Stable`, or
`Stable` → `Deprecated`.  The `--dry-run` flag prints what would change
without writing anything.

### The living compiler (`lc`)

The living compiler analyses a source file and produces fitness scores and
fixup suggestions.  It is the primary tool for adopting the latest canonical
style:

```sh
krc lc <file.kr>                      # analyse and report fitness
krc lc --surface experimental <file.kr>
krc lc --ci <file.kr>                 # exit 1 if fitness < threshold
krc lc --ci --min-fitness 70 <file.kr>
krc lc --diff <file.kr>               # show suggested rewrites as a diff
krc lc --fix --dry-run <file.kr>      # preview in-place fixes
krc lc --fix --write <file.kr>        # apply fixes in-place
```

The living compiler scans for patterns such as:

- IRQ handlers that use raw MMIO (`@ctx(irq)` + `raw_mmio_write`)
- High lock depth (call chains that hold many locks simultaneously)
- MMIO access without a lock guard

Each pattern produces a fitness score (0–100).  A score of 100 means the code
is idiomatic and no suggestions are available.

---

## 31. Binary Artifact Formats

### `.krbo` — KernRift binary object

The native binary format produced by `krc`.  A single `.krbo` contains
code for one architecture.  The 16-byte header layout:

| Offset | Size | Field          | Description                         |
|--------|------|----------------|-------------------------------------|
| 0      | 4    | magic          | `KRBO` (ASCII)                      |
| 4      | 1    | arch           | `0x01` = x86-64, `0x02` = AArch64  |
| 5      | 1    | abi            | `0x01` = SysV                       |
| 6      | 2    | reserved       | Must be zero                        |
| 8      | 4    | code\_length   | Byte length of the code section     |
| 12     | 4    | entry\_offset  | Byte offset of the entry point      |

### `.krbo` fat binary

A fat binary begins with an 8-byte magic `KRBOFATx` and contains multiple
architecture slices.  `kernrift` automatically selects the matching slice for
the host architecture at runtime.

Produce a fat binary with:

```sh
krc --emit=krbofat -o output.krbo source.kr
```

A fat binary is the recommended distribution format: a single file runs on
both x86-64 and ARM64 without recompilation.

### ELF relocatable object

`--emit=elfobj` produces a standard ELF `.o` for linking into an existing
kernel or bootloader build system.  Use `--arch arm64` or `--arch x86_64` to
override the target architecture.

### Static library

`--emit=staticlib` produces an `ar`-format `.a` archive containing the ELF
object.  Suitable for embedding KernRift modules into C/C++ kernel builds.

### Sidecar metadata (`.json`)

`--meta-out <path>` emits a JSON file alongside the binary that records the
artifact kind, surface profile, input path, byte length, and SHA-256 hash of
the binary.  Verify a binary against its sidecar at any later point:

```sh
krc verify-artifact-meta <artifact> <meta.json>
krc verify-artifact-meta --format json <artifact> <meta.json>
```

---

## Full example

A complete UART send driver plus a `print`-based hello path:

```kr
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}

lock UartLock

const uint32 TX_READY = 0x20

@ctx(thread, boot)
fn uart_ready() -> bool {
    uint32 s = UART0.Status
    return (s & TX_READY) != 0
}

@ctx(thread, boot)
fn uart_send(uint8 b) {
    acquire(UartLock)
    uint32 i = 0
    while i < 1000 {
        if uart_ready() {
            UART0.Data = b
            release(UartLock)
            return
        }
        i += 1
    }
    release(UartLock)
}

@ctx(thread, boot)
fn entry() {
    print("KernRift boot\n")
    uart_send(0x0A)            // newline over UART
}
```

---

*See `docs/ARCHITECTURE.md` for compiler internals and `docs/KRIR_SPEC.md`
for the intermediate representation specification.*
