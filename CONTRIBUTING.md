# Contributing to KernRift

## Prerequisites

- **Bootstrap compiler** — needed only once: `cargo install --git https://github.com/Pantelis23/KernRift-bootstrap kernriftc`
- After first build, `krc` compiles itself — no Rust needed

## Build

```sh
make build       # bootstrap → krc → krc2 (self-compiled)
```

## Test

```sh
make test        # 45 tests (arithmetic, control flow, functions, structs, etc.)
make bootstrap   # verify krc3 == krc4 (fixed point)
```

## Install

```sh
make install     # installs to ~/.local/bin/krc
```

## Source Structure

All compiler source is in `src/`:

| File | Purpose |
|------|---------|
| `lexer.kr` | Tokenizer |
| `parser.kr` | Parser (recursive descent + Pratt) |
| `codegen.kr` | x86_64 code generation |
| `codegen_aarch64.kr` | AArch64 code generation |
| `analysis.kr` | Safety passes |
| `living.kr` | Living compiler (7 patterns, CI gating) |
| `format_*.kr` | Output formats (ELF, Mach-O, PE, AR, KRBO) |
| `main.kr` | CLI and compilation driver |

## Guidelines

- The compiler must always self-compile to a fixed point
- Run `make bootstrap` before submitting changes
- All tests must pass (`make test`)
- No external dependencies — the compiler is fully self-contained
