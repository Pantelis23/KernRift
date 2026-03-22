# KernRift Syntax Redesign — Design Spec

**Date:** 2026-03-21
**Status:** Approved
**Scope:** Full surface syntax redesign — new parser, no backward compatibility required

---

## 1. Goals

- Replace the current low-level syntax (`stack_cell`, `branch_if_zero`, `cell_add`, etc.) with a clean, readable surface language
- Promote all experimental features to stable
- Style: C/Python hybrid — braces for blocks, no semicolons, no parens required on conditions
- Easy to learn for any developer regardless of background
- Keep KernRift's unique strengths: `@ctx`/`@eff`/`@caps` annotations, compile-time safety model, MMIO as first-class

---

## 2. Type System

### 2.1 Primitive Types

| Type | Size | Backing KRIR type | Notes |
|------|------|-------------------|-------|
| `uint8` | 8-bit | `U8` | Unsigned |
| `uint16` | 16-bit | `U16` | Unsigned |
| `uint32` | 32-bit | `U32` | Unsigned |
| `uint64` | 64-bit | `U64` | Unsigned |
| `int8` | 8-bit | `U8` (bitwise) | Signed — see §2.5 |
| `int16` | 16-bit | `U16` (bitwise) | Signed — see §2.5 |
| `int32` | 32-bit | `U32` (bitwise) | Signed — see §2.5 |
| `int64` | 64-bit | `U64` (bitwise) | Signed — see §2.5 |
| `bool` | 8-bit | `U8` (0 or 1) | See §2.6 |
| `char` | 8-bit | `U8` | ASCII 0–127 |
| `float32` | 32-bit | `F32` (SSE2) | IEEE 754 single precision |
| `float64` | 64-bit | `F64` (SSE2) | IEEE 754 double precision |
| `float16` | 16-bit | `F16` (storage) | Storage only — arithmetic deferred; convert via `float32` |

### 2.2 Aliases

| Alias | Resolves to | Rationale |
|-------|-------------|-----------|
| `byte` | `uint8` | Common in kernel/driver code |
| `addr` | `uint64` | Pointer-sized address on 64-bit targets |

### 2.3 Composite Types

| Syntax | Meaning |
|--------|---------|
| `[N]T` | Fixed array of N elements of type T |
| `[]T` | Slice — fat pointer `(ptr: uint64, len: uint64)` under SysV ABI |
| `string` | Alias for `[]char` — read-only UTF-8 string |

### 2.4 Void

Functions with no return type implicitly return `void`. The explicit annotation `-> void` is also accepted.

### 2.5 Signed Integer Semantics (V1)

Signed types (`int8`–`int64`) are stored as their unsigned bitwise equivalents in KRIR (`U8`–`U64`). The type checker enforces the distinction at the surface level. Rules for V1:

- `+`, `-`, `*`, `&`, `|`, `^`, `~`, `<<` work identically for signed and unsigned (bitwise same)
- `>>` on signed types emits arithmetic right shift (SAR); on unsigned types emits logical right shift (SHR) — these are separate KRIR ops
- `<`, `>`, `<=`, `>=` on signed types use signed comparison (KRIR `BranchIfLtSigned`); unsigned uses unsigned comparison
- `/` and `%` on signed types are deferred to a future KRIR extension — the compiler emits an error if used
- Negative literals (e.g. `-1`) are accepted for signed types and stored as two's complement

### 2.6 Bool Semantics

`bool` lowers to `uint8`. Values are `true` → `1`, `false` → `0`. A bare `if flag { ... }` where `flag: bool` lowers to `BranchIfZero` on the underlying `U8` slot. The type checker rejects non-bool values in boolean positions.

### 2.7 Float Types

`float32` and `float64` use SSE2 instructions available on every x86_64 CPU. `float16` is a storage-only type — values can be stored and loaded but arithmetic requires converting to `float32` first (F16C extension on x86_64; native on ARM).

Float literals use a decimal point or exponent: `3.14`, `0.5`, `1.0e-3`.

---

## 3. Variables

