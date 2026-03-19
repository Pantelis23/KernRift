#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kernrift-canonical-acceptance-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

FIXTURE="tests/living_compiler/migration_preview_legacy_unary.kr"
WORK_FIXTURE="$TMP_DIR/migration_preview_legacy_unary.kr"
SNAPSHOT_FIXTURE="$TMP_DIR/migration_preview_legacy_unary.snapshot.kr"

cp "$FIXTURE" "$WORK_FIXTURE"
cp "$FIXTURE" "$SNAPSHOT_FIXTURE"

run_kernriftc() {
  cargo run -q -p kernriftc -- "$@"
}

assert_contains() {
  local needle="$1"
  local path="$2"
  if ! grep -Fq -- "$needle" "$path"; then
    echo "expected to find '$needle' in $path" >&2
    exit 1
  fi
}

assert_not_contains() {
  local needle="$1"
  local path="$2"
  if grep -Fq -- "$needle" "$path"; then
    echo "expected not to find '$needle' in $path" >&2
    exit 1
  fi
}

assert_unchanged_fixture() {
  if ! cmp -s "$WORK_FIXTURE" "$SNAPSHOT_FIXTURE"; then
    echo "expected non-mutating canonical flow to leave fixture unchanged" >&2
    exit 1
  fi
}

echo "[1/12] canonical check smoke (file, exit 1)"
CHECK_TEXT_OUT="$TMP_DIR/check.text.out"
set +e
run_kernriftc check --canonical "$WORK_FIXTURE" >"$CHECK_TEXT_OUT"
status=$?
set -e
if [[ "$status" -ne 1 ]]; then
  echo "expected check --canonical exit code 1, got $status" >&2
  exit 1
fi
assert_contains "surface: stable" "$CHECK_TEXT_OUT"
assert_contains "file: $WORK_FIXTURE" "$CHECK_TEXT_OUT"
assert_contains "canonical_findings: 5" "$CHECK_TEXT_OUT"
assert_unchanged_fixture

echo "[2/12] canonical dry-run smoke (file)"
DRY_RUN_TEXT_OUT="$TMP_DIR/dry_run.text.out"
run_kernriftc fix --canonical --dry-run "$WORK_FIXTURE" >"$DRY_RUN_TEXT_OUT"
assert_contains "surface: stable" "$DRY_RUN_TEXT_OUT"
assert_contains "rewrites_planned: 5" "$DRY_RUN_TEXT_OUT"
assert_unchanged_fixture

echo "[3/12] canonical stdout smoke (file)"
STDOUT_OUT="$TMP_DIR/stdout.out"
run_kernriftc fix --canonical --stdout "$WORK_FIXTURE" >"$STDOUT_OUT"
assert_contains "@eff(alloc)" "$STDOUT_OUT"
assert_contains "@ctx(irq)" "$STDOUT_OUT"
assert_not_contains "@alloc" "$STDOUT_OUT"
assert_unchanged_fixture

echo "[4/12] canonical diff smoke (file)"
DIFF_OUT="$TMP_DIR/diff.out"
run_kernriftc fix --canonical --diff "$WORK_FIXTURE" >"$DIFF_OUT"
assert_contains "--- original" "$DIFF_OUT"
assert_contains "+++ canonical" "$DIFF_OUT"
assert_contains "+@eff(alloc)" "$DIFF_OUT"
assert_unchanged_fixture

echo "[5/12] canonical edit-plan JSON smoke (file)"
PREVIEW_JSON_OUT="$TMP_DIR/preview.json.out"
run_kernriftc migrate-preview --canonical-edits --format json "$WORK_FIXTURE" >"$PREVIEW_JSON_OUT"
assert_contains '"schema_version": "kernrift_canonical_edit_plan_v1"' "$PREVIEW_JSON_OUT"
assert_contains '"edits_count": 5' "$PREVIEW_JSON_OUT"
assert_unchanged_fixture

