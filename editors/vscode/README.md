# KernRift Language Support

Syntax highlighting for the [KernRift](https://github.com/Pantelis23/KernRift) systems programming language.

## Features

- Syntax highlighting for `.kr` files
- Keywords: `fn`, `struct`, `enum`, `if`, `else`, `while`, `for`, `match`, `import`, `type`, `static`, `unsafe`
- Types: `uint8/16/32/64`, `int8/16/32/64`, `bool`, `char`
- Annotations: `@export`, `@ctx(...)`, `@eff(...)`, `@caps(...)`
- String/char literals with escape sequences
- Hex and decimal numbers
- `true`/`false` constants
- Method syntax: `fn Point.sum(...)`
- Comment support (`//`)
- Auto-closing brackets, indentation, folding

## About KernRift

KernRift is a self-hosted systems language compiler. It compiles itself in 16ms, produces the smallest binaries (396 bytes for hello world), and runs on Linux, Windows, and ARM64.

- [GitHub](https://github.com/Pantelis23/KernRift)
- [Website](https://kernrift.org)