Variables are declared type-first, consistent with C. No semicolons.

```
TypeKeyword name = expr
TypeKeyword name         // zero-initialized
```

Reassignment:
```
name = expr
name += expr
```

**Disambiguation rule:** A statement beginning with a reserved type keyword (any of: `uint8`, `uint16`, `uint32`, `uint64`, `int8`, `int16`, `int32`, `int64`, `float32`, `float64`, `bool`, `char`, `byte`, `addr`, `string`, or a declared struct/enum name) is always a variable declaration. All type names are reserved keywords and cannot be used as variable or function names. A statement beginning with a non-type identifier is an assignment, function call, or compound statement.

**Parser lookahead rule:** The parser reads token 1. If it is a type keyword → parse declaration (`TypeKeyword ident = expr`). Otherwise, read token 2: if token 2 is `=` or a compound-assignment operator (`+=`, `-=`, etc.) → parse assignment. If token 2 is `(` → parse function call statement. Any other token is a parse error.

**Examples:**
```c
uint32 x = 5
uint8 b = 0xFF
bool ready = false
addr base = 0x10000000
char c = 'A'

x = x + 1
ready = true
base += 4
```

---

## 4. Functions

```
fn name(TypeKeyword param1, TypeKeyword param2) -> ReturnType {
    body
}

fn name(TypeKeyword param) {    // implicit void return
    body
}
```

**Return type** is parsed by the new parser and stored in `FnAst` as a new field:
```rust
pub return_ty: Option<MmioScalarType>   // None = void
```
`None` means void. For extern functions returning a value, the HIR lowering inserts a `call_capture` into `%rax` under SysV ABI. `MmioScalarType` covers all V1 integer types; `bool`/`char`/`byte`/`addr` are resolved to their backing `U8`/`U64` before being stored.

**Examples:**
```c
@ctx(thread, boot)
fn send_byte(uint8 b) {
    while UART0.Status == 0 {}
    UART0.Data = b
}

fn get_status() -> uint32 {
    return UART0.Status
}

@ctx(boot)
fn entry() {
    print("Hello, World!\n")
}
```

---

## 5. Control Flow

### 5.1 If / Else

No parentheses required around conditions.

```c
if x > 0 {
    // ...
} else if x == 0 {
    // ...
} else {
    // ...
}
```

**Lowering:** `if`/`else` blocks are lowered to KRIR `BranchIfZero` / `BranchIfNonZero` ops on a condition slot. The condition expression is evaluated into a temporary slot first, then branched on.

### 5.2 While Loop

```c
while UART0.Status == 0 {}

while x < 10 {
    x = x + 1
}
```

### 5.3 For Loop

```c
for i in 0..10 {
    // i goes from 0 to 9 inclusive
}

for i in 0..=10 {
    // i goes from 0 to 10 inclusive
}
```

**Loop variable type:** The loop variable is always `uint32` in V1. Range bounds must be integer literals or `uint32` expressions — `uint64` upper bounds are a type error in V1. This restriction is lifted when signed/64-bit range support is added.

### 5.4 Return

```c
return value    // return with value
return          // return void
```

### 5.5 Break and Continue

```c
while true {
    if done {
        break
    }
    if skip {
        continue
    }
}
```

### 5.6 Loop Lowering Strategy

Loops (`while` and `for`) are lowered using **native KRIR loop ops**. This is a deliberate backend extension — not a small add-on.

New KRIR ops added in this redesign:

| Op | Description |
|----|-------------|
| `KrirOp::LoopBegin` | Opens a loop scope (corresponds to `while`/`for` header) |
| `KrirOp::LoopEnd` | Closes a loop scope, jumps back to head |
| `KrirOp::LoopBreak` | Exits the innermost loop |
| `KrirOp::LoopContinue` | Jumps to loop condition check |

The x86_64 backend lowers these to conditional jumps:
- `LoopBegin` → emit a label `__loop_N_head`
- condition check → `BranchIfZero` to `__loop_N_end`
- `LoopEnd` → `jmp __loop_N_head`
- `LoopBreak` → `jmp __loop_N_end`
- `LoopContinue` → `jmp __loop_N_head`

