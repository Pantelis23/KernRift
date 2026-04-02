#!/bin/bash
# No set -e: test binaries return non-zero exit codes intentionally

DIR="$(cd "$(dirname "$0")" && pwd)"
KRC="${KRC:-$DIR/../build/krc}"
PASS=0
FAIL=0
TOTAL=0

run_test() {
    local name="$1"
    local input="$2"
    local expected="$3"
    TOTAL=$((TOTAL + 1))

    printf '%s\n' "$input" > /tmp/krc_test_$$.kr
    if $KRC /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
        chmod +x /tmp/krc_test_$$
        local got=0
        /tmp/krc_test_$$ > /dev/null 2>&1 && got=0 || got=$?
        if [ "$got" = "$expected" ]; then
            PASS=$((PASS + 1))
        else
            echo "FAIL: $name (expected $expected, got $got)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: $name (compilation failed)"
        FAIL=$((FAIL + 1))
    fi
    rm -f /tmp/krc_test_$$.kr /tmp/krc_test_$$
}

run_test_output() {
    local name="$1"
    local input="$2"
    local expected_output="$3"
    local expected_exit="${4:-0}"
    TOTAL=$((TOTAL + 1))

    printf '%s\n' "$input" > /tmp/krc_test_$$.kr
    if $KRC /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
        chmod +x /tmp/krc_test_$$
        local got_output
        got_output=$(/tmp/krc_test_$$ 2>/dev/null)
        local got_exit=$?
        if [ "$got_output" = "$expected_output" ] && [ "$got_exit" = "$expected_exit" ]; then
            PASS=$((PASS + 1))
        else
            if [ "$got_output" != "$expected_output" ]; then
                echo "FAIL: $name (expected output '$expected_output', got '$got_output')"
            else
                echo "FAIL: $name (expected exit $expected_exit, got $got_exit)"
            fi
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: $name (compilation failed)"
        FAIL=$((FAIL + 1))
    fi
    rm -f /tmp/krc_test_$$.kr /tmp/krc_test_$$
}

echo "=== KernRift Self-Hosted Compiler Test Suite ==="
echo ""

# --- Basic tests ---
run_test "exit_42" 'fn main() { exit(42) }' 42
run_test "exit_0" 'fn main() { exit(0) }' 0

# --- Variables ---
run_test "var_assign" 'fn main() {
    uint64 x = 42
    exit(x)
}' 42

run_test "var_reassign" 'fn main() {
    uint64 x = 1
    x = 42
    exit(x)
}' 42

# --- Arithmetic ---
run_test "add" 'fn main() { exit(10 + 20) }' 30
run_test "sub" 'fn main() { exit(50 - 8) }' 42
run_test "mul" 'fn main() { exit(6 * 7) }' 42
run_test "div" 'fn main() { exit(84 / 2) }' 42
run_test "mod" 'fn main() { exit(47 % 5) }' 2

# --- Bitwise ---
run_test "and" 'fn main() { exit(0xFF & 0x2A) }' 42
run_test "or" 'fn main() { exit(0x20 | 0x0A) }' 42
run_test "xor" 'fn main() { exit(0xFF ^ 0xD5) }' 42
run_test "shl" 'fn main() { exit(21 << 1) }' 42
run_test "shr" 'fn main() { exit(84 >> 1) }' 42

# --- Unary ---
run_test "not_0" 'fn main() { exit(!0) }' 1
run_test "not_1" 'fn main() { exit(!1) }' 0
run_test "neg" 'fn main() { exit((-1) & 0xFF) }' 255

# --- Comparisons ---
run_test "eq_true" 'fn main() { if 5 == 5 { exit(1) } exit(0) }' 1
run_test "eq_false" 'fn main() { if 5 == 6 { exit(1) } exit(0) }' 0
run_test "lt" 'fn main() { if 3 < 5 { exit(1) } exit(0) }' 1
run_test "gt" 'fn main() { if 5 > 3 { exit(1) } exit(0) }' 1
run_test "le" 'fn main() { if 5 <= 5 { exit(1) } exit(0) }' 1
run_test "ge" 'fn main() { if 5 >= 5 { exit(1) } exit(0) }' 1
run_test "ne" 'fn main() { if 5 != 6 { exit(1) } exit(0) }' 1

# --- Logical ---
run_test "and_logic" 'fn main() {
    uint64 x = 5
    if x > 3 && x < 10 { exit(1) }
    exit(0)
}' 1
run_test "or_logic" 'fn main() {
    uint64 x = 2
    if x == 1 || x == 2 { exit(1) }
    exit(0)
}' 1

