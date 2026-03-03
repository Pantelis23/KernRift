#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kernrift-artifact-exports-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="tests/must_pass/basic.kr"
KRBO_OUT="$TMP_DIR/basic.krbo"
KRBO_META="$TMP_DIR/basic.krbo.json"
ELF_OUT="$TMP_DIR/basic.o"
ELF_META="$TMP_DIR/basic.o.json"
ASM_OUT="$TMP_DIR/basic.s"

find_readelf() {
  if command -v readelf >/dev/null 2>&1; then
    printf '%s\n' "readelf"
    return 0
  fi
  if command -v llvm-readelf >/dev/null 2>&1; then
    printf '%s\n' "llvm-readelf"
    return 0
  fi
  return 1
}

assert_nonempty_file() {
  local path="$1"
  if [[ ! -s "$path" ]]; then
    echo "expected non-empty file: $path" >&2
    exit 1
  fi
}

echo "[1/6] emit krbo + metadata"
cargo run -q -p kernriftc -- \
  --emit=krbo \
  -o "$KRBO_OUT" \
  --meta-out "$KRBO_META" \
  "$FIXTURE"
assert_nonempty_file "$KRBO_OUT"
assert_nonempty_file "$KRBO_META"

echo "[2/6] emit elfobj + metadata"
cargo run -q -p kernriftc -- \
  --emit=elfobj \
  -o "$ELF_OUT" \
  --meta-out "$ELF_META" \
  "$FIXTURE"
assert_nonempty_file "$ELF_OUT"
assert_nonempty_file "$ELF_META"

echo "[3/6] emit asm"
cargo run -q -p kernriftc -- \
  --emit=asm \
  -o "$ASM_OUT" \
  "$FIXTURE"
assert_nonempty_file "$ASM_OUT"

echo "[4/6] verify krbo metadata"
cargo run -q -p kernriftc -- \
  verify-artifact-meta \
  "$KRBO_OUT" \
  "$KRBO_META"

echo "[5/6] verify elfobj metadata"
cargo run -q -p kernriftc -- \
  verify-artifact-meta \
  "$ELF_OUT" \
  "$ELF_META"

echo "[6/6] smoke-check asm structure"
grep -q '^\.text$' "$ASM_OUT"
grep -q '^bar:$' "$ASM_OUT"
grep -q '^foo:$' "$ASM_OUT"
grep -q '^    call bar$' "$ASM_OUT"
grep -q '^    ret$' "$ASM_OUT"

if READELF_TOOL="$(find_readelf)"; then
  echo "[optional] inspect emitted elfobj with $READELF_TOOL"
  READELF_OUTPUT="$("$READELF_TOOL" -h "$ELF_OUT")"
  printf '%s\n' "$READELF_OUTPUT" | grep -Eq 'Class:[[:space:]]+ELF64'
  printf '%s\n' "$READELF_OUTPUT" | grep -Eq "Data:[[:space:]]+2's complement, little endian"
  printf '%s\n' "$READELF_OUTPUT" | grep -Eq 'Type:[[:space:]]+REL'
  printf '%s\n' "$READELF_OUTPUT" | grep -Eq 'Machine:[[:space:]]+(Advanced Micro Devices X86-64|x86-64)'
fi

echo "kernriftc artifact export acceptance: PASS"
