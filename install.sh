#!/bin/bash
# KernRift Self-Hosted Compiler Installer
# Works on: Linux x86_64, Linux ARM64 (Raspberry Pi)
# No dependencies required — the compiler is fully static

set -euo pipefail

VERSION="1.0.0"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)   ARCH_NAME="x86_64" ;;
    aarch64|arm64)   ARCH_NAME="arm64" ;;
    *)               echo "error: unsupported architecture: $ARCH"; exit 1 ;;
esac

OS=$(uname -s)
case "$OS" in
    Linux)   OS_NAME="linux" ;;
    Darwin)  OS_NAME="macos" ;;
    *)       echo "error: unsupported OS: $OS (use install.ps1 for Windows)"; exit 1 ;;
esac

echo "=== KernRift Self-Hosted Compiler Installer ==="
echo "Version: $VERSION"
echo "Platform: $OS_NAME $ARCH_NAME"
echo "Install to: $INSTALL_DIR"
echo ""

# Check if we have a prebuilt binary for this platform
BINARY=""
if [ -f "$REPO_DIR/build/krc2" ] && [ "$ARCH_NAME" = "x86_64" ] && [ "$OS_NAME" = "linux" ]; then
    BINARY="$REPO_DIR/build/krc2"
elif [ -f "$REPO_DIR/dist/krc-$OS_NAME-$ARCH_NAME" ]; then
    BINARY="$REPO_DIR/dist/krc-$OS_NAME-$ARCH_NAME"
fi

if [ -n "$BINARY" ]; then
    echo "Found prebuilt binary: $BINARY"
else
    echo "No prebuilt binary found. Building from source..."

    # Check for the Rust compiler (needed for initial bootstrap)
    if [ -f "$REPO_DIR/build/krc" ]; then
        echo "Using existing bootstrap compiler: $REPO_DIR/build/krc"
    elif command -v kernriftc >/dev/null 2>&1; then
        echo "Using system kernriftc for bootstrap"
        KERNRIFTC=kernriftc bash "$REPO_DIR/build.sh"
    else
        echo "error: no bootstrap compiler found."
        echo "Install the Rust KernRift compiler first:"
        echo "  cargo install --git https://github.com/Pantelis23/KernRift kernriftc"
        echo "Then run this installer again."
        exit 1
    fi

    # Self-compile
    echo "Self-compiling krc..."
    cp "$REPO_DIR/build/krc.kr" "$REPO_DIR/test_input.kr"
    "$REPO_DIR/build/krc" 2>/dev/null
    BINARY="$REPO_DIR/a.out"

    if [ "$ARCH_NAME" = "arm64" ]; then
        echo "Cross-compiling for ARM64..."
        # The x86_64 compiler can cross-compile to ARM64
        chmod +x "$BINARY"
        cp "$BINARY" "$REPO_DIR/build/krc2"
        "$REPO_DIR/build/krc2" --arch=arm64 "$REPO_DIR/build/krc.kr" -o "$REPO_DIR/build/krc_arm64" 2>/dev/null
        BINARY="$REPO_DIR/build/krc_arm64"
    fi
fi

# Install
mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/krc"
chmod +x "$INSTALL_DIR/krc"

echo ""
echo "Installed: $INSTALL_DIR/krc"

# Verify
if "$INSTALL_DIR/krc" --version 2>/dev/null; then
    echo ""
else
    echo "(version check skipped — Rust runtime doesn't pass args)"
fi

# Check PATH
if echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR\$"; then
    echo "krc is already in your PATH."
else
    echo ""
    echo "Add to your PATH by adding this to ~/.bashrc or ~/.profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo ""
echo "Usage:"
echo "  krc program.kr                  # compile to fat binary (x86_64 + arm64)"
echo "  krc --arch=x86_64 prog.kr      # compile for x86_64 only"
echo "  krc --arch=arm64 prog.kr       # compile for arm64 only"
echo "  krc -o output prog.kr          # specify output file"
echo "  krc check prog.kr              # run analysis passes"
echo "  krc lc prog.kr                 # living compiler report"
echo "  krc --version                  # show version"
echo ""
echo "=== Installation complete ==="

# Clean up
rm -f "$REPO_DIR/test_input.kr" "$REPO_DIR/a.out"