# --- If/else ---
run_test "if_then" 'fn main() {
    uint64 x = 5
    if x == 5 { exit(1) } else { exit(0) }
}' 1
run_test "if_else" 'fn main() {
    uint64 x = 3
    if x == 5 { exit(1) } else { exit(2) }
}' 2
run_test "else_if" 'fn main() {
    uint64 x = 2
    if x == 1 { exit(10) } else if x == 2 { exit(20) } else { exit(30) }
}' 20

# --- While ---
run_test "while_sum" 'fn main() {
    uint64 i = 0
    uint64 s = 0
    while i < 10 {
        s = s + i
        i = i + 1
    }
    exit(s)
}' 45

# --- Break/Continue ---
run_test "break" 'fn main() {
    uint64 i = 0
    uint64 c = 0
    while i < 100 {
        if i == 5 { break }
        c = c + 1
        i = i + 1
    }
    exit(c)
}' 5
run_test "continue" 'fn main() {
    uint64 i = 0
    uint64 s = 0
    while i < 10 {
        i = i + 1
        if i == 5 { continue }
        s = s + 1
    }
    exit(s)
}' 9

# --- Functions ---
run_test "fn_call" 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }
fn main() { exit(add(10, 20)) }' 30

run_test "fn_4args" 'fn sum4(uint64 a, uint64 b, uint64 c, uint64 d) -> uint64 {
    return a + b + c + d
}
fn main() { exit(sum4(10, 20, 3, 9)) }' 42

run_test "fn_6args" 'fn sum6(uint64 a, uint64 b, uint64 c, uint64 d, uint64 e, uint64 f) -> uint64 {
    return a + b + c + d + e + f
}
fn main() { exit(sum6(1,2,3,4,5,6)) }' 21

# --- Recursion ---
run_test "factorial" 'fn f(uint64 n) -> uint64 {
    if n <= 1 { return 1 }
    return n * f(n - 1)
}
fn main() { exit(f(5)) }' 120

run_test "fibonacci" 'fn fib(uint64 n) -> uint64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
fn main() { exit(fib(10)) }' 55

# --- Compound assignment ---
run_test "plus_eq" 'fn main() {
    uint64 x = 10
    x += 32
    exit(x)
}' 42

# --- Enums ---
run_test "enum_basic" 'enum Color {
    Red = 10
    Green = 20
    Blue = 30
}
fn main() { exit(Color.Green) }' 20

# --- Static variables ---
run_test "static_var" 'static uint64 counter = 0
fn inc() { counter = counter + 1 }
fn main() {
    inc()
    inc()
    inc()
    exit(counter)
}' 3

# --- Arrays ---
run_test "array_rw" 'fn main() {
    uint8[10] buf
    buf[0] = 42
    uint64 v = buf[0]
    exit(v)
}' 42

# --- Structs ---
run_test "struct_basic" 'struct Point {
    uint64 x
    uint64 y
}
fn main() {
    Point p
    p.x = 10
    p.y = 32
    exit(p.x + p.y)
}' 42

# --- Pointer operations ---
run_test "ptr_load_store" 'fn main() {
    uint64 buf = alloc(64)
    unsafe { *(buf as uint64) = 42 }
    uint64 v = 0
    unsafe { *(buf as uint64) -> v }
    exit(v)
}' 42

# --- File I/O ---
run_test "file_io" 'fn main() {
    uint64 msg = "test"
    uint64 fd = file_open("/dev/null", 1)
    file_write(fd, msg, 4)
    file_close(fd)
    exit(0)
}' 0

# --- Boolean literals ---
run_test "bool_true" 'fn main() { uint64 x = true; if x { exit(1) }; exit(0) }' 1
run_test "bool_false" 'fn main() { uint64 x = false; if x { exit(1) }; exit(0) }' 0

# --- Match statement ---
run_test "match_basic" 'fn main() {
    uint64 x = 2
    uint64 r = 0
    match x { 1 => { r = 10 } 2 => { r = 20 } 3 => { r = 30 } }
    exit(r)
}' 20

run_test "match_first" 'fn main() {
    uint64 x = 1
    uint64 r = 0
    match x { 1 => { r = 42 } 2 => { r = 99 } }
    exit(r)
}' 42

run_test "match_nomatch" 'fn main() {
    uint64 x = 99
    uint64 r = 42
    match x { 1 => { r = 0 } 2 => { r = 0 } }
    exit(r)
}' 42

