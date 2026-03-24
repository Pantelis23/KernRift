#!/usr/bin/env bash
set -euo pipefail

REPO="https://github.com/Pantelis23/KernRift"
BINS="kernriftc kernrift"

# ── helpers ───────────────────────────────────────────────────────────────────
info()  { printf '\033[1;34m=>\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m✓\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m!\033[0m  %s\n' "$*" >&2; }
die()   { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── 1. reject distro-packaged Rust ────────────────────────────────────────────
if dpkg -s cargo >/dev/null 2>&1 || dpkg -s rustc >/dev/null 2>&1; then
  warn "apt-managed Rust detected (likely Rust 1.75 or older)."
  warn "KernRift requires Rust 1.93.1 — the distro package is too old."
  echo
  read -rp "Remove apt Rust and continue? [y/N] " answer
  [[ "${answer,,}" == y ]] || die "aborted — remove apt Rust manually and re-run."
  info "Removing apt Rust packages..."
  sudo apt-get remove --purge -y cargo rustc rustup \
    libstd-rust-dev 'libstd-rust-*' 2>/dev/null || true
  sudo apt-get autoremove -y 2>/dev/null || true
fi

# ── 2. install rustup if missing ──────────────────────────────────────────────
if ! command -v rustup >/dev/null 2>&1; then
  info "Installing rustup..."
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain none
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain none
  else
    die "curl or wget is required to install rustup."
  fi
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
else
  ok "rustup already installed: $(rustup --version 2>&1 | head -1)"
fi

# make sure cargo is on PATH for the rest of this script
export PATH="$HOME/.cargo/bin:$PATH"

# ── 3. install KernRift ───────────────────────────────────────────────────────
info "Installing kernriftc and kernrift from $REPO ..."
cargo install --git "$REPO" --package kernriftc --locked
cargo install --git "$REPO" --package kernrift --locked

# ── 4. verify ─────────────────────────────────────────────────────────────────
echo
for bin in $BINS; do
  path="$(command -v "$bin" 2>/dev/null || true)"
  if [[ -n "$path" ]]; then
    ok "$bin → $path"
  else
    warn "$bin not found on PATH — add \$HOME/.cargo/bin to your PATH."
  fi
done

echo
ok "Done! Try: kernriftc --help"
