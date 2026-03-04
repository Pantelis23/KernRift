#!/usr/bin/env bash
set -euo pipefail

ACCEPTANCE_LAST_READELF_HEADER=""
ACCEPTANCE_LAST_READELF_SYMBOLS=""
ACCEPTANCE_LAST_RELOCS_TEXT=""

acceptance_assemble_source() {
  local asm_compiler="$1"
  local src="$2"
  local out="$3"

  case "$asm_compiler" in
    as)
      "$asm_compiler" -o "$out" "$src"
      ;;
    *)
      "$asm_compiler" -c "$src" -o "$out"
      ;;
  esac
}

acceptance_write_hosted_runtime_stub() {
  local stub_path="$1"
  local entry_symbol="$2"
  local extern_symbol="$3"

  cat > "$stub_path" <<EOF_STUB
.text
.globl _start
.globl ${extern_symbol}
_start:
    call ${entry_symbol}
    mov \$60, %rax
    mov \$99, %rdi
    syscall
${extern_symbol}:
    mov \$60, %rax
    mov \$7, %rdi
    syscall
EOF_STUB
}

acceptance_readelf_header() {
  local readelf_tool="$1"
  local path="$2"
  ACCEPTANCE_LAST_READELF_HEADER="$($readelf_tool -h "$path")"
  printf '%s\n' "$ACCEPTANCE_LAST_READELF_HEADER"
}

acceptance_readelf_symbols() {
  local readelf_tool="$1"
  local path="$2"
  ACCEPTANCE_LAST_READELF_SYMBOLS="$($readelf_tool -sW "$path")"
  printf '%s\n' "$ACCEPTANCE_LAST_READELF_SYMBOLS"
}

acceptance_symbol_exists() {
  local symbols_text="$1"
  local symbol="$2"
  awk -v sym="$symbol" '$NF == sym { found = 1 } END { exit(found ? 0 : 1) }' <<<"$symbols_text"
}

acceptance_symbol_is_undefined() {
  local symbols_text="$1"
  local symbol="$2"
  awk -v sym="$symbol" '
    $NF == sym {
      for (i = 1; i <= NF; i++) {
        if ($i == "UND") {
          found = 1
        }
      }
    }
    END { exit(found ? 0 : 1) }
  ' <<<"$symbols_text"
}

acceptance_relocs_text() {
  local readelf_tool="$1"
  local path="$2"
  ACCEPTANCE_LAST_RELOCS_TEXT="$($readelf_tool -rW "$path")"
  printf '%s\n' "$ACCEPTANCE_LAST_RELOCS_TEXT"
}

acceptance_relocs_has_section() {
  local relocs_text="$1"
  local section_name="$2"
  awk -v sec="$section_name" 'index($0, "Relocation section") > 0 && index($0, sec) > 0 { found = 1 } END { exit(found ? 0 : 1) }' <<<"$relocs_text"
}

acceptance_relocs_refs_symbol() {
  local relocs_text="$1"
  local symbol="$2"
  awk -v sym="$symbol" '
    {
      pattern = "[[:space:]]" sym "([[:space:]]|$|[+-])"
      if ($0 ~ pattern) {
        found = 1
      }
    }
    END { exit(found ? 0 : 1) }
  ' <<<"$relocs_text"
}

acceptance_assert_elf_rel_x86_64() {
  local readelf_tool="$1"
  local path="$2"
  local header
  header="$(acceptance_readelf_header "$readelf_tool" "$path")"
  printf '%s\n' "$header" | grep -Eq 'Class:[[:space:]]+ELF64'
  printf '%s\n' "$header" | grep -Eq "Data:[[:space:]]+2's complement, little endian"
  printf '%s\n' "$header" | grep -Eq 'Type:[[:space:]]+REL'
  printf '%s\n' "$header" | grep -Eq 'Machine:[[:space:]]+(Advanced Micro Devices X86-64|x86-64)'
}

