# Getting Started with KernRift

## Prerequisites: Installing Rust

KernRift requires **Rust 1.93.1+** and **Cargo**. Both come from [rustup](https://rustup.rs) — the official Rust toolchain manager. Once installed, `rust-toolchain.toml` in the repo automatically selects the right version.

> **Do not use your OS package manager to install Rust.** `apt install cargo`, `dnf install rust`, `pacman -S rust`, and similar all ship outdated versions that will fail with `feature 'edition2024' is required`. Always use rustup.

---

### Install rustup on Linux

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When prompted, choose option **1** (default installation). Then reload your shell:

```sh
source "$HOME/.cargo/env"
```

To verify:

```sh
rustc --version   # should print 1.93.x or newer
cargo --version
```

If you already installed Rust via `apt`/`dnf`/`pacman`, remove it first:

```sh
# Debian/Ubuntu
sudo apt remove --purge cargo rustc rustup libstd-rust-dev 'libstd-rust-*'
sudo apt autoremove

# Fedora/RHEL
sudo dnf remove cargo rust

# Arch
sudo pacman -Rs rust cargo
```

Then run the `curl` command above.

---

### Install rustup on macOS

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

If you don't have `curl`, install Xcode Command Line Tools first:

```sh
xcode-select --install
```

If you installed Rust via Homebrew, unlink it first to avoid conflicts:

```sh
brew unlink rust
```

Then reload your shell and verify:

```sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

---

### Install rustup on Windows

1. Download **rustup-init.exe** from [rustup.rs](https://rustup.rs) (the site auto-detects Windows and offers the `.exe` directly).
2. Run it and choose option **1** (default installation).
3. Open a **new** terminal — Cargo's `bin` directory (`%USERPROFILE%\.cargo\bin`) is added to your PATH automatically.

To verify (in a new terminal):

```powershell
rustc --version
cargo --version
```

> **Do not install Rust via winget, choco, or scoop** — those packages lag behind and may conflict with rustup. Use the official installer from rustup.rs.

---

## Install KernRift

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

**Windows** <a name="windows"></a>

Antivirus software (including Bitdefender) blocks PowerShell scripts that download executables, regardless of their content. Use the manual steps below instead — they go through your browser and the official rustup installer, which AV products trust.

1. **Install Rust** — follow [Install rustup on Windows](#install-rustup-on-windows) above if you haven't already.
2. **Open a new terminal** (PowerShell or Command Prompt) and run:

```powershell
cargo install --git https://github.com/Pantelis23/KernRift kernrift kernriftc --locked
```

That's it. Both `kernriftc` and `kernrift` are now on your PATH.

### From source (manual)

If you already have Rust 1.93.1+ via rustup:

```sh
git clone https://github.com/Pantelis23/KernRift
cd KernRift
cargo install --path crates/kernrift
cargo install --path crates/kernriftc
```

This builds and installs `kernrift` and `kernriftc` to `~/.cargo/bin/` (Linux/macOS) or `%USERPROFILE%\.cargo\bin\` (Windows).

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
| `kernriftc lc <file.kr>` | text to **stdout** | Living compiler suggestions |
| `kernriftc lc --format json <file.kr>` | JSON to **stdout** | Same, machine-readable |
| `kernriftc lc --ci <file.kr>` | text to **stdout** | Exit 1 if fitness ≥ 50 |
| `kernriftc lc --diff <file.kr>` | text to **stdout** | New/worsened suggestions vs HEAD |
| `kernriftc lc --fix --dry-run <file.kr>` | unified diff to **stdout** | Preview tail-call fixes |
| `kernriftc lc --fix --write <file.kr>` | confirmation to **stdout** | Apply tail-call fixes |
| `kernriftc living-compiler <file.kr>` | text to **stdout** | Alias for `lc` |

---

## Next Steps

- [Language Reference](LANGUAGE.md) — types, control flow, annotations, device blocks
- [Architecture](ARCHITECTURE.md) — compiler pipeline, KRIR facts model, pass design
- [examples/](../examples/) — more example programs
- [Contributing](../CONTRIBUTING.md) — build from source, run tests, add features

---
