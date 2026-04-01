#!/bin/bash
# KernRift vs C vs Rust Benchmark Suite
set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
KRC="${KRC:-$DIR/../build/krc2}"
RESULTS="$DIR/results.md"

echo "=== KernRift Benchmark Suite ===" | tee "$RESULTS"
echo "Date: $(date -u)" | tee -a "$RESULTS"
echo "CPU: $(lscpu | grep 'Model name' | sed 's/.*: *//')" | tee -a "$RESULTS"
echo "" | tee -a "$RESULTS"

# Time a command, print result in ms
bench() {
    local label="$1"
    shift
    # Run 3 times, take median
    local times=()
    for run in 1 2 3; do
        local start=$(date +%s%N)
        "$@" >/dev/null 2>&1 || true
        local end=$(date +%s%N)
        local ms=$(( (end - start) / 1000000 ))
        times+=($ms)
    done
    # Sort and take median
    IFS=$'\n' sorted=($(sort -n <<<"${times[*]}")); unset IFS
    echo "$label: ${sorted[1]}ms (runs: ${times[0]}, ${times[1]}, ${times[2]})"
}

compile_bench() {
    local name="$1"
    echo "--- $name ---" | tee -a "$RESULTS"

    # Compile KernRift
    echo "  Compiling KernRift..."
    local start=$(date +%s%N)
    $KRC --arch=x86_64 "$DIR/$name.kr" -o "$DIR/$name.krc.bin" 2>/dev/null
    local end=$(date +%s%N)
    local kr_compile_ms=$(( (end - start) / 1000000 ))
    chmod +x "$DIR/$name.krc.bin"

    # Compile C (-O0 for fair comparison, -O2 for optimized)
    local start=$(date +%s%N)
    gcc -O0 -o "$DIR/$name.c.O0.bin" "$DIR/$name.c" 2>/dev/null
    local end=$(date +%s%N)
    local c_O0_compile_ms=$(( (end - start) / 1000000 ))

    local start=$(date +%s%N)
    gcc -O2 -o "$DIR/$name.c.O2.bin" "$DIR/$name.c" 2>/dev/null
    local end=$(date +%s%N)
    local c_O2_compile_ms=$(( (end - start) / 1000000 ))

    # Compile Rust (debug and release)
    local start=$(date +%s%N)
    rustc -o "$DIR/$name.rs.dbg.bin" "$DIR/$name.rs" 2>/dev/null
    local end=$(date +%s%N)
    local rs_dbg_compile_ms=$(( (end - start) / 1000000 ))

    local start=$(date +%s%N)
    rustc -C opt-level=2 -o "$DIR/$name.rs.rel.bin" "$DIR/$name.rs" 2>/dev/null
    local end=$(date +%s%N)
    local rs_rel_compile_ms=$(( (end - start) / 1000000 ))

    # Binary sizes
    local kr_size=$(stat -c%s "$DIR/$name.krc.bin" 2>/dev/null || echo 0)
    local c_O0_size=$(stat -c%s "$DIR/$name.c.O0.bin" 2>/dev/null || echo 0)
    local c_O2_size=$(stat -c%s "$DIR/$name.c.O2.bin" 2>/dev/null || echo 0)
    local rs_dbg_size=$(stat -c%s "$DIR/$name.rs.dbg.bin" 2>/dev/null || echo 0)
    local rs_rel_size=$(stat -c%s "$DIR/$name.rs.rel.bin" 2>/dev/null || echo 0)

    echo "" | tee -a "$RESULTS"
    echo "### $name" | tee -a "$RESULTS"
    echo "" | tee -a "$RESULTS"
    echo "**Compile time:**" | tee -a "$RESULTS"
    echo "| Compiler | Time |" | tee -a "$RESULTS"
    echo "|----------|------|" | tee -a "$RESULTS"
    echo "| krc (self-hosted) | ${kr_compile_ms}ms |" | tee -a "$RESULTS"
    echo "| gcc -O0 | ${c_O0_compile_ms}ms |" | tee -a "$RESULTS"
    echo "| gcc -O2 | ${c_O2_compile_ms}ms |" | tee -a "$RESULTS"
    echo "| rustc (debug) | ${rs_dbg_compile_ms}ms |" | tee -a "$RESULTS"
    echo "| rustc -O2 | ${rs_rel_compile_ms}ms |" | tee -a "$RESULTS"
    echo "" | tee -a "$RESULTS"

    echo "**Binary size:**" | tee -a "$RESULTS"
    echo "| Binary | Size |" | tee -a "$RESULTS"
    echo "|--------|------|" | tee -a "$RESULTS"
    echo "| krc | ${kr_size} B |" | tee -a "$RESULTS"
    echo "| gcc -O0 | ${c_O0_size} B |" | tee -a "$RESULTS"
    echo "| gcc -O2 | ${c_O2_size} B |" | tee -a "$RESULTS"
    echo "| rustc debug | ${rs_dbg_size} B |" | tee -a "$RESULTS"
    echo "| rustc -O2 | ${rs_rel_size} B |" | tee -a "$RESULTS"
    echo "" | tee -a "$RESULTS"

    echo "**Runtime (median of 3):**" | tee -a "$RESULTS"
    echo "| Binary | Time |" | tee -a "$RESULTS"
    echo "|--------|------|" | tee -a "$RESULTS"
    bench "| krc" "$DIR/$name.krc.bin" | tee -a "$RESULTS"
    bench "| gcc -O0" "$DIR/$name.c.O0.bin" | tee -a "$RESULTS"
    bench "| gcc -O2" "$DIR/$name.c.O2.bin" | tee -a "$RESULTS"
    bench "| rustc debug" "$DIR/$name.rs.dbg.bin" | tee -a "$RESULTS"
    bench "| rustc -O2" "$DIR/$name.rs.rel.bin" | tee -a "$RESULTS"
    echo "" | tee -a "$RESULTS"
}

compile_bench "fib"
compile_bench "sort"
compile_bench "sieve"
compile_bench "matmul"

# Cleanup binaries
rm -f "$DIR"/*.bin

echo "=== Done ===" | tee -a "$RESULTS"
echo "Results saved to $RESULTS"
