#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

run_acceptance() {
  local script="$1"
  echo "==> ${script}"
  "./tools/acceptance/${script}"
}

run_acceptance "krir_v0_1.sh"
run_acceptance "kernriftc_artifact_exports.sh"

echo "all acceptance: PASS"