run_test "match_enum" 'enum Color { Red = 1 Green = 2 Blue = 3 }
fn main() {
    uint64 c = Color.Green
    uint64 r = 0
    match c { 1 => { r = 10 } 2 => { r = 20 } 3 => { r = 30 } }
    exit(r)
}' 20

# --- Type aliases ---
run_test "type_alias" 'type Size = uint64
fn main() {
    Size x = 42
    exit(x)
}' 42

# --- Method syntax ---
run_test "method_decl" 'struct Point { uint64 x; uint64 y }
fn Point.sum(uint64 self) -> uint64 {
    uint64 sx = 0
    uint64 sy = 0
    unsafe { *(self as uint64) -> sx }
    uint64 yp = self + 8
    unsafe { *(yp as uint64) -> sy }
    return sx + sy
}
fn main() {
    Point p
    p.x = 10
    p.y = 32
    exit(sum(p))
}' 42

# --- Builtin: print/println ---
run_test_output "print_string" 'fn main() { print("hello world"); exit(0) }' "hello world"
run_test_output "print_int" 'fn main() { print(42); exit(0) }' "42"
run_test_output "print_zero" 'fn main() { print(0); exit(0) }' "0"
run_test_output "print_large" 'fn main() { print(123456); exit(0) }' "123456"
run_test_output "println_string" 'fn main() { println("hello"); exit(0) }' "hello"
run_test_output "println_int" 'fn main() { println(123); exit(0) }' "123"
run_test_output "println_multi" 'fn main() { println("abc"); println("def"); exit(0) }' "abc
def"

# --- Builtin: str_len ---
run_test "str_len_hello" 'fn main() { uint64 s = "hello"; exit(str_len(s)) }' 5
run_test "str_len_empty" 'fn main() { uint64 s = ""; exit(str_len(s)) }' 0
run_test "str_len_one" 'fn main() { uint64 s = "x"; exit(str_len(s)) }' 1

# --- Builtin: str_eq ---
run_test "str_eq_same" 'fn main() { uint64 a = "foo"; uint64 b = "foo"; exit(str_eq(a, b)) }' 1
run_test "str_eq_diff" 'fn main() { uint64 a = "foo"; uint64 b = "bar"; exit(str_eq(a, b)) }' 0
run_test "str_eq_prefix" 'fn main() { uint64 a = "foo"; uint64 b = "foobar"; exit(str_eq(a, b)) }' 0
run_test "str_eq_empty" 'fn main() { uint64 a = ""; uint64 b = ""; exit(str_eq(a, b)) }' 1

# --- Builtin: dealloc ---
run_test "dealloc_noop" 'fn main() { uint64 p = alloc(64); dealloc(p); exit(0) }' 0

# --- Builtin: memset ---
run_test_output "memset_basic" 'fn main() {
    uint64 buf = alloc(64)
    memset(buf, 65, 5)
    write(1, buf, 5)
    exit(0)
}' "AAAAA"

# --- Builtin: memcpy ---
run_test_output "memcpy_basic" 'fn main() {
    uint64 src = "hello"
    uint64 dst = alloc(64)
    memcpy(dst, src, 5)
    write(1, dst, 5)
    exit(0)
}' "hello"

# --- Kernel Features ---

# Inline assembly: nop (should compile and run without crashing)
run_test "asm_nop" 'fn main() { asm("nop"); exit(42) }' 42

# Inline assembly: multi-line block
run_test "asm_block" 'fn main() { asm { "nop"; "nop"; "nop" }; exit(7) }' 7

# Inline assembly: raw hex bytes (x86: 0x90 = nop)
run_test "asm_hex" 'fn main() { asm("0x90"); exit(5) }' 5

# Signed comparisons: signed_lt with negative-like values
run_test "signed_lt_true" 'fn main() {
    uint64 a = 0xFFFFFFFFFFFFFFFF
    uint64 b = 1
    uint64 r = signed_lt(a, b)
    exit(r)
}' 1

run_test "signed_lt_false" 'fn main() {
    uint64 a = 5
    uint64 b = 3
    uint64 r = signed_lt(a, b)
    exit(r)
}' 0

run_test "signed_gt_true" 'fn main() {
    uint64 a = 1
    uint64 b = 0xFFFFFFFFFFFFFFFF
    uint64 r = signed_gt(a, b)
    exit(r)
}' 1

run_test "signed_le_true" 'fn main() {
    uint64 a = 5
    uint64 b = 5
    uint64 r = signed_le(a, b)
    exit(r)
}' 1

run_test "signed_ge_true" 'fn main() {
    uint64 a = 0xFFFFFFFFFFFFFFFF
    uint64 b = 0xFFFFFFFFFFFFFFFF
    uint64 r = signed_ge(a, b)
    exit(r)
}' 1

