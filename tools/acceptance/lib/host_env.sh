#!/usr/bin/env bash
set -euo pipefail

acceptance_host_is_linux() {
  [[ "$(uname -s)" == "Linux" ]]
}

acceptance_host_is_x86_64() {
  [[ "$(uname -m)" == "x86_64" ]]
}

acceptance_tmp_is_exec() {
  local dir="$1"
  local probe_script="${dir%/}/.acceptance-exec-probe-$$.sh"

  if ! { cat > "$probe_script" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  } 2>/dev/null
  then
    return 1
  fi

  chmod +x "$probe_script" 2>/dev/null || {
    rm -f "$probe_script"
    return 1
  }

  if "$probe_script" >/dev/null 2>&1; then
    rm -f "$probe_script"
    return 0
  fi

  rm -f "$probe_script"
  return 1
}

acceptance_can_execute_binaries() {
  local exec_dir="${1:-${TMPDIR:-/tmp}}"

  if [[ "${ACCEPTANCE_DISABLE_RUNTIME_SMOKE:-0}" == "1" ]]; then
    printf '%s' "disabled by ACCEPTANCE_DISABLE_RUNTIME_SMOKE=1"
    return 1
  fi

  if ! acceptance_host_is_linux; then
    printf '%s' "host is not Linux"
    return 1
  fi

  if ! acceptance_host_is_x86_64; then
    printf '%s' "host is not x86_64"
    return 1
  fi

  if ! acceptance_tmp_is_exec "$exec_dir"; then
    printf '%s' "tmp execution unavailable (noexec or restricted): ${exec_dir}"
    return 1
  fi

  return 0
}
