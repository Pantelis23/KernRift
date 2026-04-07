#!/bin/sh
# KernRift Installer — download and install krc + kr from GitHub releases
# Usage: curl -sSf https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.sh | sh
#
# Alternative installation methods:
#   Homebrew (macOS/Linux):  brew install kernrift
#   Scoop (Windows):         scoop bucket add kernrift https://github.com/Pantelis23/KernRift && scoop install kernrift
#   Winget (Windows):        winget install Pantelis23.KernRift
#   Debian/Ubuntu (.deb):    curl -sSLO https://github.com/.../kernrift_2.1.0_amd64.deb && sudo dpkg -i kernrift_*.deb
#   AUR (Arch Linux):        yay -S kernrift
#   PowerShell (Windows):    irm https://raw.githubusercontent.com/Pantelis23/KernRift/main/install.ps1 | iex
#
set -e

REPO="Pantelis23/KernRift"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
ARCH=$(uname -m)
IS_ANDROID=0
if [ -f "/system/bin/linker64" ]; then
    IS_ANDROID=1
fi

case "$ARCH" in
    x86_64|amd64) ARCH_NAME="x86_64" ;;
    aarch64|arm64)
        if [ "$IS_ANDROID" = "1" ]; then
            ARCH_NAME="android-arm64"
        else
            ARCH_NAME="arm64"
        fi
        ;;
    *) echo "error: unsupported architecture: $ARCH"; exit 1 ;;
esac

OS=$(uname -s)
case "$OS" in
    Linux)
        if [ "$IS_ANDROID" = "1" ]; then
            OS_NAME="android"
        else
            OS_NAME="linux"
        fi
        ;;
    Darwin) OS_NAME="macos" ;;
    *)      echo "error: unsupported OS: $OS"; exit 1 ;;
esac

# Android install path
if [ "$IS_ANDROID" = "1" ]; then
    if [ -d "/data/data/com.termux" ]; then
        INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
    else
        INSTALL_DIR="${INSTALL_DIR:-/data/local/tmp}"
    fi
fi

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
curl -sL -o "$INSTALL_DIR/kr" "$BASE/kr-$OS_NAME-$ARCH_NAME"
chmod +x "$INSTALL_DIR/kr"

# Download standard library
STD_DIR="$HOME/.local/share/kernrift/std"
echo "Installing standard library..."
mkdir -p "$STD_DIR"
for mod in string io math fmt mem vec map; do
    curl -sL -o "$STD_DIR/$mod.kr" \
        "https://raw.githubusercontent.com/Pantelis23/KernRift/main/std/$mod.kr"
done
echo "Standard library: $STD_DIR"

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