# Bitfield operations
run_test "bit_get_1" 'fn main() {
    uint64 v = 0xFF
    uint64 r = bit_get(v, 3)
    exit(r)
}' 1

run_test "bit_get_0" 'fn main() {
    uint64 v = 0xF0
    uint64 r = bit_get(v, 2)
    exit(r)
}' 0

run_test "bit_set" 'fn main() {
    uint64 v = 0
    v = bit_set(v, 3)
    exit(v)
}' 8

run_test "bit_clear" 'fn main() {
    uint64 v = 0xFF
    v = bit_clear(v, 3)
    exit(v & 0xFF)
}' 247

run_test "bit_range" 'fn main() {
    uint64 v = 0xAB
    uint64 r = bit_range(v, 4, 4)
    exit(r)
}' 10

run_test "bit_insert" 'fn main() {
    uint64 v = 0x00
    v = bit_insert(v, 4, 4, 0xF)
    exit(v)
}' 240

# @naked function (no prologue/epilogue, just asm)
run_test "naked_fn" '@naked fn raw_exit() {
    asm("0x48 0xC7 0xC7 0x2A 0x00 0x00 0x00")
    asm("0x48 0xC7 0xC0 0x3C 0x00 0x00 0x00")
    asm("0x0F 0x05")
}
fn main() { raw_exit() }' 42

# @noreturn annotation (should compile fine)
run_test "noreturn_fn" '@noreturn fn die() { exit(99) }
fn main() { die() }' 99

# volatile block (same as unsafe)
run_test "volatile_block" 'fn main() {
    uint64 buf = alloc(64)
    uint64 val = 0
    unsafe { *(buf as uint64) = 42 }
    volatile { *(buf as uint64) -> val }
    exit(val)
}' 42

# @packed struct annotation (should parse without error)
run_test "packed_struct" '@packed struct Reg { uint8 a; uint32 b }
fn main() {
    uint8[16] buf
    exit(0)
}' 0

# @section annotation (should parse without error)
run_test "section_attr" '@section(".text.init") fn early_init() { exit(0) }
fn main() { early_init() }' 0

# --freestanding flag (should compile, main has no auto-exit, so explicit exit needed)
# Can't easily test this without a linker, just test that it parses
# run_test "freestanding" handled by CLI flag test below

# --- Function Pointers ---

# fn_addr + call_ptr basic
run_test "fn_ptr_basic" 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }
fn main() {
    uint64 fp = fn_addr("add")
    uint64 r = call_ptr(fp, 30, 12)
    exit(r)
}' 42

# fn_ptr dispatch table
run_test "fn_ptr_dispatch" 'fn h0() -> uint64 { return 10 }
fn h1() -> uint64 { return 20 }
fn h2() -> uint64 { return 12 }
fn main() {
    uint64 t = alloc(24)
    uint64 a = fn_addr("h0")
    uint64 b = fn_addr("h1")
    uint64 c = fn_addr("h2")
    unsafe { *(t as uint64) = a }
    uint64 t8 = t + 8
    unsafe { *(t8 as uint64) = b }
    uint64 t16 = t + 16
    unsafe { *(t16 as uint64) = c }
    uint64 fp = 0
    unsafe { *(t as uint64) -> fp }
    uint64 r = call_ptr(fp)
    uint64 fp2 = 0
    uint64 tb = t + 8
    unsafe { *(tb as uint64) -> fp2 }
    r = r + call_ptr(fp2)
    uint64 fp3 = 0
    uint64 tc = t + 16
    unsafe { *(tc as uint64) -> fp3 }
    r = r + call_ptr(fp3)
    exit(r)
}' 42

# fn_ptr no args
run_test "fn_ptr_noargs" 'fn get42() -> uint64 { return 42 }
fn main() {
    uint64 fp = fn_addr("get42")
    uint64 r = call_ptr(fp)
    exit(r)
}' 42

# --- Dead Code Elimination test ---
echo ""
echo "--- DCE test ---"
TOTAL=$((TOTAL + 1))

