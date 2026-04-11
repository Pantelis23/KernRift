# KernRift Language Support

Language support for the [KernRift](https://github.com/Pantelis23/KernRift) systems programming language — version-locked to the compiler.

## Features

- **Syntax highlighting** for `.kr` files
- **File icon** (blue cracked K) in the explorer and tabs
- **LSP** powered by `krc check` — diagnostics, completions, hover docs, go-to-definition
- Short type aliases: `u8`/`u16`/`u32`/`u64`, `i8`/`i16`/`i32`/`i64`
- Long type forms: `uint8`..`int64`
- v2.6 builtins highlighted as built-in functions:
  - **Pointer ops**: `load8`/`load16`/`load32`/`load64`, `store8/16/32/64`
  - **Volatile ops**: `vload8/16/32/64`, `vstore8/16/32/64`
  - **String output**: `print_str`, `println_str`
  - **Atomics**: `atomic_load`, `atomic_store`, `atomic_cas`, `atomic_add/sub/and/or/xor`
  - **Bitfield**: `bit_get`, `bit_set`, `bit_clear`, `bit_range`, `bit_insert`
  - **Signed compare**: `signed_lt`, `signed_gt`, `signed_le`, `signed_ge`
  - **Platform**: `get_target_os`, `get_arch_id`, `syscall_raw`, `exec_process`
- **Device blocks** for MMIO: `device NAME at ADDR { FIELD at OFF : TYPE rw }`
- **Static/struct arrays**: `static u8[N] name`, `Point[10] pts`
- **Slice parameters**: `fn foo([u8] data)` with `data.len`
- **`#lang`** directive highlighting for `#lang stable` / `#lang experimental`
- **Method syntax**: `fn Point.sum(Point self) -> u64`
- Annotations: `@export`, `@noreturn`, `@naked`, `@packed`, `@section("name")`
- String/char literals with escape sequences
- Line (`//`) and block (`/* */`) comments
- Auto-closing brackets, indentation, folding

## About KernRift

KernRift is a self-hosted, bare-metal systems language. It compiles itself in ~60ms, produces fat binaries with 7 platform slices, and runs on Linux, Windows, macOS, and Android on x86_64 and ARM64 — without any C toolchain, runtime, or libc.

- [GitHub](https://github.com/Pantelis23/KernRift)
- [Website](https://kernrift.org)
- [Language Reference](https://github.com/Pantelis23/KernRift/blob/main/docs/LANGUAGE.md)
