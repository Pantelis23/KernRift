#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kernrift-acceptance-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="tests/must_pass/locks_ok.kr"
PROOF_FIXTURE="examples/uart_console_executable.kr"
VALUE_FLOW_FIXTURE="examples/uart_console_value_flow.kr"
EXPLICIT_SLOT_FIXTURE="examples/uart_console_explicit_slot.kr"
BRANCH_ZERO_FIXTURE="examples/uart_console_branch_zero.kr"
BRANCH_EQ_FIXTURE="examples/uart_console_branch_eq.kr"
BRANCH_MASK_FIXTURE="examples/uart_console_branch_mask.kr"
CALL_RETURN_FIXTURE="examples/uart_console_call_return.kr"
STACK_CELL_FIXTURE="examples/uart_console_stack_cell.kr"
CONTRACTS_OUT="$TMP_DIR/contracts.json"
HASH_OUT="$TMP_DIR/contracts.sha256"
REPORT_OUT="$TMP_DIR/verify.report.json"
INSPECT_REPORT_JSON="$TMP_DIR/inspect.report.json"
MALFORMED_REPORT="$TMP_DIR/malformed.report.json"
MALFORMED_STDOUT="$TMP_DIR/malformed.inspect.stdout"
MALFORMED_STDERR="$TMP_DIR/malformed.inspect.stderr"
BAD_HASH_OUT="$TMP_DIR/bad.sha256"
PROOF_ELFEXE_OUT="$TMP_DIR/uart_console_executable.elf"
PROOF_INSPECT_JSON="$TMP_DIR/uart_console_executable.inspect.json"
VALUE_FLOW_ELFEXE_OUT="$TMP_DIR/uart_console_value_flow.elf"
VALUE_FLOW_INSPECT_JSON="$TMP_DIR/uart_console_value_flow.inspect.json"
EXPLICIT_SLOT_ELFEXE_OUT="$TMP_DIR/uart_console_explicit_slot.elf"
EXPLICIT_SLOT_INSPECT_JSON="$TMP_DIR/uart_console_explicit_slot.inspect.json"
BRANCH_ZERO_ELFEXE_OUT="$TMP_DIR/uart_console_branch_zero.elf"
BRANCH_ZERO_INSPECT_JSON="$TMP_DIR/uart_console_branch_zero.inspect.json"
BRANCH_EQ_ELFEXE_OUT="$TMP_DIR/uart_console_branch_eq.elf"
BRANCH_EQ_INSPECT_JSON="$TMP_DIR/uart_console_branch_eq.inspect.json"
BRANCH_MASK_ELFEXE_OUT="$TMP_DIR/uart_console_branch_mask.elf"
BRANCH_MASK_INSPECT_JSON="$TMP_DIR/uart_console_branch_mask.inspect.json"
CALL_RETURN_ELFEXE_OUT="$TMP_DIR/uart_console_call_return.elf"
CALL_RETURN_INSPECT_JSON="$TMP_DIR/uart_console_call_return.inspect.json"
STACK_CELL_ELFEXE_OUT="$TMP_DIR/uart_console_stack_cell.elf"
STACK_CELL_INSPECT_JSON="$TMP_DIR/uart_console_stack_cell.inspect.json"

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

echo "[6/7] smoke: proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$PROOF_ELFEXE_OUT" \
  "$PROOF_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$PROOF_ELFEXE_OUT" \
  --format json > "$PROOF_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$PROOF_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$PROOF_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$PROOF_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$PROOF_INSPECT_JSON"

echo "[7/8] smoke: value-flow proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$VALUE_FLOW_ELFEXE_OUT" \
  "$VALUE_FLOW_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$VALUE_FLOW_ELFEXE_OUT" \
  --format json > "$VALUE_FLOW_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$VALUE_FLOW_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$VALUE_FLOW_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$VALUE_FLOW_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$VALUE_FLOW_INSPECT_JSON"
grep -q '"mirror_status"' "$VALUE_FLOW_INSPECT_JSON"
grep -q '"mirror_watchdog"' "$VALUE_FLOW_INSPECT_JSON"

