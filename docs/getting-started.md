# Getting Started with KernRift

## Prerequisites

- **Rust 1.93.1** — install via [rustup](https://rustup.rs). Once you clone this repo, `rust-toolchain.toml` auto-selects the correct version; no manual pinning needed.
- **Cargo** — bundled with Rust

> **Do not use `apt install cargo` / `apt install rustc` on Ubuntu/Debian.** The distro-packaged Rust is several major versions behind and will fail with a `feature 'edition2024' is required` error. Always install via rustup.rs.

## Install

### Quick install (recommended)

These scripts handle everything: installing rustup if needed, removing conflicting system Rust, and placing both `kernriftc` and `kernrift` on your PATH.

**Linux**
```sh
bash <(curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/scripts/install-linux.sh)
```

**macOS**
```sh
bash <(curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/scripts/install-macos.sh)
```

**Windows** (PowerShell, run as your user — no admin needed)
```powershell
powershell -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/Pantelis23/KernRift/main/scripts/install-windows.ps1 | iex"
```

### From source (manual)

If you already have Rust 1.93.1+ via rustup:

```sh
git clone https://github.com/Pantelis23/KernRift
cd KernRift
cargo install --path crates/kernriftc
```

This builds and installs both `kernriftc` and `kernrift` to `~/.cargo/bin/` (Linux/macOS) or `%USERPROFILE%\.cargo\bin\` (Windows).

### Prebuilt binary — Linux / macOS

Download from the [Releases page](https://github.com/Pantelis23/KernRift/releases) and verify:

```sh
# Download
curl -L -o kernriftc https://github.com/Pantelis23/KernRift/releases/latest/download/kernriftc-linux-x86_64
curl -L -o kernriftc.sha256 https://github.com/Pantelis23/KernRift/releases/latest/download/kernriftc-linux-x86_64.sha256

# Verify
sha256sum --check kernriftc.sha256

# Install
chmod +x kernriftc
sudo mv kernriftc /usr/local/bin/
```

### Prebuilt binary — Windows

```powershell
# Download
Invoke-WebRequest -Uri "https://github.com/Pantelis23/KernRift/releases/latest/download/kernriftc-windows-x86_64.exe" -OutFile kernriftc.exe
Invoke-WebRequest -Uri "https://github.com/Pantelis23/KernRift/releases/latest/download/kernriftc-windows-x86_64.sha256" -OutFile kernriftc.sha256

# Verify
$expected = (Get-Content kernriftc.sha256).Split(" ")[0]
$actual   = (Get-FileHash kernriftc.exe -Algorithm SHA256).Hash.ToLower()
if ($expected -ne $actual) { Write-Error "SHA256 mismatch!" } else { Write-Host "OK" }

# Add to PATH — move to a directory already on your PATH, e.g.:
Move-Item kernriftc.exe "$env:USERPROFILE\bin\kernriftc.exe"
```

---

## Your First Program

The repo includes `hello.kr`:

```kr
@ctx(thread, boot)
fn entry() {
    print("Hello, World!\n")
}
```

- `@ctx(thread, boot)` — this function may only be called from thread or boot contexts. Calling it from `@ctx(irq)` is a compile error.
- `print("...")` — compiler intrinsic that emits a debug string (maps to a platform debug port in kernel context).

Compile it:

```sh
kernriftc hello.kr
```

On success, `kernriftc` exits 0 and produces `hello.krbo` in the current directory. Run it with:

```sh
kernrift hello.krbo
```

A context violation looks like this:

```kr
@ctx(irq)
fn bad_call() {
    entry();   // error: entry() requires ctx(thread|boot), caller is ctx(irq)
}
```

```
error[E0002]: context mismatch: `entry` requires {thread, boot}, called from {irq}
  --> bad.kr:3:5
```

---

## Command Reference

| Command | Output | Description |
|---------|--------|-------------|
| `kernriftc <file.kr>` | `<stem>.krbo` in CWD | **Default compile** |
| `kernrift <file.krbo>` | — | **Run a compiled program** |
| `kernriftc check <file.kr>` | stderr diagnostics | Analysis only, no binary |
| `kernriftc check --emit=krir <file.kr>` | JSON to **stdout** | KRIR canonical IR |
| `kernriftc check --emit=lockgraph <file.kr>` | JSON to **stdout** | Lock graph analysis |
| `kernriftc check --emit=caps <file.kr>` | JSON to **stdout** | Capabilities manifest |
| `kernriftc check --emit=contracts <file.kr>` | JSON to **stdout** | Signed contracts artifact |
| `kernriftc check --report <metrics> <file.kr>` | JSON to **stdout** | Analysis report |
| `kernriftc verify --contracts <f> --hash <h>` | JSON to **stdout** | Verify artifact hash |
| `kernriftc policy --policy <p> --contracts <c>` | JSON to **stdout** | Policy evaluation |
| `kernriftc inspect-artifact <path>` | JSON/text to **stdout** | Artifact inspection |
| `kernriftc fix ...` | Source edits | Apply canonical fixes |

---

## Next Steps

- [Language Reference](LANGUAGE.md) — types, control flow, annotations, device blocks
- [Architecture](ARCHITECTURE.md) — compiler pipeline, KRIR facts model, pass design
- [examples/](../examples/) — more example programs
- [Contributing](../CONTRIBUTING.md) — build from source, run tests, add features

---