**Backend architecture changes required for loops (Phase 4 scope):**
The current backend enforces a single-block linear structure per function (`validate_executable_krir_linear_structure`) and uses function calls (`call then_symbol`) for all branching. To support intra-function jumps, Phase 4 must:
1. Remove or relax the single-block constraint for functions containing loops
2. Add label/jump `X86_64AsmInstruction` variants: `Label(String)`, `Jmp(String)`, `JmpIfZero(String)`, `JmpIfNonZero(String)`
3. Add label emission to the ASM text emitter
4. Add label resolution and `REL32` encoding to the object byte encoder
5. Track nested loop depth for unique label naming (`__loop_N_head`, `__loop_N_end`)

This is the highest-risk implementation phase. It does not change the KRIR semantics or any other pass — only the backend emitter is affected.

Loop variables (e.g. `i` in `for i in 0..10`) are stack-allocated slots in the enclosing function frame. **Local variables in the new syntax are function-scoped stack slots**, visible throughout the function body including inside nested loops.

---

## 6. Operators

### Operator Precedence (high to low, matching C)

| Level | Operators | Associativity | V1 Status |
|-------|-----------|---------------|-----------|
| 1 (highest) | `!` `~` (unary) | Right | Supported |
| 2 | `*` `/` `%` | Left | **Deferred** — no KRIR Mul/Div ops |
| 3 | `+` `-` | Left | Supported |
| 4 | `<<` `>>` | Left | Supported |
| 5 | `<` `>` `<=` `>=` | Left | Supported |
| 6 | `==` `!=` | Left | Supported |
| 7 | `&` | Left | Supported |
| 8 | `^` | Left | Supported |
| 9 | `\|` | Left | Supported |
| 10 | `&&` | Left | Supported |
| 11 (lowest) | `\|\|` | Left | Supported |

`*`, `/`, `%` are reserved tokens in the lexer in V1 but emit a "not yet implemented" compiler error. They are listed in §17 Out of Scope.

### Compound Assignment
`+=`  `-=`  `&=`  `|=`  `^=`  `<<=`  `>>=`

`*=` and `/=` are deferred with `*` and `/`.

### L-value vs R-value

An **l-value** (assignable location) is one of:
- A local variable name: `x`
- A device register field: `UART0.Data` (write, only if declared `rw` or `wo`)
- A slice element: `buf[i]` (write)

An **r-value** (readable expression) is one of:
- An integer, bool, or char literal
- A local variable name
- A device register field: `UART0.Status` (read, only if declared `rw` or `ro`)
- A function call result: `get_status()`
- A binary/unary expression
- A slice element: `buf[i]` (read)
- A string literal: `"text"`
- A slice length: `buf.len`

Assigning to an `ro` register or reading from a `wo` register is a compile-time error.

---

## 7. Structs and Enums

### Structs
Fields are type-first, packed layout, no padding.

```c
struct UartConfig {
    uint32 baud
    uint8 bits
    bool enabled
}
```

Field offset: `UartConfig::baud` (compile-time constant). Instantiation and field access are deferred to a future pass — structs are used as layout descriptors for MMIO overlays in V1.

### Enums
```c
enum UartMode: uint8 {
    Normal   = 0
    Loopback = 1
    Test     = 2
}
```

Variant reference: `UartMode::Normal`.

---

## 8. Constants

Constants use `const uint32 NAME = value` (type-first, matching variable style) for consistency.

```c
const uint32 MAX_BAUD  = 115200
const addr   BASE_ADDR = 0x3F000000
```

---

## 9. Comments

```c
// Single line comment

/*
   Multi-line comment
*/
```

---

## 10. Kernel-Specific Features

### 10.1 Module Capabilities

```c
@module_caps(MmioRaw)
@module_caps(Mmio, Ioport)
```

No trailing semicolon (consistent with removal of semicolons from language).

### 10.2 Device / MMIO Declarations

Replaces `mmio NAME = ...` and `mmio_reg BASE.REG = OFFSET : TYPE ACCESS`.