echo "[6/12] canonical edit-plan text smoke (file)"
PREVIEW_TEXT_OUT="$TMP_DIR/preview.text.out"
run_kernriftc migrate-preview --canonical-edits --format text "$WORK_FIXTURE" >"$PREVIEW_TEXT_OUT"
assert_contains "surface: stable" "$PREVIEW_TEXT_OUT"
assert_contains "file: $WORK_FIXTURE" "$PREVIEW_TEXT_OUT"
assert_contains "edits_count: 5" "$PREVIEW_TEXT_OUT"
assert_unchanged_fixture

echo "[7/12] canonical check smoke (stdin, exit 1)"
CHECK_STDIN_TEXT_OUT="$TMP_DIR/check.stdin.text.out"
set +e
cat "$WORK_FIXTURE" | run_kernriftc check --canonical --stdin >"$CHECK_STDIN_TEXT_OUT"
status=$?
set -e
if [[ "$status" -ne 1 ]]; then
  echo "expected check --canonical --stdin exit code 1, got $status" >&2
  exit 1
fi
assert_contains "surface: stable" "$CHECK_STDIN_TEXT_OUT"
assert_contains "file: <stdin>" "$CHECK_STDIN_TEXT_OUT"
assert_contains "canonical_findings: 5" "$CHECK_STDIN_TEXT_OUT"

echo "[8/12] canonical dry-run smoke (stdin)"
DRY_RUN_STDIN_TEXT_OUT="$TMP_DIR/dry_run.stdin.text.out"
cat "$WORK_FIXTURE" | run_kernriftc fix --canonical --dry-run --stdin >"$DRY_RUN_STDIN_TEXT_OUT"
assert_contains "file: <stdin>" "$DRY_RUN_STDIN_TEXT_OUT"
assert_contains "rewrites_planned: 5" "$DRY_RUN_STDIN_TEXT_OUT"

echo "[9/12] canonical stdout smoke (stdin)"
STDOUT_STDIN_OUT="$TMP_DIR/stdout.stdin.out"
cat "$WORK_FIXTURE" | run_kernriftc fix --canonical --stdout --stdin >"$STDOUT_STDIN_OUT"
assert_contains "@eff(alloc)" "$STDOUT_STDIN_OUT"
assert_contains "@ctx(irq)" "$STDOUT_STDIN_OUT"
assert_not_contains "@alloc" "$STDOUT_STDIN_OUT"

echo "[10/12] canonical diff smoke (stdin)"
DIFF_STDIN_OUT="$TMP_DIR/diff.stdin.out"
cat "$WORK_FIXTURE" | run_kernriftc fix --canonical --diff --stdin >"$DIFF_STDIN_OUT"
assert_contains "--- original" "$DIFF_STDIN_OUT"
assert_contains "+++ canonical" "$DIFF_STDIN_OUT"
assert_contains "+@eff(alloc)" "$DIFF_STDIN_OUT"

echo "[11/12] canonical edit-plan text smoke (stdin)"
PREVIEW_STDIN_TEXT_OUT="$TMP_DIR/preview.stdin.text.out"
cat "$WORK_FIXTURE" | run_kernriftc migrate-preview --canonical-edits --stdin >"$PREVIEW_STDIN_TEXT_OUT"
assert_contains "surface: stable" "$PREVIEW_STDIN_TEXT_OUT"
assert_contains "file: <stdin>" "$PREVIEW_STDIN_TEXT_OUT"
assert_contains "edits_count: 5" "$PREVIEW_STDIN_TEXT_OUT"

echo "[12/12] canonical edit-plan JSON smoke (stdin)"
PREVIEW_STDIN_JSON_OUT="$TMP_DIR/preview.stdin.json.out"
cat "$WORK_FIXTURE" | run_kernriftc migrate-preview --canonical-edits --stdin --format json >"$PREVIEW_STDIN_JSON_OUT"
assert_contains '"schema_version": "kernrift_canonical_edit_plan_v1"' "$PREVIEW_STDIN_JSON_OUT"
assert_contains '"edits_count": 5' "$PREVIEW_STDIN_JSON_OUT"

echo "kernriftc canonical acceptance: PASS"
