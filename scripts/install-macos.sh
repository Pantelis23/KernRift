#!/usr/bin/env bash
set -euo pipefail

REPO="https://github.com/Pantelis23/KernRift"
BINS="kernriftc kernrift"

# ── helpers ───────────────────────────────────────────────────────────────────
info()  { printf '\033[1;34m=>\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m✓\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m!\033[0m  %s\n' "$*" >&2; }
die()   { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── 1. warn about Homebrew Rust ───────────────────────────────────────────────
if command -v brew >/dev/null 2>&1 && brew list rust >/dev/null 2>&1; then
  warn "Homebrew-managed Rust detected."
  warn "It may conflict with rustup. If the install fails, run:"
  warn "  brew unlink rust"
  warn "and re-run this script."
  echo
fi

# ── 2. install rustup if missing ──────────────────────────────────────────────
if ! command -v rustup >/dev/null 2>&1; then
  info "Installing rustup..."
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain none
  else
    die "curl is required. Install Xcode Command Line Tools first: xcode-select --install"
  fi
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
else
  ok "rustup already installed: $(rustup --version 2>&1 | head -1)"
fi

export PATH="$HOME/.cargo/bin:$PATH"

# ── 3. clone and install ──────────────────────────────────────────────────────
info "Installing kernriftc and kernrift from $REPO ..."
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

git clone --depth=1 "$REPO" "$tmpdir/kernrift"
cargo install --path "$tmpdir/kernrift/crates/kernriftc" --locked
cargo install --path "$tmpdir/kernrift/crates/kernrift"  --locked

# ── 4. verify ─────────────────────────────────────────────────────────────────
echo
for bin in $BINS; do
  path="$(command -v "$bin" 2>/dev/null || true)"
  if [[ -n "$path" ]]; then
    ok "$bin → $path"
  else
    warn "$bin not found on PATH — add \$HOME/.cargo/bin to your PATH."
    warn "Add this to your shell profile (~/.zshrc or ~/.bash_profile):"
    warn '  export PATH="$HOME/.cargo/bin:$PATH"'
  fi
done

echo
ok "Done! Try: kernriftc --help"