```c
device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
    Ctrl   at 0x08 : uint32 rw
}
```

**Lowering:** The parser emits one `MmioBaseDecl { name: "UART0", addr: 0x3F000000 }` and N `MmioRegisterDecl { base: "UART0", reg: "Data", offset: 0x00, ty: U8, access: RW }` entries — identical to what the current parser emits for `mmio`/`mmio_reg`. No new HIR nodes needed.

**Device symbol resolution pass:** After parsing, build a register map:
```
(device_name: &str, reg_name: &str) → (base_addr: u64, offset: u64, ty: MmioScalarType, access: Access)
```

When lowering an expression `DEVICE.REG`:
- If on the left of `=` → emit `MmioWrite { addr: IdentPlusOffset { base, offset }, ty, value }`; error if access is `ro`
- If on the right → emit `MmioRead { addr: IdentPlusOffset { base, offset }, ty }` into a fresh slot; error if access is `wo`

Raw MMIO (requires `@module_caps(MmioRaw)`):

```c
raw_write<uint8>(0x10000000, 0x48)
uint8 val = raw_read<uint8>(0x10000000)
```

### 10.3 Function Annotations

All annotations are unchanged — they are a core KernRift differentiator.

```c
@ctx(thread, boot, irq, nmi)
@eff(mmio, alloc, block, yield, preempt_off, ioport, dma_map)
@caps(PhysMap, PageTableWrite, IrqRoute, IoPort, Mmio, DmaMap)
@noyield
@hook(sched_in)
@hook(sched_out)
```

### 10.4 Locks

`spinlock` keyword is renamed to `lock`. The `ModuleAst.spinlocks: Vec<String>` field is renamed to `ModuleAst.locks: Vec<String>` in the new parser. All HIR lowering code that reads `ast.spinlocks` is updated to `ast.locks`.

```c
lock UartLock       // module-level declaration

@ctx(thread)
fn send_safe(uint8 b) {
    acquire(UartLock)
    UART0.Data = b
    release(UartLock)
}
```

Lock-order cycle detection remains a compile-time check.

### 10.5 Per-CPU Variables

```c
percpu cpu_ticks: uint32        // declaration (colon syntax intentional for declarations)

percpu_read<uint32>(cpu_ticks, val)
percpu_write<uint32>(cpu_ticks, val)
```

The explicit type annotation on `percpu_read`/`percpu_write` is required in V1. Type inference from the declaration is deferred.

### 10.6 Extern Functions

```c
extern @ctx(irq) @eff() @caps() fn irq_ack()
extern @ctx(thread) @eff(alloc) fn kmalloc(uint64 size) -> addr
```

The new parser adds return type parsing to `FnAst`. For `extern` functions returning a value, the calling convention follows SysV ABI: return value in `%rax`.

### 10.7 Critical Sections and Unsafe

```c
critical {
    // preemption/interrupts disabled
}

unsafe {
    // explicit escape hatch
}
```

### 10.8 Yield Point

```c
yieldpoint()    // voluntarily yield CPU; only in @ctx(thread, boot)
```

### 10.9 Scheduler Hooks

```c
@hook(sched_in) @noyield
fn on_sched_in() {
    percpu_write<uint32>(cpu_ticks, 0)
}
```

---

## 11. String Literals and Print

`print` is a compiler intrinsic that writes a string literal to the UART buffer at `KERN_UART_BASE` (0x10000000). It lowers to a sequence of `RawMmioWrite<U8>` ops — one per character plus a null terminator.

```c
print("Hello, World!\n")
```

The `krboexe` startup stub maps the UART page and flushes the buffer to stdout on exit.

**String local variables:** A `string` local variable may only hold a string literal in V1. Its `.len` is a compile-time constant known to the compiler:
```c
string s = "hello"
uint32 l = s.len    // compile-time constant 5 — valid in V1
```

**String indexing** (`s[i]`) requires a runtime-address load op not currently in KRIR. Deferred to V2. In V1, strings/slices received as function parameters are pass-through only — passed to extern functions or have `.len` read.

