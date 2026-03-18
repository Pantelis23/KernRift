#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

export CARGO_BUILD_JOBS=1

run_step() {
  echo "==> $*"
  "$@"
}

run_step cargo fmt --all
run_step cargo test -p parser -- --test-threads=1
run_step cargo test -p hir -- --test-threads=1
run_step cargo test -p krir -- --test-threads=1
run_step cargo test -p kernriftc --test cli_contract -- --test-threads=1
run_step cargo test -p kernriftc --test golden -- --test-threads=1
run_step cargo test -p kernriftc -- --test-threads=1
run_step cargo clippy --workspace --all-targets -- -D warnings
run_step ./tools/acceptance/krir_v0_1.sh
run_step ./tools/acceptance/kernriftc_artifact_exports.sh
run_step ./tools/acceptance/all.sh
run_step cargo run -q -p kernriftc -- --selftest
run_step cargo build --release
