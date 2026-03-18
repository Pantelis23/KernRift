# KR0 Canonical Authoring Reference

This is the compact authoring guide for writing KR0 in the canonical surface that the repository teaches by default.
It is intentionally narrower than the full grammar/spec documents and only shows syntax that the compiler accepts today on current `main`.

Use this document when you want copyable templates.
Use `docs/spec/krir-v0.1.md` when you need the full accepted-surface grammar and KRIR semantics.

## Quick Rules

- Prefer `@ctx(...)`, `@eff(...)`, `@caps(...)`, and `@module_caps(...)`.
- Prefer `critical { ... }` for block-scoped critical regions.
- Use `extern @ctx(...) @eff(...) @caps() fn ...;` for every extern declaration.
- Use structured MMIO declarations before symbolic MMIO reads/writes.
- Keep `raw_mmio_*` visually loud and gated behind `@module_caps(MmioRaw)`.
- Alias fixtures under `tests/living_compiler/*alias*.kr` are compatibility locks, not preferred authoring style.
- Only empty parameter lists `()` are supported today.

## Canonical Function Forms

Minimal function:

```kr
fn entry() { }
```

Function with explicit context and capability facts:

```kr
@ctx(thread, boot)
@caps(PhysMap)
fn map_io() { helper(); }
```

Function with explicit effect fact:

```kr
@ctx(thread)
@eff(block)
fn worker() { blockpoint(); }
```

## Canonical Extern Form

Extern declarations should always spell out context, effects, and capabilities explicitly, even when a set is empty:

```kr
extern @ctx(thread, boot) @eff(block) @caps() fn sleep();
```

Extern with a privileged capability:

```kr
extern @ctx(thread, boot, irq) @eff(mmio) @caps(PhysMap) fn map_io();
```

## Canonical Context, Effect, and Capability Facts

Canonical frontend facts:

- `@ctx(thread)`
- `@ctx(thread, boot)`
- `@ctx(irq)`
- `@eff(block)`
- `@eff(alloc)`
- `@eff(yield)`
- `@caps()`
- `@caps(PhysMap)`
- `@module_caps(PhysMap)`
- `@module_caps(MmioRaw)`

Copyable minimal module with module caps plus function facts:

```kr
@module_caps(PhysMap);

@ctx(thread, boot)
@caps(PhysMap)
fn entry() { }
```

## Canonical MMIO Declaration and Use

Structured MMIO base and register declarations:

```kr
mmio UART0 = 0x1000;
mmio_reg UART0.DR = 0x00 : u32 rw;
mmio_reg UART0.SR = 0x04 : u32 ro;
mmio_reg UART0.CR = 0x08 : u16 rw;
```

Structured MMIO use:

```kr
fn entry() {
  mmio_read<u32>(UART0 + 0x04);
  mmio_write<u32>(UART0 + 0x00, value);
  mmio_write<u16>(UART0 + 0x08, 0xff);
}
```

Raw MMIO escape hatch:

```kr
@module_caps(MmioRaw);
mmio UART0 = 0x1000;

fn entry() {
  raw_mmio_write<u32>(0x1014, x);
}
```

Notes:

- `mmio NAME = INT_LITERAL;` and `mmio_reg BASE.REG = INT_LITERAL : TYPE ACCESS;` are module-scope declarations.
- Typed MMIO uses `T in {u8,u16,u32,u64}`.
- `addr` must be one of:
  - identifier
  - integer literal
  - identifier + integer literal
- `value` must be one of:
  - identifier
  - integer literal

## Canonical Critical and Yield Usage

Block-scoped critical region:

```kr
fn entry() {
  critical {
    acquire(LockA);
    release(LockA);
  }
}
```

Explicit yield point:

```kr
@ctx(thread)
fn pump() {
  yieldpoint();
}
```

Whole-function critical attribute remains distinct from the block form:

```kr
@critical
fn entry() { helper(); }
```

Use `@critical` only when the whole function is meant to be treated as critical.
Use `critical { ... }` when only a local region is meant to be critical.

## Common Mistakes -> Canonical Replacement

| Mistake | Canonical replacement |
|---|---|
| `@thread_entry` | `@ctx(thread)` |
| `@irq_handler` | `@ctx(irq)` |
| `@may_block` | `@eff(block)` |
| `@irq_legacy` | `@ctx(irq)` |
| `extern fn sleep();` | `extern @ctx(...) @eff(...) @caps() fn sleep();` |
| `@yieldpoint` | `yieldpoint()` |
| `mmio_read()` | `mmio_read<T>(addr)` |
| `mmio_write()` | `mmio_write<T>(addr, value)` |
| `raw_mmio_write()` without module cap | `@module_caps(MmioRaw);` plus `raw_mmio_write<T>(addr, value)` |
| using `@critical` for a local region | `critical { ... }` |

## Alias Fixture Note

Files under `tests/living_compiler/*alias*.kr` intentionally preserve accepted alias behavior such as `@thread_entry`, `@irq_handler`, and `@may_block`.
They are regression locks for compatibility behavior, not preferred style guides for new KR0 source.