---

## 12. Character Literals

Single-quoted ASCII characters are supported in the lexer and lower to their numeric value:

```c
char c = 'A'    // lowers to uint8 slot with value 65
byte b = '\n'   // lowers to uint8 slot with value 10
```

Supported escape sequences:

| Sequence | Meaning | Byte value |
|----------|---------|------------|
| `\n` | Newline | 0x0A |
| `\r` | Carriage return | 0x0D |
| `\t` | Horizontal tab | 0x09 |
| `\b` | Backspace | 0x08 |
| `\a` | Bell/alert | 0x07 |
| `\f` | Form feed | 0x0C |
| `\v` | Vertical tab | 0x0B |
| `\\` | Backslash | 0x5C |
| `\'` | Single quote | 0x27 |
| `\"` | Double quote | 0x22 |
| `\0` | Null | 0x00 |
| `\xHH` | Hex byte literal | e.g. `\xFF` = 255 |

Octal escapes (`\ooo`) are not supported.

---

## 13. Full Example — Hello World

```c
@module_caps(MmioRaw)

@ctx(boot)
fn entry() {
    print("Hello, World!\n")
}
```

---

## 14. Full Example — UART Driver

```c
@module_caps(Mmio)

device UART0 at 0x3F000000 {
    Data   at 0x00 : uint8  rw
    Status at 0x04 : uint32 ro
}

lock UartLock

@ctx(thread, boot)
fn uart_send(uint8 b) {
    acquire(UartLock)
    while UART0.Status == 0 {}
    UART0.Data = b
    release(UartLock)
}
```

---

## 15. Promoted Features (Previously Experimental)

| Feature | Old syntax | New syntax |
|---------|-----------|------------|
| Arithmetic ops | `cell_add<T>(dst, src)` | `dst += src` |
| Function call with args | `call_with_args(fn, a, b)` | `fn(a, b)` |
| Tail call | `tail_call(fn, [args])` | `return fn(args)` |
| Comparisons | `branch_if_eq(s, v, t, f)` | `if s == v { ... }` |
| Multiple locals | `stack_cell<T>(x); stack_cell<T>(y)` | `uint32 x = 0` / `uint32 y = 0` |
| Spinlock renamed | `spinlock NAME` | `lock NAME` |

---

## 16. Implementation Phases

| Phase | Work |
|-------|------|
| 1 — Lexer | New token set: type keywords, no-semicolon stream, char literals `'A'`, `..` and `..=` range operators |
| 2 — Parser | type-first variables, if/else/while/for, device blocks, `fn` with return types, infix expressions with precedence climbing |
| 3 — HIR lowering | device → MmioBaseDecl + MmioRegisterDecl; `DEVICE.REG` symbol resolution; if/else → BranchIfZero; `print` → RawMmioWrite sequence; char literals → integer |
| 4 — KRIR loop ops | Add `LoopBegin`/`LoopEnd`/`LoopBreak`/`LoopContinue` to KRIR; backend lowers to labels + conditional jumps |
| 5 — Operator lowering | Infix `+`/`-`/`&`/`|`/`^`/`<<`/`>>` → existing KRIR arithmetic ops; signed ops (SAR, signed compare) → new KRIR ops |
| 6 — Type checker | New type names, alias resolution, bool/char/signed rules |
| 7 — Docs | Rewrite `docs/LANGUAGE.md` for new syntax |
| 8 — Tests | Update all `.kr` test files to new syntax |

---

## 17. Out of Scope (V1)

- `*`, `/`, `%`, `*=`, `/=` operators (no KRIR `Mul`/`Div` ops — reserved tokens, emit error)
- `float32` / `float64` arithmetic (type names reserved, emit error if used)
- String/slice indexing `s[i]` (requires new KRIR `LoadFromAddr` op)
- Signed integer division and modulo (`/` and `%` on signed types)
- `uint64` upper bounds in `for` range loops
- Struct instantiation and field access (structs used as layout descriptors only)
- Generic types, traits, polymorphism
- Exception handling
