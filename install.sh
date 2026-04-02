#!/bin/sh
# KernRift Installer — download and install krc + kr from GitHub releases
# Usage: curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh
set -e

REPO="Pantelis23/KernRift"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) ARCH_NAME="x86_64" ;;
    aarch64|arm64) ARCH_NAME="arm64" ;;
    *) echo "error: unsupported architecture: $ARCH"; exit 1 ;;
esac

OS=$(uname -s)
case "$OS" in
    Linux)  OS_NAME="linux" ;;
    Darwin) OS_NAME="macos" ;;
    *)      echo "error: unsupported OS: $OS"; exit 1 ;;
esac

echo "=== KernRift Installer ==="
echo "Platform: $OS_NAME $ARCH_NAME"
echo "Install to: $INSTALL_DIR"
echo ""

mkdir -p "$INSTALL_DIR"

BASE="https://github.com/$REPO/releases/latest/download"

# Download krc compiler
echo "Downloading krc..."
curl -sL -o "$INSTALL_DIR/krc" "$BASE/krc-$OS_NAME-$ARCH_NAME"
chmod +x "$INSTALL_DIR/krc"

# Download kr runner
echo "Downloading kr..."
curl -sL -o "$INSTALL_DIR/kr" "$BASE/kr"
chmod +x "$INSTALL_DIR/kr"

# Download native extractor for kr
curl -sL -o "$INSTALL_DIR/kr-$OS_NAME-$ARCH_NAME" "$BASE/kr-$OS_NAME-$ARCH_NAME" 2>/dev/null || true
chmod +x "$INSTALL_DIR/kr-$OS_NAME-$ARCH_NAME" 2>/dev/null || true

echo ""

# Verify
if "$INSTALL_DIR/krc" --version 2>/dev/null; then
    echo ""
fi

# Check PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) echo "krc is in your PATH." ;;
    *) echo "Add to PATH:  export PATH=\"$INSTALL_DIR:\$PATH\""
       echo "Or add that line to ~/.bashrc" ;;
esac

echo ""
echo "Usage:"
echo "  krc hello.kr -o hello.krbo    # compile (fat binary)"
echo "  kr hello.krbo                 # run on any platform"
echo "  krc --arch=x86_64 hello.kr    # native ELF"
echo "  krc check module.kr           # safety analysis"
echo "  krc lc program.kr             # living compiler"
echo ""
echo "=== Done ==="
