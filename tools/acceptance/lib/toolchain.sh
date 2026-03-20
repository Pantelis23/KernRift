#!/usr/bin/env bash
set -euo pipefail

acceptance_optional_tool_discovery_disabled() {
  [[ "${ACCEPTANCE_DISABLE_OPTIONAL_TOOL_DISCOVERY:-0}" == "1" ]]
}

acceptance_find_readelf() {
  if acceptance_optional_tool_discovery_disabled; then
    return 1
  fi
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

acceptance_find_linker() {
  if acceptance_optional_tool_discovery_disabled; then
    return 1
  fi
  if command -v ld.lld >/dev/null 2>&1; then
    printf '%s\n' "ld.lld"
    return 0
  fi
  if command -v ld >/dev/null 2>&1; then
    printf '%s\n' "ld"
    return 0
  fi
  return 1
}

acceptance_find_c_compiler() {
  if acceptance_optional_tool_discovery_disabled; then
    return 1
  fi
  if command -v cc >/dev/null 2>&1; then
    printf '%s\n' "cc"
    return 0
  fi
  if command -v clang >/dev/null 2>&1; then
    printf '%s\n' "clang"
    return 0
  fi
  if command -v gcc >/dev/null 2>&1; then
    printf '%s\n' "gcc"
    return 0
  fi
  return 1
}

acceptance_find_asm_compiler() {
  if acceptance_optional_tool_discovery_disabled; then
    return 1
  fi
  if command -v as >/dev/null 2>&1; then
    printf '%s\n' "as"
    return 0
  fi
  if command -v clang >/dev/null 2>&1; then
    printf '%s\n' "clang"
    return 0
  fi
  if command -v gcc >/dev/null 2>&1; then
    printf '%s\n' "gcc"
    return 0
  fi
  return 1
}
