#!/usr/bin/env bash
set -euo pipefail

acceptance_assert_nonempty_file() {
  local path="$1"
  if [[ ! -s "$path" ]]; then
    echo "expected non-empty file: $path" >&2
    exit 1
  fi
}

acceptance_optional_skip() {
  local label="$1"
  local reason="$2"
  echo "[optional] ${label} skipped: ${reason}"
}

acceptance_assert_json_string_field() {
  local path="$1"
  local field="$2"
  local expected="$3"
  if ! grep -Eq "\"${field}\"[[:space:]]*:[[:space:]]*\"${expected}\"" "$path"; then
    echo "expected JSON field ${field}=${expected} in ${path}" >&2
    exit 1
  fi
}

acceptance_assert_json_number_field() {
  local path="$1"
  local field="$2"
  local expected="$3"
  if ! grep -Eq "\"${field}\"[[:space:]]*:[[:space:]]*${expected}" "$path"; then
    echo "expected JSON field ${field}=${expected} in ${path}" >&2
    exit 1
  fi
}
