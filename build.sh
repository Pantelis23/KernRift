#!/bin/bash
set -euo pipefail

KERNRIFTC="${KERNRIFTC:-kernriftc}"
DIR="$(cd "$(dirname "$0")" && pwd)"

# Combine all source files into a single compilation unit
# Order matters: libraries first, then main
cat \
    "$DIR/src/lexer.kr" \
    "$DIR/src/ast.kr" \
    "$DIR/src/parser.kr" \
    "$DIR/src/codegen.kr" \
    "$DIR/src/codegen_aarch64.kr" \
    "$DIR/src/format_macho.kr" \
    "$DIR/src/format_pe.kr" \
    "$DIR/src/format_archive.kr" \
    "$DIR/src/analysis.kr" \
    "$DIR/src/living.kr" \
    "$DIR/src/runtime.kr" \
    "$DIR/src/main.kr" \
    > "$DIR/build/krc.kr"

# Compile to native hostexe
"$KERNRIFTC" --emit=hostexe "$DIR/build/krc.kr" -o "$DIR/build/krc"
chmod +x "$DIR/build/krc"

echo "Built: $DIR/build/krc"
