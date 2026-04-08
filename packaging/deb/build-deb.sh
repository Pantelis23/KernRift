#!/bin/bash
# Build .deb packages for KernRift
# Usage: ./build-deb.sh [version]
# Produces: kernrift_VERSION_amd64.deb and kernrift_VERSION_arm64.deb
set -e

VERSION="${1:-2.4.0}"
REPO="Pantelis23/KernRift"
BASE="https://github.com/$REPO/releases/latest/download"
RAW="https://raw.githubusercontent.com/$REPO/main"

build_deb() {
    local arch="$1"      # amd64 or arm64
    local bin_name="$2"  # krc-linux-x86_64 or krc-linux-arm64

    local PKG="kernrift_${VERSION}_${arch}"
    rm -rf "$PKG"

    # Create directory structure
    mkdir -p "$PKG/DEBIAN"
    mkdir -p "$PKG/usr/bin"
    mkdir -p "$PKG/usr/share/kernrift/std"
    mkdir -p "$PKG/usr/share/doc/kernrift"

    # Control file
    cat > "$PKG/DEBIAN/control" <<EOF
Package: kernrift
Version: $VERSION
Section: devel
Priority: optional
Architecture: $arch
Maintainer: Pantelis Christou <pantelis@kernrift.org>
Homepage: https://kernrift.org
Description: Self-hosted systems language compiler for kernel development
 KernRift is a self-hosting systems language compiler that produces
 native executables for x86_64 and AArch64. It compiles itself to a
 fixed point in under 25ms. Features include inline assembly, naked
 functions, packed structs, signed comparisons, bitfield operations,
 volatile memory access, and freestanding mode for bare-metal targets.
 .
 The compiler is a single static binary with zero dependencies.
EOF

    # Download krc binary
    echo "  Downloading $bin_name..."
    curl -sSL -o "$PKG/usr/bin/krc" "$BASE/$bin_name"
    chmod 755 "$PKG/usr/bin/krc"

    # Download kr runner
    echo "  Downloading kr..."
    curl -sSL -o "$PKG/usr/bin/kr" "$BASE/kr"
    chmod 755 "$PKG/usr/bin/kr"

    # Download stdlib
    for mod in string io math fmt mem vec map color fb fixedpoint font memfast widget; do
        echo "  Downloading std/$mod.kr..."
        curl -sSL -o "$PKG/usr/share/kernrift/std/$mod.kr" "$RAW/std/$mod.kr"
    done

    # Copyright file
    curl -sSL -o "$PKG/usr/share/doc/kernrift/copyright" "$RAW/LICENSE"

    # Build .deb
    dpkg-deb --build --root-owner-group "$PKG"
    echo "  Built: ${PKG}.deb"
    rm -rf "$PKG"
}

echo "=== Building KernRift $VERSION .deb packages ==="
build_deb "amd64" "krc-linux-x86_64"
build_deb "arm64" "krc-linux-arm64"
echo "=== Done ==="
