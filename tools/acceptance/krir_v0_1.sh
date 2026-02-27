#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kernrift-acceptance-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="tests/must_pass/locks_ok.kr"
CONTRACTS_OUT="$TMP_DIR/contracts.json"
HASH_OUT="$TMP_DIR/contracts.sha256"
REPORT_OUT="$TMP_DIR/verify.report.json"
BAD_HASH_OUT="$TMP_DIR/bad.sha256"

echo "[1/5] cargo test --workspace"
cargo test --workspace

echo "[2/5] cargo test -p kernriftc --test golden"
cargo test -p kernriftc --test golden

echo "[3/5] smoke: check emits contracts/hash"
cargo run -q -p kernriftc -- \
  check \
  --contracts-out "$CONTRACTS_OUT" \
  --hash-out "$HASH_OUT" \
  "$FIXTURE"

echo "[4/5] smoke: verify pass writes report"
cargo run -q -p kernriftc -- \
  verify \
  --contracts "$CONTRACTS_OUT" \
  --hash "$HASH_OUT" \
  --report "$REPORT_OUT"

grep -q '"schema_version": "kernrift_verify_report_v1"' "$REPORT_OUT"
grep -q '"result": "pass"' "$REPORT_OUT"

echo "[5/5] smoke: verify deny on hash mismatch (exit 1)"
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

echo "KRIR v0.1 acceptance: PASS"