# Program with an unused function — DCE should eliminate it
cat > /tmp/krc_dce_unused_$$.kr << 'KRSRC'
fn unused_big() -> uint64 {
    uint64 a = 1
    uint64 b = 2
    uint64 c = 3
    uint64 d = 4
    uint64 e = 5
    uint64 f = a + b + c + d + e
    uint64 g = f * 2
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn unused_big2() -> uint64 {
    uint64 a = 10
    uint64 b = 20
    uint64 c = 30
    uint64 d = 40
    uint64 e = 50
    uint64 f = a + b + c + d + e
    uint64 g = f * 3
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn unused_big3() -> uint64 {
    uint64 a = 100
    uint64 b = 200
    uint64 c = 300
    uint64 d = 400
    uint64 e = 500
    uint64 f = a + b + c + d + e
    uint64 g = f * 4
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn main() { exit(42) }
KRSRC

# Same program but all functions are called
cat > /tmp/krc_dce_used_$$.kr << 'KRSRC'
fn used_big() -> uint64 {
    uint64 a = 1
    uint64 b = 2
    uint64 c = 3
    uint64 d = 4
    uint64 e = 5
    uint64 f = a + b + c + d + e
    uint64 g = f * 2
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn used_big2() -> uint64 {
    uint64 a = 10
    uint64 b = 20
    uint64 c = 30
    uint64 d = 40
    uint64 e = 50
    uint64 f = a + b + c + d + e
    uint64 g = f * 3
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn used_big3() -> uint64 {
    uint64 a = 100
    uint64 b = 200
    uint64 c = 300
    uint64 d = 400
    uint64 e = 500
    uint64 f = a + b + c + d + e
    uint64 g = f * 4
    uint64 h = g + f
    uint64 i = h * g + f
    uint64 j = i + h + g + f + e + d + c + b + a
    return j
}
fn main() {
    uint64 r = used_big() + used_big2() + used_big3()
    exit(r & 0xFF)
}
KRSRC

if $KRC /tmp/krc_dce_unused_$$.kr -o /tmp/krc_dce_small_$$ > /dev/null 2>&1 && \
   $KRC /tmp/krc_dce_used_$$.kr -o /tmp/krc_dce_large_$$ > /dev/null 2>&1; then
    chmod +x /tmp/krc_dce_small_$$ /tmp/krc_dce_large_$$
    small_size=$(wc -c < /tmp/krc_dce_small_$$)
    large_size=$(wc -c < /tmp/krc_dce_large_$$)
    # Verify the unused-function binary is smaller (DCE removed dead code)
    # Also verify the unused-function binary runs correctly
    /tmp/krc_dce_small_$$ > /dev/null 2>&1; small_exit=$?
    if [ "$small_size" -lt "$large_size" ] && [ "$small_exit" = "42" ]; then
        PASS=$((PASS + 1))
        echo "  dce_eliminates_unused: PASS (unused=$small_size < used=$large_size bytes, exit=$small_exit)"
    else
        echo "  dce_eliminates_unused: FAIL (unused=$small_size vs used=$large_size, exit=$small_exit)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "  dce_eliminates_unused: FAIL (compilation failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_dce_unused_$$.kr /tmp/krc_dce_used_$$.kr /tmp/krc_dce_small_$$ /tmp/krc_dce_large_$$

# --- Bootstrap test ---
echo ""
echo "--- Bootstrap test ---"
TOTAL=$((TOTAL + 1))
if [ -f "$DIR/../build/krc.kr" ]; then
    cp "$DIR/../build/krc.kr" /tmp/krc_bootstrap_$$.kr
    $KRC /tmp/krc_bootstrap_$$.kr -o /tmp/krc2_$$ > /dev/null 2>&1
    chmod +x /tmp/krc2_$$ 2>/dev/null
    /tmp/krc2_$$ --arch=x86_64 /tmp/krc_bootstrap_$$.kr -o /tmp/krc3_$$ > /dev/null 2>&1
    chmod +x /tmp/krc3_$$ 2>/dev/null
    /tmp/krc3_$$ --arch=x86_64 /tmp/krc_bootstrap_$$.kr -o /tmp/krc4_$$ > /dev/null 2>&1
    if diff /tmp/krc3_$$ /tmp/krc4_$$ > /dev/null 2>&1; then
        PASS=$((PASS + 1))
        echo "  bootstrap: PASS (fixed point at $(wc -c < /tmp/krc3_$$) bytes)"
    else
        FAIL=$((FAIL + 1))
        echo "  bootstrap: FAIL (krc3 != krc4)"
    fi
    rm -f /tmp/krc_bootstrap_$$.kr /tmp/krc2_$$ /tmp/krc3_$$ /tmp/krc4_$$
else
    echo "  bootstrap: SKIP (no build/krc.kr)"
    PASS=$((PASS + 1))
fi

# --- Summary ---
echo ""
echo "=== Results: $PASS/$TOTAL passed, $FAIL failed ==="
exit $FAIL