acceptance_assert_elf_exec_x86_64() {
  local readelf_tool="$1"
  local path="$2"
  local header
  header="$(acceptance_readelf_header "$readelf_tool" "$path")"
  printf '%s\n' "$header" | grep -Eq 'Class:[[:space:]]+ELF64'
  printf '%s\n' "$header" | grep -Eq "Data:[[:space:]]+2's complement, little endian"
  printf '%s\n' "$header" | grep -Eq 'Type:[[:space:]]+EXEC'
  printf '%s\n' "$header" | grep -Eq 'Machine:[[:space:]]+(Advanced Micro Devices X86-64|x86-64)'
}

acceptance_assert_symbol_present() {
  local readelf_tool="$1"
  local path="$2"
  local symbol="$3"
  local symbols
  symbols="$(acceptance_readelf_symbols "$readelf_tool" "$path")"
  if ! acceptance_symbol_exists "$symbols" "$symbol"; then
    echo "expected symbol '${symbol}' to be present in $path" >&2
    exit 1
  fi
}

acceptance_assert_symbol_undefined() {
  local readelf_tool="$1"
  local path="$2"
  local symbol="$3"
  local symbols
  symbols="$(acceptance_readelf_symbols "$readelf_tool" "$path")"
  if ! acceptance_symbol_is_undefined "$symbols" "$symbol"; then
    echo "expected symbol '${symbol}' to be undefined in $path" >&2
    exit 1
  fi
}

acceptance_assert_symbol_not_undefined() {
  local readelf_tool="$1"
  local path="$2"
  local symbol="$3"
  local symbols
  symbols="$(acceptance_readelf_symbols "$readelf_tool" "$path")"
  if acceptance_symbol_is_undefined "$symbols" "$symbol"; then
    echo "expected symbol '${symbol}' to be defined in $path" >&2
    exit 1
  fi
}

acceptance_assert_rela_text_present() {
  local readelf_tool="$1"
  local path="$2"
  local relocs
  relocs="$(acceptance_relocs_text "$readelf_tool" "$path")"
  if ! acceptance_relocs_has_section "$relocs" ".rela.text"; then
    echo "expected .rela.text relocations in $path" >&2
    exit 1
  fi
}

acceptance_assert_no_relocations() {
  local readelf_tool="$1"
  local path="$2"
  local relocs
  relocs="$(acceptance_relocs_text "$readelf_tool" "$path")"
  if grep -Eq "Relocation section '[^']+'" <<<"$relocs"; then
    echo "expected no relocations in $path" >&2
    exit 1
  fi
}

acceptance_assert_relocation_references_symbol() {
  local readelf_tool="$1"
  local path="$2"
  local symbol="$3"
  local relocs
  relocs="$(acceptance_relocs_text "$readelf_tool" "$path")"
  if ! acceptance_relocs_refs_symbol "$relocs" "$symbol"; then
    echo "expected relocation references to symbol '${symbol}' in $path" >&2
    exit 1
  fi
}

acceptance_assert_relocation_not_references_symbol() {
  local readelf_tool="$1"
  local path="$2"
  local symbol="$3"
  local relocs
  relocs="$(acceptance_relocs_text "$readelf_tool" "$path")"
  if acceptance_relocs_refs_symbol "$relocs" "$symbol"; then
    echo "expected no relocation references to symbol '${symbol}' in $path" >&2
    exit 1
  fi
}

acceptance_run_binary_expect_exit_or_skip() {
  local path="$1"
  local label="$2"
  local expected_exit="$3"
  local stderr_path="$4"
  local exec_probe_dir="${5:-${TMPDIR:-/tmp}}"
  local status=0
  local reason=""

  if ! reason="$(acceptance_can_execute_binaries "$exec_probe_dir")"; then
    acceptance_optional_skip "$label runtime smoke" "$reason"
    return 0
  fi

  chmod +x "$path" || true
  set +e
  "$path" >/dev/null 2>"$stderr_path"
  status=$?
  set -e

  if [[ "$status" -eq "$expected_exit" ]]; then
    return 0
  fi

  if [[ "$status" -eq 126 ]] && grep -Eq 'Permission denied|Operation not permitted' "$stderr_path"; then
    acceptance_optional_skip "$label runtime smoke" "execution unavailable"
    return 0
  fi

  echo "expected ${label} runtime smoke to exit ${expected_exit}, got ${status}" >&2
  if [[ -s "$stderr_path" ]]; then
    cat "$stderr_path" >&2
  fi
  exit 1
}
