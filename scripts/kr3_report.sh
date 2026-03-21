#!/usr/bin/env sh
# KR3 safety and binary report.
#
# Prints:
#   1. Compile-time rejected bug classes (static guarantees over C)
#   2. UART driver ELF object size (text section)
#   3. Reminder to run KR3 contract tests for full validation.
#
# Usage: ./scripts/kr3_report.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

UART_KR="examples/kernel/uart_driver.kr"

echo "=== KernRift KR3 Safety Report ==="
echo ""

echo "Compile-time rejected bug classes (vs C):"
echo "  1. Lock-order inversion        — LOCK_ORDER_INVERSION (lock graph DFS)"
echo "  2. Yield in IRQ context        — CTX_DISALLOWED (ctx_check pass)"
echo "  3. Missing capability          — CAP_DISALLOWED (cap_check pass)"
echo "  4. Undeclared lock class       — HIR validation"
echo "  5. Scheduler hook without @noyield — HOOK_MISSING_NOYIELD (sched_hook_check)"
echo "  6. Undeclared per-cpu variable — HIR validation"
echo "  7. Blocking call from irq/nmi  — EFF_DISALLOWED (effect_check pass)"
echo "  8. Allocation in critical section — CRITICAL_ALLOC (boundary check)"
echo "  9. Unbalanced acquire/release  — LOCK_BUDGET_EXCEEDED (critical-region pass)"
echo ""

echo "UART driver semantic check:"
if cargo run -q --bin kernriftc -- check "$UART_KR" 2>/dev/null; then
    echo "  $UART_KR — PASS"
else
    echo "  $UART_KR — FAIL (see cargo run --bin kernriftc -- check $UART_KR)"
fi
echo ""

echo "UART driver lock graph:"
cargo run -q --bin kernriftc -- --emit lockgraph "$UART_KR" 2>/dev/null \
    | python3 -c "
import sys, json
data = json.load(sys.stdin)
edges = data.get('lock_edges', [])
if edges:
    for e in edges:
        print('  ' + e['from'] + ' -> ' + e['to'])
else:
    print('  (no lock edges)')
" 2>/dev/null || echo "  (lockgraph output unavailable)"
echo ""

echo "Run KR3 contract tests:"
echo "  cargo test --test kr3_contract"
echo ""
echo "Run all KR0-KR3 contract tests:"
echo "  cargo test"
