#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

tmp_root="${TMPDIR:-/tmp}"
tmp_dir="$(mktemp -d "${tmp_root%/}/kernrift-inspect-fixtures.XXXXXX")"
keep_tmp="${KEEP_TMP:-0}"

cleanup() {
  if [[ "${keep_tmp}" != "1" ]]; then
    rm -rf "${tmp_dir}"
  else
    echo "[inspect-helper] keeping temp dir: ${tmp_dir}"
  fi
}
trap cleanup EXIT

echo "[inspect-helper] temp dir: ${tmp_dir}"

emit_artifact() {
  local fixture="$1"
  local emit_kind="$2"
  local out_path="$3"
  cargo run -q -p kernriftc -- --emit="${emit_kind}" -o "${out_path}" "${fixture}"
}

inspect_artifact() {
  local artifact_path="$1"
  local text_out="$2"
  local json_out="$3"
  cargo run -q -p kernriftc -- inspect-artifact "${artifact_path}" > "${text_out}"
  cargo run -q -p kernriftc -- inspect-artifact "${artifact_path}" --format json > "${json_out}"
}

run_fixture() {
  local fixture="$1"
  local name="$2"
  local emit_kind="$3"
  local ext="$4"

  local artifact_path="${tmp_dir}/${name}.${ext}"
  local text_out="${tmp_dir}/${name}.${emit_kind}.inspect.txt"
  local json_out="${tmp_dir}/${name}.${emit_kind}.inspect.json"

  emit_artifact "${fixture}" "${emit_kind}" "${artifact_path}"
  inspect_artifact "${artifact_path}" "${text_out}" "${json_out}"

  echo "[inspect-helper] fixture=${name} emit=${emit_kind}"
  echo "  artifact: ${artifact_path}"
  echo "  inspect text: ${text_out}"
  echo "  inspect json: ${json_out}"
}

basic_fixture="tests/must_pass/basic.kr"
extern_fixture="tests/must_pass/extern_call_object.kr"
mixed_fixture="tests/must_pass/extern_internal_chain.kr"

run_fixture "${basic_fixture}" "basic" "krbo" "krbo"
run_fixture "${basic_fixture}" "basic" "elfobj" "o"
run_fixture "${basic_fixture}" "basic" "asm" "s"
run_fixture "${extern_fixture}" "extern_call_object" "elfobj" "o"
run_fixture "${extern_fixture}" "extern_call_object" "asm" "s"
run_fixture "${mixed_fixture}" "extern_internal_chain" "elfobj" "o"
run_fixture "${mixed_fixture}" "extern_internal_chain" "asm" "s"

echo "[inspect-helper] completed"
