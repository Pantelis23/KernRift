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

| Platform | Command |
|----------|---------|
| Linux / macOS | `cargo install --git https://github.com/Pantelis23/KernRift --bin kernriftc` |
| Windows | See [Getting Started](docs/getting-started.md#prebuilt-binary----windows) |
| All (prebuilt) | See [Releases](../../releases) |

## Quickstart

```sh
# Write a kernel function
cat > entry.kr << 'EOF'
@module_caps(MmioRaw);

@ctx(thread, boot)
fn entry() {
    raw_write<u8>(0x10000000, 0x48);
}
EOF

# Compile it
kernriftc entry.kr
# → entry.krbo

# Or run the analysis pass only
kernriftc check entry.kr
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

KR0 (facts-only pipeline + artifact emission) is complete. KR1–KR3 (driver subset, kernel module, real OS integration) are in progress. See [KR0_KR3_PLAN.md](docs/KR0_KR3_PLAN.md) for the roadmap.

## License

MIT
