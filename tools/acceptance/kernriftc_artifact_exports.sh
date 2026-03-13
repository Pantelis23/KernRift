#!/bin/bash
set -euo pipefail

if [[ -z "${PATH:-}" ]]; then
  PATH="$HOME/.cargo/bin:/usr/bin:/bin"
  export ACCEPTANCE_DISABLE_OPTIONAL_TOOL_DISCOVERY=1
fi

if [[ "${ACCEPTANCE_TRACE:-0}" == "1" ]]; then
  set -x
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck source=tools/acceptance/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"
# shellcheck source=tools/acceptance/lib/toolchain.sh
source "$SCRIPT_DIR/lib/toolchain.sh"
# shellcheck source=tools/acceptance/lib/host_env.sh
source "$SCRIPT_DIR/lib/host_env.sh"
# shellcheck source=tools/acceptance/lib/hosted_x86_64.sh
source "$SCRIPT_DIR/lib/hosted_x86_64.sh"

cd "$ROOT_DIR"

if [[ -n "${CARGO_BIN:-}" ]]; then
  CARGO_CMD="$CARGO_BIN"
elif command -v cargo >/dev/null 2>&1; then
  CARGO_CMD="$(command -v cargo)"
elif [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  CARGO_CMD="$HOME/.cargo/bin/cargo"
else
  echo "missing cargo executable for acceptance script" >&2
  exit 1
fi

tmp_root="${ACCEPTANCE_TMPDIR:-${TMPDIR:-/tmp}}"
TMP_DIR="$(mktemp -d "${tmp_root%/}/kernrift-artifact-exports-XXXXXX")"

cleanup_tmp() {
  if [[ "${ACCEPTANCE_KEEP_TMP:-0}" == "1" ]]; then
    echo "[info] ACCEPTANCE_KEEP_TMP=1 preserving tmp dir: $TMP_DIR"
  else
    rm -rf "$TMP_DIR"
  fi
}

CURRENT_STEP=""
CURRENT_FIXTURE=""
declare -a CURRENT_ARTIFACTS=()

acceptance_on_error() {
  local status="$1"
  {
    echo "[failure] step: ${CURRENT_STEP:-unknown}"
    if [[ -n "${CURRENT_FIXTURE:-}" ]]; then
      echo "[failure] fixture: $CURRENT_FIXTURE"
    fi
    if ((${#CURRENT_ARTIFACTS[@]})); then
      echo "[failure] artifact paths:"
      for artifact in "${CURRENT_ARTIFACTS[@]}"; do
        echo "  - $artifact"
      done
    fi
    if [[ -n "${ACCEPTANCE_LAST_READELF_HEADER:-}" ]]; then
      echo "[failure] last readelf header output:"
      printf '%s\n' "$ACCEPTANCE_LAST_READELF_HEADER"
    fi
    if [[ -n "${ACCEPTANCE_LAST_READELF_SYMBOLS:-}" ]]; then
      echo "[failure] last readelf symbols output:"
      printf '%s\n' "$ACCEPTANCE_LAST_READELF_SYMBOLS"
    fi
    if [[ -n "${ACCEPTANCE_LAST_RELOCS_TEXT:-}" ]]; then
      echo "[failure] last readelf relocations output:"
      printf '%s\n' "$ACCEPTANCE_LAST_RELOCS_TEXT"
    fi
    echo "[failure] tmp dir: $TMP_DIR"
  } >&2
  exit "$status"
}

trap 'acceptance_on_error $?' ERR
trap cleanup_tmp EXIT

BASIC_FIXTURE="tests/must_pass/basic.kr"
EXTERN_FIXTURE="tests/must_pass/extern_call_object.kr"
MIXED_FIXTURE="tests/must_pass/extern_internal_chain.kr"

BASIC_KRBO_OUT="$TMP_DIR/basic.krbo"
BASIC_KRBO_META="$TMP_DIR/basic.krbo.json"
BASIC_ELF_OUT="$TMP_DIR/basic.o"
BASIC_ELF_META="$TMP_DIR/basic.o.json"
BASIC_ASM_OUT="$TMP_DIR/basic.s"

EXTERN_ELF_OUT="$TMP_DIR/extern_call_object.o"
EXTERN_ELF_META="$TMP_DIR/extern_call_object.o.json"
EXTERN_ASM_OUT="$TMP_DIR/extern_call_object.s"
EXTERN_ASM_OBJ="$TMP_DIR/extern_call_object.from_asm.o"

MIXED_ELF_OUT="$TMP_DIR/extern_internal_chain.o"
MIXED_ELF_META="$TMP_DIR/extern_internal_chain.o.json"
MIXED_ASM_OUT="$TMP_DIR/extern_internal_chain.s"
MIXED_ASM_OBJ="$TMP_DIR/extern_internal_chain.from_asm.o"
INSPECT_BASIC_ELF_TXT="$TMP_DIR/inspect.basic.elf.txt"
INSPECT_EXTERN_ELF_TXT="$TMP_DIR/inspect.extern.elf.txt"
INSPECT_MIXED_ASM_TXT="$TMP_DIR/inspect.mixed.asm.txt"
INSPECT_BASIC_KRBO_JSON="$TMP_DIR/inspect.basic.krbo.json"
INSPECT_BASIC_ELF_JSON="$TMP_DIR/inspect.basic.elf.json"
INSPECT_MIXED_ASM_JSON="$TMP_DIR/inspect.mixed.asm.json"
VERIFY_BASIC_KRBO_JSON="$TMP_DIR/verify.basic.krbo.json"
VERIFY_BASIC_ELF_JSON="$TMP_DIR/verify.basic.elf.json"

RELINKED_BASIC_ELF_OUT="$TMP_DIR/basic.relinked.o"
FINAL_RUNTIME_STUB_SRC="$TMP_DIR/runtime_stub.s"
FINAL_RUNTIME_STUB_OBJ="$TMP_DIR/runtime_stub.o"
FINAL_EXTERN_ELF_OUT="$TMP_DIR/extern_call_object.elf.final"
FINAL_MIXED_ELF_OUT="$TMP_DIR/extern_internal_chain.elf.final"
FINAL_EXTERN_ASM_OUT="$TMP_DIR/extern_call_object.asm.final"
FINAL_MIXED_ASM_OUT="$TMP_DIR/extern_internal_chain.asm.final"

set_context() {
  local fixture="$1"
  shift || true
  CURRENT_FIXTURE="$fixture"
  CURRENT_ARTIFACTS=("$@")
}

step() {
  local label="$1"
  local fn="$2"
  CURRENT_STEP="$label"
  ACCEPTANCE_LAST_READELF_HEADER=""
  ACCEPTANCE_LAST_READELF_SYMBOLS=""
  ACCEPTANCE_LAST_RELOCS_TEXT=""
  echo "==> $label"
  "$fn"
}

run_kernriftc() {
  "$CARGO_CMD" run -q -p kernriftc -- "$@"
}

emit_artifact_with_meta() {
  local emit_kind="$1"
  local output_path="$2"
  local meta_path="$3"
  local fixture="$4"

  run_kernriftc \
    "--emit=${emit_kind}" \
    -o "$output_path" \
    --meta-out "$meta_path" \
    "$fixture"
  acceptance_assert_nonempty_file "$output_path"
  acceptance_assert_nonempty_file "$meta_path"
}

emit_artifact() {
  local emit_kind="$1"
  local output_path="$2"
  local fixture="$3"

  run_kernriftc \
    "--emit=${emit_kind}" \
    -o "$output_path" \
    "$fixture"
  acceptance_assert_nonempty_file "$output_path"
}

verify_artifact_meta() {
  local artifact_path="$1"
  local meta_path="$2"

  run_kernriftc \
    verify-artifact-meta \
    "$artifact_path" \
    "$meta_path"
}

inspect_artifact_text() {
  local artifact_path="$1"
  local output_path="$2"
  run_kernriftc inspect-artifact "$artifact_path" >"$output_path"
  acceptance_assert_nonempty_file "$output_path"
}

inspect_artifact_json() {
  local artifact_path="$1"
  local output_path="$2"
  run_kernriftc inspect-artifact "$artifact_path" --format json >"$output_path"
  acceptance_assert_nonempty_file "$output_path"
}

verify_artifact_meta_json() {
  local artifact_path="$1"
  local meta_path="$2"
  local output_path="$3"
  run_kernriftc \
    verify-artifact-meta \
    --format json \
    "$artifact_path" \
    "$meta_path" >"$output_path"
  acceptance_assert_nonempty_file "$output_path"
}

assert_asm_line() {
  local line="$1"
  local file="$2"
  grep -q "^${line}$" "$file"
}

inspect_final_linked_artifact() {
  local readelf_tool="$1"
  local path="$2"

  acceptance_assert_elf_exec_x86_64 "$readelf_tool" "$path"
  acceptance_assert_symbol_present "$readelf_tool" "$path" "entry"
  acceptance_assert_symbol_present "$readelf_tool" "$path" "ext"
  acceptance_assert_symbol_not_undefined "$readelf_tool" "$path" "ext"
  acceptance_assert_relocation_not_references_symbol "$readelf_tool" "$path" "ext"
}

step_emit_internal_artifacts() {
  set_context "$BASIC_FIXTURE" "$BASIC_KRBO_OUT" "$BASIC_KRBO_META" "$BASIC_ELF_OUT" "$BASIC_ELF_META" "$BASIC_ASM_OUT"
  emit_artifact_with_meta "krbo" "$BASIC_KRBO_OUT" "$BASIC_KRBO_META" "$BASIC_FIXTURE"
  emit_artifact_with_meta "elfobj" "$BASIC_ELF_OUT" "$BASIC_ELF_META" "$BASIC_FIXTURE"
  emit_artifact "asm" "$BASIC_ASM_OUT" "$BASIC_FIXTURE"
}

step_verify_internal_sidecars() {
  set_context "$BASIC_FIXTURE" "$BASIC_KRBO_OUT" "$BASIC_KRBO_META" "$BASIC_ELF_OUT" "$BASIC_ELF_META" "$VERIFY_BASIC_KRBO_JSON" "$VERIFY_BASIC_ELF_JSON"
  verify_artifact_meta "$BASIC_KRBO_OUT" "$BASIC_KRBO_META"
  verify_artifact_meta "$BASIC_ELF_OUT" "$BASIC_ELF_META"
  verify_artifact_meta_json "$BASIC_KRBO_OUT" "$BASIC_KRBO_META" "$VERIFY_BASIC_KRBO_JSON"
  verify_artifact_meta_json "$BASIC_ELF_OUT" "$BASIC_ELF_META" "$VERIFY_BASIC_ELF_JSON"

  grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"kernrift_verify_artifact_meta_v1"' "$VERIFY_BASIC_KRBO_JSON"
  grep -Eq '"result"[[:space:]]*:[[:space:]]*"pass"' "$VERIFY_BASIC_KRBO_JSON"
  grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"kernrift_verify_artifact_meta_v1"' "$VERIFY_BASIC_ELF_JSON"
  grep -Eq '"result"[[:space:]]*:[[:space:]]*"pass"' "$VERIFY_BASIC_ELF_JSON"
}

step_emit_simple_extern_artifacts() {
  set_context "$EXTERN_FIXTURE" "$EXTERN_ELF_OUT" "$EXTERN_ELF_META" "$EXTERN_ASM_OUT"
  emit_artifact_with_meta "elfobj" "$EXTERN_ELF_OUT" "$EXTERN_ELF_META" "$EXTERN_FIXTURE"
  emit_artifact "asm" "$EXTERN_ASM_OUT" "$EXTERN_FIXTURE"
}

step_verify_simple_extern_sidecar() {
  set_context "$EXTERN_FIXTURE" "$EXTERN_ELF_OUT" "$EXTERN_ELF_META"
  verify_artifact_meta "$EXTERN_ELF_OUT" "$EXTERN_ELF_META"
}

step_emit_mixed_artifacts() {
  set_context "$MIXED_FIXTURE" "$MIXED_ELF_OUT" "$MIXED_ELF_META" "$MIXED_ASM_OUT"
  emit_artifact_with_meta "elfobj" "$MIXED_ELF_OUT" "$MIXED_ELF_META" "$MIXED_FIXTURE"
  emit_artifact "asm" "$MIXED_ASM_OUT" "$MIXED_FIXTURE"
}

step_verify_mixed_sidecar() {
  set_context "$MIXED_FIXTURE" "$MIXED_ELF_OUT" "$MIXED_ELF_META"
  verify_artifact_meta "$MIXED_ELF_OUT" "$MIXED_ELF_META"
}

step_check_asm_text_shapes() {
  set_context "$BASIC_FIXTURE" "$BASIC_ASM_OUT" "$EXTERN_ASM_OUT" "$MIXED_ASM_OUT"

  assert_asm_line "\\.text" "$BASIC_ASM_OUT"
  assert_asm_line "\\.globl bar" "$BASIC_ASM_OUT"
  assert_asm_line "bar:" "$BASIC_ASM_OUT"
  assert_asm_line "\\.globl foo" "$BASIC_ASM_OUT"
  assert_asm_line "foo:" "$BASIC_ASM_OUT"
  assert_asm_line "    call bar" "$BASIC_ASM_OUT"
  assert_asm_line "    ret" "$BASIC_ASM_OUT"

  assert_asm_line "\\.text" "$EXTERN_ASM_OUT"
  assert_asm_line "\\.globl entry" "$EXTERN_ASM_OUT"
  assert_asm_line "entry:" "$EXTERN_ASM_OUT"
  assert_asm_line "    call ext" "$EXTERN_ASM_OUT"
  assert_asm_line "    ret" "$EXTERN_ASM_OUT"

  assert_asm_line "\\.text" "$MIXED_ASM_OUT"
  assert_asm_line "\\.globl entry" "$MIXED_ASM_OUT"
  assert_asm_line "entry:" "$MIXED_ASM_OUT"
  assert_asm_line "    call helper" "$MIXED_ASM_OUT"
  assert_asm_line "\\.globl helper" "$MIXED_ASM_OUT"
  assert_asm_line "helper:" "$MIXED_ASM_OUT"
  assert_asm_line "    call ext" "$MIXED_ASM_OUT"
  assert_asm_line "    ret" "$MIXED_ASM_OUT"
}

step_inspect_artifact_cli_smoke() {
  set_context "$BASIC_FIXTURE" "$BASIC_KRBO_OUT" "$BASIC_ELF_OUT" "$EXTERN_ELF_OUT" "$MIXED_ASM_OUT" "$INSPECT_BASIC_ELF_TXT" "$INSPECT_EXTERN_ELF_TXT" "$INSPECT_MIXED_ASM_TXT" "$INSPECT_BASIC_KRBO_JSON" "$INSPECT_BASIC_ELF_JSON" "$INSPECT_MIXED_ASM_JSON"

  inspect_artifact_text "$BASIC_ELF_OUT" "$INSPECT_BASIC_ELF_TXT"
  inspect_artifact_text "$EXTERN_ELF_OUT" "$INSPECT_EXTERN_ELF_TXT"
  inspect_artifact_text "$MIXED_ASM_OUT" "$INSPECT_MIXED_ASM_TXT"
  inspect_artifact_json "$BASIC_KRBO_OUT" "$INSPECT_BASIC_KRBO_JSON"
  inspect_artifact_json "$BASIC_ELF_OUT" "$INSPECT_BASIC_ELF_JSON"
  inspect_artifact_json "$MIXED_ASM_OUT" "$INSPECT_MIXED_ASM_JSON"

  grep -q "^Artifact: elf_relocatable$" "$INSPECT_BASIC_ELF_TXT"
  grep -q "^Defined symbols:$" "$INSPECT_BASIC_ELF_TXT"
  grep -q "^- foo$" "$INSPECT_BASIC_ELF_TXT"

  grep -q "^Artifact: elf_relocatable$" "$INSPECT_EXTERN_ELF_TXT"
  grep -q "^Undefined symbols:$" "$INSPECT_EXTERN_ELF_TXT"
  grep -q "^- ext$" "$INSPECT_EXTERN_ELF_TXT"
  grep -q "^- \\.rela\\.text R_X86_64_PLT32 -> ext$" "$INSPECT_EXTERN_ELF_TXT"

  grep -q "^Artifact: asm_text$" "$INSPECT_MIXED_ASM_TXT"
  grep -q "^ASM direct call targets:$" "$INSPECT_MIXED_ASM_TXT"
  grep -q "^- helper$" "$INSPECT_MIXED_ASM_TXT"
  grep -q "^- ext$" "$INSPECT_MIXED_ASM_TXT"

  grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"kernrift_inspect_artifact_v1"' "$INSPECT_BASIC_KRBO_JSON"
  grep -Eq '"artifact_kind"[[:space:]]*:[[:space:]]*"krbo"' "$INSPECT_BASIC_KRBO_JSON"
  grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"kernrift_inspect_artifact_v1"' "$INSPECT_BASIC_ELF_JSON"
  grep -Eq '"artifact_kind"[[:space:]]*:[[:space:]]*"elf_relocatable"' "$INSPECT_BASIC_ELF_JSON"
  grep -Eq '"schema_version"[[:space:]]*:[[:space:]]*"kernrift_inspect_artifact_v1"' "$INSPECT_MIXED_ASM_JSON"
  grep -Eq '"artifact_kind"[[:space:]]*:[[:space:]]*"asm_text"' "$INSPECT_MIXED_ASM_JSON"
}

step_optional_elf_inspection_matrix() {
  set_context "$BASIC_FIXTURE" "$BASIC_ELF_OUT" "$EXTERN_ELF_OUT" "$MIXED_ELF_OUT"

  if READELF_TOOL="$(acceptance_find_readelf)"; then
    echo "[optional] inspect emitted elf objects with $READELF_TOOL"

    acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$BASIC_ELF_OUT"
    acceptance_assert_symbol_present "$READELF_TOOL" "$BASIC_ELF_OUT" "bar"
    acceptance_assert_symbol_present "$READELF_TOOL" "$BASIC_ELF_OUT" "foo"
    acceptance_assert_no_relocations "$READELF_TOOL" "$BASIC_ELF_OUT"

    acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$EXTERN_ELF_OUT"
    acceptance_assert_symbol_present "$READELF_TOOL" "$EXTERN_ELF_OUT" "entry"
    acceptance_assert_symbol_present "$READELF_TOOL" "$EXTERN_ELF_OUT" "ext"
    acceptance_assert_symbol_undefined "$READELF_TOOL" "$EXTERN_ELF_OUT" "ext"
    acceptance_assert_rela_text_present "$READELF_TOOL" "$EXTERN_ELF_OUT"
    acceptance_assert_relocation_references_symbol "$READELF_TOOL" "$EXTERN_ELF_OUT" "ext"

    acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$MIXED_ELF_OUT"
    acceptance_assert_symbol_present "$READELF_TOOL" "$MIXED_ELF_OUT" "entry"
    acceptance_assert_symbol_present "$READELF_TOOL" "$MIXED_ELF_OUT" "helper"
    acceptance_assert_symbol_present "$READELF_TOOL" "$MIXED_ELF_OUT" "ext"
    acceptance_assert_symbol_undefined "$READELF_TOOL" "$MIXED_ELF_OUT" "ext"
    acceptance_assert_symbol_not_undefined "$READELF_TOOL" "$MIXED_ELF_OUT" "helper"
    acceptance_assert_rela_text_present "$READELF_TOOL" "$MIXED_ELF_OUT"
    acceptance_assert_relocation_references_symbol "$READELF_TOOL" "$MIXED_ELF_OUT" "ext"
    acceptance_assert_relocation_not_references_symbol "$READELF_TOOL" "$MIXED_ELF_OUT" "helper"
  else
    acceptance_optional_skip "emitted-ELF inspection matrix" "readelf/llvm-readelf not found"
  fi
}

step_optional_downstream_matrix() {
  set_context "$EXTERN_FIXTURE" "$RELINKED_BASIC_ELF_OUT" "$FINAL_EXTERN_ELF_OUT" "$FINAL_MIXED_ELF_OUT" "$FINAL_EXTERN_ASM_OUT" "$FINAL_MIXED_ASM_OUT"

  if LINKER_TOOL="$(acceptance_find_linker)"; then
    echo "[optional] relocatable relink smoke with $LINKER_TOOL"
    "$LINKER_TOOL" -r "$BASIC_ELF_OUT" -o "$RELINKED_BASIC_ELF_OUT"
    acceptance_assert_nonempty_file "$RELINKED_BASIC_ELF_OUT"
    if READELF_TOOL="$(acceptance_find_readelf)"; then
      acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$RELINKED_BASIC_ELF_OUT"
    fi
  else
    acceptance_optional_skip "relocatable relink smoke" "ld.lld/ld not found"
  fi

  if LINKER_TOOL="$(acceptance_find_linker)" && ASM_COMPILER="$(acceptance_find_asm_compiler)"; then
    echo "[optional] final-link + hosted runtime for emitted elfobj with $LINKER_TOOL + $ASM_COMPILER"
    acceptance_write_hosted_runtime_stub "$FINAL_RUNTIME_STUB_SRC" "entry" "ext"
    acceptance_assemble_source "$ASM_COMPILER" "$FINAL_RUNTIME_STUB_SRC" "$FINAL_RUNTIME_STUB_OBJ"
    acceptance_assert_nonempty_file "$FINAL_RUNTIME_STUB_OBJ"

    "$LINKER_TOOL" -m elf_x86_64 -e _start -o "$FINAL_EXTERN_ELF_OUT" "$EXTERN_ELF_OUT" "$FINAL_RUNTIME_STUB_OBJ"
    acceptance_assert_nonempty_file "$FINAL_EXTERN_ELF_OUT"
    "$LINKER_TOOL" -m elf_x86_64 -e _start -o "$FINAL_MIXED_ELF_OUT" "$MIXED_ELF_OUT" "$FINAL_RUNTIME_STUB_OBJ"
    acceptance_assert_nonempty_file "$FINAL_MIXED_ELF_OUT"

    if READELF_TOOL="$(acceptance_find_readelf)"; then
      inspect_final_linked_artifact "$READELF_TOOL" "$FINAL_EXTERN_ELF_OUT"
      inspect_final_linked_artifact "$READELF_TOOL" "$FINAL_MIXED_ELF_OUT"
    fi

    acceptance_run_binary_expect_exit_or_skip "$FINAL_EXTERN_ELF_OUT" "emitted-elfobj-extern" 7 "$TMP_DIR/emitted-elfobj-extern.stderr" "$TMP_DIR"
    acceptance_run_binary_expect_exit_or_skip "$FINAL_MIXED_ELF_OUT" "emitted-elfobj-mixed" 7 "$TMP_DIR/emitted-elfobj-mixed.stderr" "$TMP_DIR"
  else
    missing_tools=()
    if ! LINKER_TOOL="$(acceptance_find_linker)"; then
      missing_tools+=("ld.lld/ld")
    fi
    if ! ASM_COMPILER="$(acceptance_find_asm_compiler)"; then
      missing_tools+=("as/clang/gcc")
    fi
    acceptance_optional_skip "final-link + hosted runtime for emitted elfobj" "${missing_tools[*]}"
  fi

  if ASM_COMPILER="$(acceptance_find_asm_compiler)" && READELF_TOOL="$(acceptance_find_readelf)"; then
    echo "[optional] assemble emitted asm and inspect relocations with $ASM_COMPILER + $READELF_TOOL"
    acceptance_assemble_source "$ASM_COMPILER" "$EXTERN_ASM_OUT" "$EXTERN_ASM_OBJ"
    acceptance_assert_nonempty_file "$EXTERN_ASM_OBJ"
    acceptance_assemble_source "$ASM_COMPILER" "$MIXED_ASM_OUT" "$MIXED_ASM_OBJ"
    acceptance_assert_nonempty_file "$MIXED_ASM_OBJ"

    acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$EXTERN_ASM_OBJ"
    acceptance_assert_rela_text_present "$READELF_TOOL" "$EXTERN_ASM_OBJ"
    acceptance_assert_relocation_references_symbol "$READELF_TOOL" "$EXTERN_ASM_OBJ" "ext"

    acceptance_assert_elf_rel_x86_64 "$READELF_TOOL" "$MIXED_ASM_OBJ"
    acceptance_assert_rela_text_present "$READELF_TOOL" "$MIXED_ASM_OBJ"
    acceptance_assert_relocation_references_symbol "$READELF_TOOL" "$MIXED_ASM_OBJ" "helper"
    acceptance_assert_relocation_references_symbol "$READELF_TOOL" "$MIXED_ASM_OBJ" "ext"
  else
    missing_tools=()
    if ! ASM_COMPILER="$(acceptance_find_asm_compiler)"; then
      missing_tools+=("as/clang/gcc")
    fi
    if ! READELF_TOOL="$(acceptance_find_readelf)"; then
      missing_tools+=("readelf/llvm-readelf")
    fi
    acceptance_optional_skip "assemble emitted asm and inspect relocations" "${missing_tools[*]}"
  fi

  if LINKER_TOOL="$(acceptance_find_linker)" && ASM_COMPILER="$(acceptance_find_asm_compiler)"; then
    echo "[optional] final-link + hosted runtime for emitted asm with $LINKER_TOOL + $ASM_COMPILER"

    acceptance_assemble_source "$ASM_COMPILER" "$EXTERN_ASM_OUT" "$EXTERN_ASM_OBJ"
    acceptance_assert_nonempty_file "$EXTERN_ASM_OBJ"
    acceptance_assemble_source "$ASM_COMPILER" "$MIXED_ASM_OUT" "$MIXED_ASM_OBJ"
    acceptance_assert_nonempty_file "$MIXED_ASM_OBJ"

    if [[ ! -s "$FINAL_RUNTIME_STUB_OBJ" ]]; then
      acceptance_write_hosted_runtime_stub "$FINAL_RUNTIME_STUB_SRC" "entry" "ext"
      acceptance_assemble_source "$ASM_COMPILER" "$FINAL_RUNTIME_STUB_SRC" "$FINAL_RUNTIME_STUB_OBJ"
      acceptance_assert_nonempty_file "$FINAL_RUNTIME_STUB_OBJ"
    fi

    "$LINKER_TOOL" -m elf_x86_64 -e _start -o "$FINAL_EXTERN_ASM_OUT" "$EXTERN_ASM_OBJ" "$FINAL_RUNTIME_STUB_OBJ"
    acceptance_assert_nonempty_file "$FINAL_EXTERN_ASM_OUT"
    "$LINKER_TOOL" -m elf_x86_64 -e _start -o "$FINAL_MIXED_ASM_OUT" "$MIXED_ASM_OBJ" "$FINAL_RUNTIME_STUB_OBJ"
    acceptance_assert_nonempty_file "$FINAL_MIXED_ASM_OUT"

    if READELF_TOOL="$(acceptance_find_readelf)"; then
      inspect_final_linked_artifact "$READELF_TOOL" "$FINAL_EXTERN_ASM_OUT"
      inspect_final_linked_artifact "$READELF_TOOL" "$FINAL_MIXED_ASM_OUT"
    fi

    acceptance_run_binary_expect_exit_or_skip "$FINAL_EXTERN_ASM_OUT" "emitted-asm-extern" 7 "$TMP_DIR/emitted-asm-extern.stderr" "$TMP_DIR"
    acceptance_run_binary_expect_exit_or_skip "$FINAL_MIXED_ASM_OUT" "emitted-asm-mixed" 7 "$TMP_DIR/emitted-asm-mixed.stderr" "$TMP_DIR"
  else
    missing_tools=()
    if ! LINKER_TOOL="$(acceptance_find_linker)"; then
      missing_tools+=("ld.lld/ld")
    fi
    if ! ASM_COMPILER="$(acceptance_find_asm_compiler)"; then
      missing_tools+=("as/clang/gcc")
    fi
    acceptance_optional_skip "final-link + hosted runtime for emitted asm" "${missing_tools[*]}"
  fi
}

step_complete() {
  echo "kernriftc artifact export acceptance: PASS"
}

if [[ "${ACCEPTANCE_KEEP_TMP:-0}" == "1" ]]; then
  echo "[info] using tmp dir: $TMP_DIR"
fi

step "[1/11] internal-only fixture: emit krbo/elfobj/asm" step_emit_internal_artifacts
step "[2/11] internal-only fixture: verify sidecars" step_verify_internal_sidecars
step "[3/11] simple extern fixture: emit elfobj/asm" step_emit_simple_extern_artifacts
step "[4/11] simple extern fixture: verify elfobj sidecar" step_verify_simple_extern_sidecar
step "[5/11] mixed internal+extern fixture: emit elfobj/asm" step_emit_mixed_artifacts
step "[6/11] mixed internal+extern fixture: verify elfobj sidecar" step_verify_mixed_sidecar
step "[7/11] asm text structure smoke" step_check_asm_text_shapes
step "[8/11] inspect-artifact CLI smoke" step_inspect_artifact_cli_smoke
step "[9/11] optional emitted-ELF inspection matrix" step_optional_elf_inspection_matrix
step "[10/11] optional downstream relink/final-link/runtime matrix" step_optional_downstream_matrix
step "[11/11] hosted artifact matrix complete" step_complete