echo "[8/9] smoke: explicit-slot proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$EXPLICIT_SLOT_ELFEXE_OUT" \
  "$EXPLICIT_SLOT_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$EXPLICIT_SLOT_ELFEXE_OUT" \
  --format json > "$EXPLICIT_SLOT_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$EXPLICIT_SLOT_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$EXPLICIT_SLOT_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$EXPLICIT_SLOT_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$EXPLICIT_SLOT_INSPECT_JSON"
grep -q '"mirror_status"' "$EXPLICIT_SLOT_INSPECT_JSON"
grep -q '"mirror_watchdog"' "$EXPLICIT_SLOT_INSPECT_JSON"

echo "[9/10] smoke: branch-zero proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$BRANCH_ZERO_ELFEXE_OUT" \
  "$BRANCH_ZERO_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$BRANCH_ZERO_ELFEXE_OUT" \
  --format json > "$BRANCH_ZERO_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$BRANCH_ZERO_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$BRANCH_ZERO_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$BRANCH_ZERO_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$BRANCH_ZERO_INSPECT_JSON"
grep -q '"send_idle_word"' "$BRANCH_ZERO_INSPECT_JSON"
grep -q '"send_ready_word"' "$BRANCH_ZERO_INSPECT_JSON"

echo "[10/11] smoke: branch-eq proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$BRANCH_EQ_ELFEXE_OUT" \
  "$BRANCH_EQ_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$BRANCH_EQ_ELFEXE_OUT" \
  --format json > "$BRANCH_EQ_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$BRANCH_EQ_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$BRANCH_EQ_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$BRANCH_EQ_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$BRANCH_EQ_INSPECT_JSON"
grep -q '"send_idle_word"' "$BRANCH_EQ_INSPECT_JSON"
grep -q '"send_ready_word"' "$BRANCH_EQ_INSPECT_JSON"

echo "[11/12] smoke: branch-mask proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$BRANCH_MASK_ELFEXE_OUT" \
  "$BRANCH_MASK_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$BRANCH_MASK_ELFEXE_OUT" \
  --format json > "$BRANCH_MASK_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$BRANCH_MASK_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$BRANCH_MASK_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$BRANCH_MASK_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$BRANCH_MASK_INSPECT_JSON"
grep -q '"send_idle_word"' "$BRANCH_MASK_INSPECT_JSON"
grep -q '"send_ready_word"' "$BRANCH_MASK_INSPECT_JSON"

echo "[12/13] smoke: call-return proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$CALL_RETURN_ELFEXE_OUT" \
  "$CALL_RETURN_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$CALL_RETURN_ELFEXE_OUT" \
  --format json > "$CALL_RETURN_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$CALL_RETURN_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$CALL_RETURN_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$CALL_RETURN_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$CALL_RETURN_INSPECT_JSON"
grep -q '"read_status"' "$CALL_RETURN_INSPECT_JSON"
grep -q '"send_idle_word"' "$CALL_RETURN_INSPECT_JSON"
grep -q '"send_ready_word"' "$CALL_RETURN_INSPECT_JSON"

echo "[13/13] smoke: stack-cell proof program emits backend elf executable"
cargo run -q -p kernriftc -- \
  --emit=elfexe \
  -o "$STACK_CELL_ELFEXE_OUT" \
  "$STACK_CELL_FIXTURE"

cargo run -q -p kernriftc -- \
  inspect-artifact \
  "$STACK_CELL_ELFEXE_OUT" \
  --format json > "$STACK_CELL_INSPECT_JSON"

grep -q '"artifact_kind": "elf_executable"' "$STACK_CELL_INSPECT_JSON"
grep -q '"machine": "x86_64"' "$STACK_CELL_INSPECT_JSON"
grep -q '"has_entry_symbol": true' "$STACK_CELL_INSPECT_JSON"
grep -q '"has_undefined_symbols": false' "$STACK_CELL_INSPECT_JSON"
grep -q '"read_status"' "$STACK_CELL_INSPECT_JSON"
grep -q '"read_control"' "$STACK_CELL_INSPECT_JSON"
grep -q '"send_idle_word"' "$STACK_CELL_INSPECT_JSON"
grep -q '"send_ready_word"' "$STACK_CELL_INSPECT_JSON"

echo "KRIR v0.1 acceptance: PASS"
