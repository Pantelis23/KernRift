# KernRift

A kernel-first systems language that turns OS invariants into compile-time errors — not boot-time crashes.

Generic systems languages don't model kernel reality. KernRift bakes interrupt contexts, lock ordering, MMIO semantics, and capability requirements directly into the type system. Invalid kernel behaviour fails at compile time.

## Features

- **Context safety** — functions are annotated with allowed execution contexts (`boot`, `thread`, `irq`, `nmi`); invalid call edges are rejected
- **Lock ordering** — deadlock cycles are detected and rejected at compile time
- **MMIO correctness** — hardware register access is typed and volatile-safe
- **Capability gating** — privileged operations require explicit module capability declarations
- **Effect tracking** — allocation, blocking, and yield in disallowed paths are compile errors
- **Signed artifacts** — contracts can be hashed and signed with Ed25519 for supply-chain verification

## Install

| Platform | One-liner |
|----------|-----------|
| Linux | `bash <(curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/scripts/install-linux.sh)` |
| macOS | `bash <(curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/scripts/install-macos.sh)` |
| Windows | `cargo install --git https://github.com/Pantelis23/KernRift --bin kernriftc --bin kernrift --locked` (after [installing rustup](docs/getting-started.md#install-rustup-on-windows)) |
| All (prebuilt) | See [Releases](../../releases) |

See [Getting Started](docs/getting-started.md) for manual install and prebuilt binaries.

## Quickstart

```sh
# Write a kernel function
cat > hello.kr << 'EOF'
@ctx(thread, boot)
fn entry() {
    print("Hello, World!\n")
}
EOF

# Compile — produces a fat binary (x86_64 + ARM64 slices)
kernriftc hello.kr
# → hello.krbo  (KRBOFAT fat binary, LZ4-compressed)

# Run — kernrift detects the host architecture and executes the right slice
kernrift hello.krbo

# Or just analyse (no binary output)
kernriftc check hello.kr
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Getting Started](docs/getting-started.md) | Install, first program, full command reference |
| [Language Reference](docs/LANGUAGE.md) | Complete syntax and type system |
| [Architecture](docs/ARCHITECTURE.md) | Compiler pipeline and design decisions |
| [Contributing](CONTRIBUTING.md) | Build, test, crate map, PR checklist |
| [Changelog](CHANGELOG.md) | Release history |

## Testing

Run the default repo-owned local-safe validation path on 32 GB class machines:

```bash
bash tools/validation/local_safe.sh
```

For the heavier local `hir` coverage and per-crate serialized test steps closer to the CI-style path:

```bash
bash tools/validation/full_serial.sh
```

## Status

KR0–KR3 (facts-only pipeline + artifact emission, driver subset, kernel module, real OS integration) are complete. See [KR0_KR3_PLAN.md](docs/KR0_KR3_PLAN.md) for the roadmap.

## License

MIT © 2025 Pantelis Christou — see [LICENSE](LICENSE) for the full text.
