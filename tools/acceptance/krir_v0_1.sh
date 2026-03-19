#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kernrift-acceptance-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="tests/must_pass/locks_ok.kr"
PROOF_FIXTURE="examples/uart_console_probe.kr"
CONTRACTS_OUT="$TMP_DIR/contracts.json"
HASH_OUT="$TMP_DIR/contracts.sha256"
REPORT_OUT="$TMP_DIR/verify.report.json"
INSPECT_REPORT_JSON="$TMP_DIR/inspect.report.json"
MALFORMED_REPORT="$TMP_DIR/malformed.report.json"
MALFORMED_STDOUT="$TMP_DIR/malformed.inspect.stdout"
MALFORMED_STDERR="$TMP_DIR/malformed.inspect.stderr"
BAD_HASH_OUT="$TMP_DIR/bad.sha256"
PROOF_ASM_OUT="$TMP_DIR/uart_console_probe.s"

echo "[1/3] smoke: check emits contracts/hash"
cargo run -q -p kernriftc -- \
  check \
  --contracts-out "$CONTRACTS_OUT" \
  --hash-out "$HASH_OUT" \
  "$FIXTURE"

echo "[2/3] smoke: verify pass writes report"
cargo run -q -p kernriftc -- \
  verify \
  --contracts "$CONTRACTS_OUT" \
  --hash "$HASH_OUT" \
  --report "$REPORT_OUT"

grep -q '"schema_version": "kernrift_verify_report_v1"' "$REPORT_OUT"
grep -q '"result": "pass"' "$REPORT_OUT"

echo "[3/4] smoke: inspect-report emits structured JSON"
cargo run -q -p kernriftc -- \
  inspect-report \
  --report "$REPORT_OUT" \
  --format json > "$INSPECT_REPORT_JSON"

grep -q '"schema_version": "kernrift_inspect_report_v1"' "$INSPECT_REPORT_JSON"
grep -q "\"file\": \"$REPORT_OUT\"" "$INSPECT_REPORT_JSON"
grep -q '"result": "pass"' "$INSPECT_REPORT_JSON"

echo "[4/5] smoke: inspect-report malformed JSON mode stays stderr-only (exit 2)"
printf '{}\n' > "$MALFORMED_REPORT"
set +e
cargo run -q -p kernriftc -- \
  inspect-report \
  --report "$MALFORMED_REPORT" \
  --format json >"$MALFORMED_STDOUT" 2>"$MALFORMED_STDERR"
status=$?
set -e
if [[ "$status" -ne 2 ]]; then
  echo "expected inspect-report malformed exit code 2, got $status" >&2
  exit 1
fi
test ! -s "$MALFORMED_STDOUT"
grep -q "missing string field 'schema_version'" "$MALFORMED_STDERR"

echo "[5/6] smoke: verify deny on hash mismatch (exit 1)"
printf '%064d\n' 0 > "$BAD_HASH_OUT"
set +e
cargo run -q -p kernriftc -- \
  verify \
  --contracts "$CONTRACTS_OUT" \
  --hash "$BAD_HASH_OUT" \
  --report "$TMP_DIR/verify.deny.report.json"
status=$?
set -e
if [[ "$status" -ne 1 ]]; then
  echo "expected verify hash-mismatch exit code 1, got $status" >&2
  exit 1
fi

echo "[6/6] smoke: proof program emits backend asm"
cargo run -q -p kernriftc -- \
  --emit=asm \
  -o "$PROOF_ASM_OUT" \
  "$PROOF_FIXTURE"

grep -q "call platform_barrier" "$PROOF_ASM_OUT"
grep -q 'movabs \$0x1014, %rax' "$PROOF_ASM_OUT"
grep -q 'movl \$0xdeadbeef, %ecx' "$PROOF_ASM_OUT"

echo "KRIR v0.1 acceptance: PASS"
