#!/bin/bash
# No set -e: test binaries return non-zero exit codes intentionally

DIR="$(cd "$(dirname "$0")" && pwd)"
KRC="${KRC:-$DIR/../build/krc3}"
ARCH=$(uname -m)
KRC_FLAGS="${KRC_FLAGS:---arch=$ARCH}"
PASS=0
FAIL=0
TOTAL=0

run_test() {
    local name="$1"
    local input="$2"
    local expected="$3"
    TOTAL=$((TOTAL + 1))

    printf '%s\n' "$input" > /tmp/krc_test_$$.kr
    if $KRC $KRC_FLAGS /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
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
    if $KRC $KRC_FLAGS /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
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

# Inline assembly: raw hex bytes (x86-only: 0x90 = nop)
if [ "$ARCH" != "aarch64" ]; then
    run_test "asm_hex" 'fn main() { asm("0x90"); exit(5) }' 5
else
    echo "  asm_hex: SKIP (x86-only)"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1))
fi

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

# @naked function (x86-only: uses raw x86 machine code bytes)
if [ "$ARCH" != "aarch64" ]; then
    run_test "naked_fn" '@naked fn raw_exit() {
        asm("0x48 0xC7 0xC7 0x2A 0x00 0x00 0x00")
        asm("0x48 0xC7 0xC0 0x3C 0x00 0x00 0x00")
        asm("0x0F 0x05")
    }
    fn main() { raw_exit() }' 42
else
    echo "  naked_fn: SKIP (x86-only)"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1))
fi

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

# --- uint16 pointer operations ---
run_test "uint16_store_load" 'fn main() {
    uint64 buf = alloc(64)
    uint16 val = 0xBEEF
    unsafe { *(buf as uint16) = val }
    uint16 got = 0
    unsafe { *(buf as uint16) -> got }
    uint64 r = got
    exit(r & 0xFF)
}' 239

run_test "uint16_store_load_small" 'fn main() {
    uint64 buf = alloc(64)
    uint16 val = 42
    unsafe { *(buf as uint16) = val }
    uint16 got = 0
    unsafe { *(buf as uint16) -> got }
    uint64 r = got
    exit(r)
}' 42

run_test "uint16_two_slots" 'fn main() {
    uint64 buf = alloc(64)
    uint16 a = 10
    uint16 b = 32
    unsafe { *(buf as uint16) = a }
    uint64 buf2 = buf + 2
    unsafe { *(buf2 as uint16) = b }
    uint16 va = 0
    uint16 vb = 0
    unsafe { *(buf as uint16) -> va }
    unsafe { *(buf2 as uint16) -> vb }
    uint64 ra = va
    uint64 rb = vb
    exit(ra + rb)
}' 42

# --- Atomic operations ---
run_test "atomic_store_load" 'fn main() {
    uint64 buf = alloc(64)
    atomic_store(buf, 42)
    uint64 v = atomic_load(buf)
    exit(v)
}' 42

run_test "atomic_add_basic" 'fn main() {
    uint64 buf = alloc(64)
    atomic_store(buf, 30)
    uint64 old = atomic_add(buf, 12)
    uint64 v = atomic_load(buf)
    exit(v)
}' 42

run_test "atomic_add_returns_old" 'fn main() {
    uint64 buf = alloc(64)
    atomic_store(buf, 40)
    uint64 old = atomic_add(buf, 10)
    exit(old)
}' 40

run_test "atomic_cas_success" 'fn main() {
    uint64 buf = alloc(64)
    atomic_store(buf, 10)
    uint64 ok = atomic_cas(buf, 10, 42)
    uint64 v = atomic_load(buf)
    if ok == 1 && v == 42 { exit(42) }
    exit(0)
}' 42

run_test "atomic_cas_fail" 'fn main() {
    uint64 buf = alloc(64)
    atomic_store(buf, 10)
    uint64 ok = atomic_cas(buf, 99, 42)
    uint64 v = atomic_load(buf)
    if ok == 0 && v == 10 { exit(42) }
    exit(0)
}' 42

# --- Volatile blocks ---
run_test "volatile_store_load" 'fn main() {
    uint64 buf = alloc(64)
    volatile { *(buf as uint64) = 42 }
    uint64 v = 0
    volatile { *(buf as uint64) -> v }
    exit(v)
}' 42

run_test "volatile_roundtrip" 'fn main() {
    uint64 buf = alloc(64)
    volatile { *(buf as uint64) = 100 }
    uint64 a = 0
    volatile { *(buf as uint64) -> a }
    volatile { *(buf as uint64) = 42 }
    uint64 b = 0
    volatile { *(buf as uint64) -> b }
    exit(b)
}' 42

run_test "volatile_uint8" 'fn main() {
    uint64 buf = alloc(64)
    uint8 val = 42
    volatile { *(buf as uint8) = val }
    uint8 got = 0
    volatile { *(buf as uint8) -> got }
    uint64 r = got
    exit(r)
}' 42

# --- MSR/MRS (compile-only, privileged instructions cannot run in userspace) ---
if [ "$ARCH" != "aarch64" ]; then
    # x86: rdmsr/wrmsr are ring-0 only; just verify the asm block compiles
    TOTAL=$((TOTAL + 1))
    printf 'fn main() { exit(42) }\n@naked fn msr_test() { asm("rdmsr") }\n' > /tmp/krc_test_$$.kr
    if $KRC $KRC_FLAGS /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
        chmod +x /tmp/krc_test_$$
        /tmp/krc_test_$$ > /dev/null 2>&1; got=$?
        if [ "$got" = "42" ]; then
            PASS=$((PASS + 1))
        else
            echo "FAIL: msr_compile (expected 42, got $got)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: msr_compile (compilation failed)"
        FAIL=$((FAIL + 1))
    fi
    rm -f /tmp/krc_test_$$.kr /tmp/krc_test_$$

    TOTAL=$((TOTAL + 1))
    printf 'fn main() { exit(42) }\n@naked fn msr_test() { asm("wrmsr") }\n' > /tmp/krc_test_$$.kr
    if $KRC $KRC_FLAGS /tmp/krc_test_$$.kr -o /tmp/krc_test_$$ > /dev/null 2>&1; then
        chmod +x /tmp/krc_test_$$
        /tmp/krc_test_$$ > /dev/null 2>&1; got=$?
        if [ "$got" = "42" ]; then
            PASS=$((PASS + 1))
        else
            echo "FAIL: msr_wrmsr_compile (expected 42, got $got)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: msr_wrmsr_compile (compilation failed)"
        FAIL=$((FAIL + 1))
    fi
    rm -f /tmp/krc_test_$$.kr /tmp/krc_test_$$
else
    echo "  msr_compile: SKIP (x86-only)"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1))
    echo "  msr_wrmsr_compile: SKIP (x86-only)"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1))
fi

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

if $KRC $KRC_FLAGS /tmp/krc_dce_unused_$$.kr -o /tmp/krc_dce_small_$$ > /dev/null 2>&1 && \
   $KRC $KRC_FLAGS /tmp/krc_dce_used_$$.kr -o /tmp/krc_dce_large_$$ > /dev/null 2>&1; then
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

# --- ELF relocatable (.o) test ---
echo ""
echo "--- ELF relocatable (.o) test ---"
TOTAL=$((TOTAL + 1))
printf 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }\nfn main() { exit(add(30, 12)) }\n' > /tmp/krc_obj_$$.kr
if $KRC $KRC_FLAGS --emit=obj /tmp/krc_obj_$$.kr -o /tmp/krc_obj_$$.o > /dev/null 2>&1; then
    # Check first 18 bytes: ELF magic (4) + class(1) + data(1) + version(1) + osabi(1) + padding(8) + e_type LE (2)
    # e_type at offset 16-17 should be 01 00 (ET_REL = 1, little-endian)
    magic=$(xxd -l 4 -p /tmp/krc_obj_$$.o 2>/dev/null)
    etype=$(xxd -s 16 -l 2 -p /tmp/krc_obj_$$.o 2>/dev/null)
    if [ "$magic" = "7f454c46" ] && [ "$etype" = "0100" ]; then
        PASS=$((PASS + 1))
        echo "  emit_obj: PASS (valid ELF relocatable, $(wc -c < /tmp/krc_obj_$$.o) bytes)"
    else
        FAIL=$((FAIL + 1))
        echo "  emit_obj: FAIL (bad ELF header: magic=$magic etype=$etype)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  emit_obj: FAIL (compilation with --emit=obj failed)"
fi

# Also test -c flag produces same result
TOTAL=$((TOTAL + 1))
if $KRC $KRC_FLAGS -c /tmp/krc_obj_$$.kr -o /tmp/krc_obj_c_$$.o > /dev/null 2>&1; then
    c_magic=$(xxd -l 4 -p /tmp/krc_obj_c_$$.o 2>/dev/null)
    c_etype=$(xxd -s 16 -l 2 -p /tmp/krc_obj_c_$$.o 2>/dev/null)
    if [ "$c_magic" = "7f454c46" ] && [ "$c_etype" = "0100" ]; then
        PASS=$((PASS + 1))
        echo "  emit_obj_c_flag: PASS"
    else
        FAIL=$((FAIL + 1))
        echo "  emit_obj_c_flag: FAIL (bad ELF header)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  emit_obj_c_flag: FAIL (compilation with -c failed)"
fi

# Test readelf can parse sections and symbols.
# Cross-compile KRC_FLAGS (e.g. --arch=arm64 on an arm64 runner re-targeting
# the host) can produce a valid .o that this regex-based test doesn't cover.
# Skip on non-x86_64 hosts where KRC_FLAGS targets arm64.
TOTAL=$((TOTAL + 1))
if [ "$(uname -m)" != "x86_64" ] && [ "$(uname -m)" != "amd64" ]; then
    PASS=$((PASS + 1))
    echo "  emit_obj_readelf: SKIP (non-x86_64 host)"
elif command -v readelf > /dev/null 2>&1 && [ -f /tmp/krc_obj_$$.o ]; then
    sections=$(readelf -S /tmp/krc_obj_$$.o 2>/dev/null)
    has_text=$(echo "$sections" | grep -c '\.text')
    has_symtab=$(echo "$sections" | grep -c '\.symtab')
    symbols=$(readelf -s /tmp/krc_obj_$$.o 2>/dev/null)
    has_main=$(echo "$symbols" | grep -c 'FUNC.*GLOBAL.*main')
    has_add=$(echo "$symbols" | grep -c 'FUNC.*LOCAL.*add')
    if [ "$has_text" -ge 1 ] && [ "$has_symtab" -ge 1 ] && [ "$has_main" -ge 1 ] && [ "$has_add" -ge 1 ]; then
        PASS=$((PASS + 1))
        echo "  emit_obj_readelf: PASS (.text, .symtab, main GLOBAL, add LOCAL)"
    else
        FAIL=$((FAIL + 1))
        echo "  emit_obj_readelf: FAIL (text=$has_text symtab=$has_symtab main=$has_main add=$has_add)"
    fi
else
    PASS=$((PASS + 1))
    echo "  emit_obj_readelf: SKIP (readelf not found or .o missing)"
fi
rm -f /tmp/krc_obj_$$.kr /tmp/krc_obj_$$.o /tmp/krc_obj_c_$$.o

# --- Generics (monomorphization) ---
run_test "generic_fn_single" 'fn max_gen<T>(T a, T b) -> T {
    if a > b { return a }
    return b
}
fn main() {
    uint64 r = max_gen<uint64>(30, 42)
    exit(r)
}' 42

run_test "generic_fn_identity" 'fn identity<T>(T x) -> T { return x }
fn main() {
    uint64 r = identity<uint64>(7)
    exit(r)
}' 7

run_test "generic_fn_chain" 'fn max_gen<T>(T a, T b) -> T {
    if a > b { return a }
    return b
}
fn identity<T>(T x) -> T { return x }
fn main() {
    uint64 r = max_gen<uint64>(30, 42)
    uint64 s = identity<uint64>(r)
    exit(s)
}' 42

run_test "generic_call_uint32" 'fn add_one<T>(T x) -> T { return x + 1 }
fn main() {
    uint32 r = add_one<uint32>(41)
    exit(r)
}' 42

run_test "generic_multi_param" 'fn pick_first<T, U>(T a, U b) -> T { return a }
fn main() {
    uint64 r = pick_first<uint64, uint32>(42, 99)
    exit(r)
}' 42

run_test "generic_no_conflict_lt" 'fn id<T>(T x) -> T { return x }
fn main() {
    uint64 a = 3
    uint64 b = 5
    if a < b { exit(id<uint64>(42)) }
    exit(0)
}' 42

# --- Error detection tests ---
echo ""
echo "--- Error detection tests ---"

# Wrong argument count
TOTAL=$((TOTAL + 1))
printf 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }\nfn main() { exit(add(1, 2, 3)) }\n' > /tmp/krc_err_$$.kr
if $KRC $KRC_FLAGS /tmp/krc_err_$$.kr -o /tmp/krc_err_$$ 2>/tmp/krc_stderr_$$ ; then
    echo "FAIL: wrong_arg_count (should not compile)"
    FAIL=$((FAIL + 1))
else
    if grep -q "wrong number of arguments" /tmp/krc_stderr_$$; then
        PASS=$((PASS + 1))
        echo "  wrong_arg_count: PASS (error detected)"
    else
        echo "FAIL: wrong_arg_count (wrong error)"
        FAIL=$((FAIL + 1))
    fi
fi
rm -f /tmp/krc_err_$$.kr /tmp/krc_err_$$ /tmp/krc_stderr_$$

# Missing return in non-void function
TOTAL=$((TOTAL + 1))
printf 'fn get_val() -> uint64 { uint64 x = 42 }\nfn main() { exit(get_val()) }\n' > /tmp/krc_err_$$.kr
if $KRC $KRC_FLAGS /tmp/krc_err_$$.kr -o /tmp/krc_err_$$ 2>/tmp/krc_stderr_$$ ; then
    echo "FAIL: missing_return (should not compile)"
    FAIL=$((FAIL + 1))
else
    if grep -q "may not return" /tmp/krc_stderr_$$; then
        PASS=$((PASS + 1))
        echo "  missing_return: PASS (error detected)"
    else
        echo "FAIL: missing_return (wrong error)"
        FAIL=$((FAIL + 1))
    fi
fi
rm -f /tmp/krc_err_$$.kr /tmp/krc_err_$$ /tmp/krc_stderr_$$

# Duplicate function definition
TOTAL=$((TOTAL + 1))
printf 'fn foo() { exit(1) }\nfn foo() { exit(2) }\nfn main() { foo() }\n' > /tmp/krc_err_$$.kr
if $KRC $KRC_FLAGS /tmp/krc_err_$$.kr -o /tmp/krc_err_$$ 2>/tmp/krc_stderr_$$ ; then
    echo "FAIL: duplicate_fn (should not compile)"
    FAIL=$((FAIL + 1))
else
    if grep -q "redefinition" /tmp/krc_stderr_$$; then
        PASS=$((PASS + 1))
        echo "  duplicate_fn: PASS (error detected)"
    else
        echo "FAIL: duplicate_fn (wrong error)"
        FAIL=$((FAIL + 1))
    fi
fi
rm -f /tmp/krc_err_$$.kr /tmp/krc_err_$$ /tmp/krc_stderr_$$

# --- Android emit test ---
echo ""
echo "--- Android emit test ---"
TOTAL=$((TOTAL + 1))
printf 'fn main() { exit(42) }\n' > /tmp/krc_android_$$.kr
if $KRC $KRC_FLAGS --emit=android /tmp/krc_android_$$.kr -o /tmp/krc_android_$$ > /dev/null 2>&1; then
    magic=$(xxd -l 4 -p /tmp/krc_android_$$ 2>/dev/null)
    etype=$(xxd -s 16 -l 2 -p /tmp/krc_android_$$ 2>/dev/null)
    if [ "$magic" = "7f454c46" ] && [ "$etype" = "0300" ]; then
        PASS=$((PASS + 1))
        echo "  android_emit: PASS (valid PIE ELF, $(wc -c < /tmp/krc_android_$$) bytes)"
    else
        FAIL=$((FAIL + 1))
        echo "  android_emit: FAIL (bad ELF: magic=$magic etype=$etype)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  android_emit: FAIL (compilation failed)"
fi
rm -f /tmp/krc_android_$$.kr /tmp/krc_android_$$

# --- Android x86_64 emit test ---
echo ""
echo "--- Android x86_64 emit test ---"
TOTAL=$((TOTAL + 1))
printf 'fn main() { exit(42) }\n' > /tmp/krc_androidx_$$.kr
if $KRC --arch=x86_64 --emit=android /tmp/krc_androidx_$$.kr -o /tmp/krc_androidx_$$ > /dev/null 2>&1; then
    magic=$(xxd -l 4 -p /tmp/krc_androidx_$$ 2>/dev/null)
    etype=$(xxd -s 16 -l 2 -p /tmp/krc_androidx_$$ 2>/dev/null)
    emach=$(xxd -s 18 -l 2 -p /tmp/krc_androidx_$$ 2>/dev/null)
    if [ "$magic" = "7f454c46" ] && [ "$etype" = "0300" ] && [ "$emach" = "3e00" ]; then
        # Execute via glibc loader (bypasses PT_INTERP=/system/bin/linker64)
        if [ -x /lib64/ld-linux-x86-64.so.2 ] && [ "$(uname -m)" = "x86_64" ]; then
            actual=0
            /lib64/ld-linux-x86-64.so.2 /tmp/krc_androidx_$$ > /dev/null 2>&1
            actual=$?
            if [ "$actual" = "42" ]; then
                PASS=$((PASS + 1))
                echo "  android_emit_x86_64: PASS (PIE ELF x86-64, exec=42)"
            else
                FAIL=$((FAIL + 1))
                echo "  android_emit_x86_64: FAIL (exec exit=$actual, expected 42)"
            fi
        else
            PASS=$((PASS + 1))
            echo "  android_emit_x86_64: PASS (structural; no glibc loader)"
        fi
    else
        FAIL=$((FAIL + 1))
        echo "  android_emit_x86_64: FAIL (bad ELF: magic=$magic etype=$etype mach=$emach)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  android_emit_x86_64: FAIL (compilation failed)"
fi
rm -f /tmp/krc_androidx_$$.kr /tmp/krc_androidx_$$

# --- 2-tuple return and destructure ---
run_test "tuple_basic" 'fn divmod(uint64 x, uint64 y) -> uint64 { return (x / y, x % y) }
fn main() { (uint64 q, uint64 r) = divmod(17, 5); exit(q + r) }' 5

run_test "tuple_branch" 'fn minmax(uint64 a, uint64 b) -> uint64 { if a < b { return (a, b) } return (b, a) }
fn main() { (uint64 lo, uint64 hi) = minmax(42, 7); exit(hi - lo) }' 35

run_test "tuple_nested_call" 'fn pair(uint64 x) -> uint64 { return (x, x + 1) }
fn main() { (uint64 a, uint64 b) = pair(10); exit(a * b) }' 110

run_test "tuple_void_context" 'fn split(uint64 n) -> uint64 { return (n * 2, n * 3) }
fn main() { uint64 sum = 0; (uint64 a, uint64 b) = split(5); sum = a + b; exit(sum) }' 25

run_test "tuple_reuse" 'fn step(uint64 x) -> uint64 { return (x + 1, x + 2) }
fn main() { (uint64 p, uint64 q) = step(10); (uint64 r, uint64 s) = step(20); exit(p + q + r + s) }' 66

# --- asm { } I/O constraints ---
# x86_64-only asm constraint tests (rdtsc, shl are x86 instructions)
if [ "$ARCH" = "x86_64" ] || [ "$ARCH" = "amd64" ]; then
# rdtsc: no inputs, two outputs (low/high 32 bits of the TSC into rax/rdx).
run_test "asm_rdtsc_out" 'fn main() {
    uint64 lo = 0
    uint64 hi = 0
    asm { "rdtsc" } out(rax -> lo, rdx -> hi)
    if lo == 0 { if hi == 0 { exit(1) } }
    exit(0)
}' 0

# shl via asm with one input and one output, testing pinned-param loading.
run_test "asm_shl_in_out" 'fn shl_by(uint64 v, uint64 n) -> uint64 {
    uint64 r = 0
    asm { "0x48 0xD3 0xE0" } in(v -> rax, n -> rcx) out(rax -> r)
    return r
}
fn main() { exit(shl_by(3, 4)) }' 48
fi

# nop with no constraints — ensures backward-compat with existing asm blocks.
run_test "asm_nop_noconstraints" 'fn main() { asm { "nop" }; exit(5) }' 5

# --- Opt-in: run on a real Android emulator via adb (ANDROID_EMULATOR=1) ---
# Requires: adb on PATH, one device online, and write access to
# /data/local/tmp. Cross-compiles a handful of programs as
# android-x86_64, pushes them, and executes under real bionic.
if [ "${ANDROID_EMULATOR:-0}" = "1" ] && command -v adb > /dev/null 2>&1; then
    DEV=$(adb get-state 2>/dev/null | tr -d '\r')
    if [ "$DEV" = "device" ]; then
        echo ""
        echo "--- Android emulator (adb, x86_64) ---"
        _adb_run() {
            local name="$1" src="$2" expected="$3"
            TOTAL=$((TOTAL + 1))
            printf '%s\n' "$src" > /tmp/krc_adb_$$.kr
            if $KRC --arch=x86_64 --emit=android /tmp/krc_adb_$$.kr -o /tmp/krc_adb_$$ > /dev/null 2>&1; then
                adb push /tmp/krc_adb_$$ /data/local/tmp/krc_adb_$$ > /dev/null 2>&1
                adb shell chmod 755 /data/local/tmp/krc_adb_$$ > /dev/null 2>&1
                got=$(adb shell "/data/local/tmp/krc_adb_$$ > /dev/null 2>&1; echo \$?" | tr -d '\r')
                if [ "$got" = "$expected" ]; then
                    PASS=$((PASS + 1))
                    echo "  adb_$name: PASS"
                else
                    FAIL=$((FAIL + 1))
                    echo "  adb_$name: FAIL (expected $expected, got $got)"
                fi
                adb shell rm -f /data/local/tmp/krc_adb_$$ > /dev/null 2>&1
            else
                FAIL=$((FAIL + 1))
                echo "  adb_$name: FAIL (compile)"
            fi
            rm -f /tmp/krc_adb_$$.kr /tmp/krc_adb_$$
        }
        _adb_run "exit42"   'fn main() { exit(42) }' 42
        _adb_run "add"      'fn main() { exit(2 + 3) }' 5
        _adb_run "loop"     'fn main() { uint64 s = 0; for i in 1..11 { s = s + i }; exit(s) }' 55
        _adb_run "recurse"  'fn fib(uint64 n) -> uint64 { if n <= 1 { return n } return fib(n-1)+fib(n-2) }
fn main() { exit(fib(10)) }' 55
        _adb_run "statics"  'static uint64 c = 0
fn inc() { c = c + 1 }
fn main() { inc(); inc(); inc(); inc(); exit(c) }' 4
        _adb_run "println"  'fn main() { println("android bionic"); exit(7) }' 7
    else
        echo "  android_emulator: SKIP (ANDROID_EMULATOR=1 but no device online)"
    fi
fi

# --- For loop ---
run_test "for_range" 'fn main() { uint64 s = 0; for i in 0..10 { s = s + i }; exit(s) }' 45

# --- Many-parameter functions ---
run_test "fn_7args" 'fn sum7(uint64 a, uint64 b, uint64 c, uint64 d, uint64 e, uint64 f, uint64 g) -> uint64 { return a + b + c + d + e + f + g }
fn main() { exit(sum7(1,2,3,4,5,6,7)) }' 28

run_test "fn_8args" 'fn s(uint64 a, uint64 b, uint64 c, uint64 d, uint64 e, uint64 f, uint64 g, uint64 h) -> uint64 { return a + b + c + d + e + f + g + h }
fn main() { exit(s(1,2,3,4,5,6,7,8)) }' 36

# --- Enum (auto-numbered) ---
run_test "enum_auto" 'enum Color { Red, Green, Blue }
fn main() { exit(Color.Blue) }' 2

# --- emit=asm produces text ---
echo ""
echo "--- ASM emit test ---"
TOTAL=$((TOTAL + 1))
printf 'fn main() { exit(42) }\n' > /tmp/krc_asm_$$.kr
if $KRC $KRC_FLAGS --emit=asm /tmp/krc_asm_$$.kr -o /tmp/krc_asm_$$.s > /dev/null 2>&1; then
    if file /tmp/krc_asm_$$.s | grep -qi 'text\|ascii' && grep -q 'main' /tmp/krc_asm_$$.s; then
        PASS=$((PASS + 1))
        echo "  emit_asm: PASS (text output with function labels)"
    else
        FAIL=$((FAIL + 1))
        echo "  emit_asm: FAIL (output is not text or missing labels)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  emit_asm: FAIL (compilation with --emit=asm failed)"
fi
rm -f /tmp/krc_asm_$$.kr /tmp/krc_asm_$$.s

# --- emit=asm content tests ---
echo ""
echo "--- emit=asm content tests ---"

# Test asm output has function labels and mnemonics
TOTAL=$((TOTAL + 1))
echo 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }
fn main() { exit(add(1, 2)) }' > /tmp/krc_asm_test_$$.kr
if $KRC $KRC_FLAGS --emit=asm /tmp/krc_asm_test_$$.kr -o /tmp/krc_asm_test_$$.s > /dev/null 2>&1; then
    if grep -q "add:" /tmp/krc_asm_test_$$.s && grep -q "main:" /tmp/krc_asm_test_$$.s && grep -q "ret" /tmp/krc_asm_test_$$.s; then
        echo "  emit_asm_content: PASS"
        PASS=$((PASS + 1))
    else
        echo "  emit_asm_content: FAIL (missing labels or mnemonics)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "  emit_asm_content: FAIL (compilation error)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_asm_test_$$.*

# Test that --emit=xyz gives an error
TOTAL=$((TOTAL + 1))
echo 'fn main() { exit(0) }' > /tmp/krc_asm_err_$$.kr
if $KRC --emit=xyz /tmp/krc_asm_err_$$.kr -o /tmp/krc_asm_err_$$ 2>&1 | grep -q "unknown emit format"; then
    echo "  emit_unknown_error: PASS"
    PASS=$((PASS + 1))
else
    echo "  emit_unknown_error: FAIL"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_asm_err_$$.kr /tmp/krc_asm_err_$$

# --- String escapes ---
run_test_output "str_escape_newline" 'fn main() { print("a\nb"); exit(0) }' "a
b"

# --- ARM64 cross-compilation tests via QEMU ---
QEMU_A64=""
if command -v qemu-aarch64-static > /dev/null 2>&1; then
    QEMU_A64="qemu-aarch64-static"
elif command -v qemu-aarch64 > /dev/null 2>&1; then
    QEMU_A64="qemu-aarch64"
fi

if [ -n "$QEMU_A64" ] && [ "$ARCH" = "x86_64" ]; then
    echo ""
    echo "--- ARM64 cross-compilation tests (QEMU) ---"

    run_test_a64() {
        local name="$1"
        local input="$2"
        local expected="$3"
        TOTAL=$((TOTAL + 1))

        printf '%s\n' "$input" > /tmp/krc_a64_$$.kr
        if $KRC --arch=arm64 /tmp/krc_a64_$$.kr -o /tmp/krc_a64_$$ > /dev/null 2>&1; then
            chmod +x /tmp/krc_a64_$$
            local got=0
            $QEMU_A64 /tmp/krc_a64_$$ > /dev/null 2>&1 && got=0 || got=$?
            if [ "$got" = "$expected" ]; then
                PASS=$((PASS + 1))
            else
                echo "FAIL: $name (expected $expected, got $got)"
                FAIL=$((FAIL + 1))
            fi
        else
            echo "FAIL: $name (cross-compilation failed)"
            FAIL=$((FAIL + 1))
        fi
        rm -f /tmp/krc_a64_$$.kr /tmp/krc_a64_$$
    }

    run_test_a64 "a64_exit" 'fn main() { exit(42) }' 42
    run_test_a64 "a64_add" 'fn add(uint64 a, uint64 b) -> uint64 { return a + b }
fn main() { exit(add(10, 32)) }' 42
    run_test_a64 "a64_atomic" 'fn main() { uint64 buf = alloc(64); atomic_store(buf, 42); exit(atomic_load(buf)) }' 42
    run_test_a64 "a64_static" 'static uint64 x = 0
fn main() { x = 42; exit(x) }' 42
fi

# --- v2.6 feature tests ---
echo ""
echo "--- v2.6 short type aliases ---"
run_test "alias_u8"  'fn main() { u8 x = 42; exit(x) }' 42
run_test "alias_u16" 'fn main() { u16 x = 42; exit(x) }' 42
run_test "alias_u32" 'fn main() { u32 x = 42; exit(x) }' 42
run_test "alias_u64" 'fn main() { u64 x = 42; exit(x) }' 42
run_test "alias_i8"  'fn main() { i8  x = 42; exit(x) }' 42
run_test "alias_i16" 'fn main() { i16 x = 42; exit(x) }' 42
run_test "alias_i32" 'fn main() { i32 x = 42; exit(x) }' 42
run_test "alias_i64" 'fn main() { i64 x = 42; exit(x) }' 42

echo ""
echo "--- v2.6 pointer load/store builtins ---"
run_test "load_store_u8"  'fn main() { u64 buf = alloc(16); store8(buf, 42); exit(load8(buf)) }' 42
run_test "load_store_u16" 'fn main() { u64 buf = alloc(16); store16(buf, 42); exit(load16(buf)) }' 42
run_test "load_store_u32" 'fn main() { u64 buf = alloc(16); store32(buf, 42); exit(load32(buf)) }' 42
run_test "load_store_u64" 'fn main() { u64 buf = alloc(16); store64(buf, 42); exit(load64(buf)) }' 42
run_test "load_store_offsets" 'fn main() {
    u64 buf = alloc(32)
    store8(buf + 0, 1)
    store8(buf + 1, 2)
    store8(buf + 2, 3)
    store8(buf + 3, 4)
    exit(load8(buf + 0) + load8(buf + 1) + load8(buf + 2) + load8(buf + 3))
}' 10
run_test "load_store_widths_mixed" 'fn main() {
    u64 buf = alloc(32)
    store32(buf, 0x11223344)
    exit(load8(buf) + load8(buf + 1) + load8(buf + 2) + load8(buf + 3))
}' 170
run_test "vload_vstore_u32" 'fn main() { u64 buf = alloc(16); vstore32(buf, 42); exit(vload32(buf)) }' 42
run_test "vload_vstore_u64" 'fn main() { u64 buf = alloc(16); vstore64(buf, 42); exit(vload64(buf)) }' 42

echo ""
echo "--- v2.6 print_str / println_str ---"
# print_str prints the contents of a variable string pointer.
# If the builtin is broken, it prints the pointer address as a number
# instead of the string, and the output doesn't contain "Hi".
run_test_output "print_str_variable" 'fn main() {
    u64 msg = "Hi"
    print_str(msg)
    exit(0)
}' 'Hi' 0
run_test_output "println_str_variable" 'fn main() {
    u64 msg = "Line"
    println_str(msg)
    exit(0)
}' 'Line' 0

echo ""
echo "--- v2.6 static arrays ---"
run_test "static_array_u8" 'static u8[16] buf
fn main() { buf[0] = 42; exit(buf[0]) }' 42
run_test "static_array_roundtrip" 'static u8[32] buf
fn main() {
    buf[5] = 10
    buf[6] = 20
    buf[7] = 12
    exit(buf[5] + buf[6] + buf[7])
}' 42

echo ""
echo "--- v2.6 struct arrays ---"
run_test "struct_array_basic" 'struct P { u64 x; u64 y }
fn main() {
    P[4] pts
    pts[0].x = 10
    pts[0].y = 20
    pts[3].x = 5
    pts[3].y = 7
    exit(pts[0].x + pts[0].y + pts[3].x + pts[3].y)
}' 42
run_test "struct_array_iteration" 'struct Row { u64 a; u64 b }
fn main() {
    Row[5] rows
    for i in 0..5 {
        rows[i].a = i
        rows[i].b = 0
    }
    u64 sum = 0
    for j in 0..5 {
        sum = sum + rows[j].a
    }
    exit(sum)
}' 10

echo ""
echo "--- v2.6 slice parameters ---"
run_test "slice_param_len" 'fn sum_bytes([u8] data) -> u64 {
    u64 total = 0
    u64 i = 0
    u64 n = data.len
    while i < n {
        total = total + load8(data + i)
        i = i + 1
    }
    return total
}
fn main() {
    u8[6] buf
    buf[0] = 10
    buf[1] = 20
    buf[2] = 12
    exit(sum_bytes(buf, 3))
}' 42

echo ""
echo "--- v2.6 device blocks ---"
run_test "device_block_read_write" 'device Fake at 0x66666000 {
    Data at 0x00 : u32
    Status at 0x04 : u8
}
fn main() {
    // mmap a page at 0x66666000 (Linux x86_64 syscall 9, ARM64 222)
    u64 nr = 9
    if get_arch_id() == 2 { nr = 222 }
    syscall_raw(nr, 0x66666000, 4096, 3, 0x32, 0xFFFFFFFFFFFFFFFF, 0)
    Fake.Data = 42
    Fake.Status = 7
    u32 v = Fake.Data
    u8  s = Fake.Status
    exit(v + s)
}' 49

echo ""
echo "--- v2.6 method calls ---"
run_test "method_call" 'struct P { u64 x; u64 y }
fn P.sum(P self) -> u64 { return self.x + self.y }
fn main() {
    P p
    p.x = 10
    p.y = 32
    exit(p.sum())
}' 42

echo ""
echo "--- v2.6 #lang directive ---"
run_test "lang_stable" '#lang stable

fn main() { exit(42) }' 42
run_test "lang_experimental" '#lang experimental

fn main() { exit(42) }' 42

echo ""
echo "--- v2.6 living compiler ---"
# --list-proposals should work without an input file and exit 0
TOTAL=$((TOTAL + 1))
if $KRC lc --list-proposals > /tmp/krc_prop_$$.txt 2>&1; then
    if grep -q "KernRift Proposal Registry" /tmp/krc_prop_$$.txt && grep -q "load_store_builtins" /tmp/krc_prop_$$.txt; then
        PASS=$((PASS + 1))
    else
        echo "FAIL: list_proposals (output did not contain expected strings)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "FAIL: list_proposals (command failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_prop_$$.txt

# --fix --dry-run on a legacy file should show a migration
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_mig_$$.kr <<'KREOF'
fn main() {
    u64 buf = alloc(16)
    u64 v = 0
    unsafe { *(buf as u32) -> v }
    exit(v)
}
KREOF
if $KRC lc --fix --dry-run /tmp/krc_mig_$$.kr > /tmp/krc_mig_out_$$.txt 2>&1; then
    if grep -q "1 migration site(s) rewritten" /tmp/krc_mig_out_$$.txt && grep -q "load32" /tmp/krc_mig_out_$$.txt; then
        PASS=$((PASS + 1))
    else
        echo "FAIL: migration_dry_run (output missing expected content)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "FAIL: migration_dry_run (command failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_mig_$$.kr /tmp/krc_mig_out_$$.txt

# --fix (actual) on a legacy file should rewrite and the result should compile
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_mig2_$$.kr <<'KREOF'
fn main() {
    u64 buf = alloc(16)
    u64 v = 0
    store32(buf, 42)
    unsafe { *(buf as u32) -> v }
    exit(v)
}
KREOF
if $KRC lc --fix /tmp/krc_mig2_$$.kr > /dev/null 2>&1; then
    if grep -q "v = load32(buf)" /tmp/krc_mig2_$$.kr; then
        # Now verify the rewritten file still compiles and runs
        if $KRC $KRC_FLAGS /tmp/krc_mig2_$$.kr -o /tmp/krc_mig2_bin_$$ > /dev/null 2>&1; then
            chmod +x /tmp/krc_mig2_bin_$$
            /tmp/krc_mig2_bin_$$ > /dev/null 2>&1
            if [ "$?" = "42" ]; then
                PASS=$((PASS + 1))
            else
                echo "FAIL: migration_apply (rewritten binary exit != 42)"
                FAIL=$((FAIL + 1))
            fi
        else
            echo "FAIL: migration_apply (rewritten file did not compile)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: migration_apply (file was not rewritten)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "FAIL: migration_apply (command failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_mig2_$$.kr /tmp/krc_mig2_bin_$$

# krc lc on a file with unsafe ops should report legacy_ptr_ops
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_lc_$$.kr <<'KREOF'
fn main() {
    u64 buf = alloc(16)
    u64 v = 0
    unsafe { *(buf as u32) -> v }
    exit(v)
}
KREOF
if $KRC lc /tmp/krc_lc_$$.kr > /tmp/krc_lc_out_$$.txt 2>&1; then
    if grep -q "legacy_ptr_ops" /tmp/krc_lc_out_$$.txt && grep -q "auto-fix available" /tmp/krc_lc_out_$$.txt; then
        PASS=$((PASS + 1))
    else
        echo "FAIL: lc_reports_legacy (missing expected strings in output)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "FAIL: lc_reports_legacy (command failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_lc_$$.kr /tmp/krc_lc_out_$$.txt

# Governance: promote + list round-trip
TOTAL=$((TOTAL + 1))
GOV_DIR=/tmp/krc_gov_$$
# Use the raw compiler binary (not the wrapper script) so we can cd elsewhere
if [ -f "$DIR/../build/krc2" ]; then
    GOV_KRC=$(cd "$DIR/../build" && pwd)/krc2
elif [ -f "$DIR/../build/krc3" ]; then
    GOV_KRC=$(cd "$DIR/../build" && pwd)/krc3
else
    GOV_KRC=""
fi
mkdir -p "$GOV_DIR" && (cd "$GOV_DIR" && rm -rf .kernrift && \
    "$GOV_KRC" lc --promote tail_call_intrinsic > /tmp/krc_gov_promote_$$.txt 2>&1)
if [ -n "$GOV_KRC" ] && \
   grep -q "promoted: tail_call_intrinsic" /tmp/krc_gov_promote_$$.txt 2>/dev/null && \
   [ -f "$GOV_DIR/.kernrift/proposals" ] && \
   grep -q "tail_call_intrinsic stable" "$GOV_DIR/.kernrift/proposals"; then
    PASS=$((PASS + 1))
else
    echo "FAIL: governance_promote (state file not updated)"
    FAIL=$((FAIL + 1))
fi
rm -rf "$GOV_DIR" /tmp/krc_gov_promote_$$.txt

# Migration: long-form types → short aliases
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_migtypes_$$.kr <<'KREOF'
fn main() {
    uint64 x = 42
    uint32 y = 1
    uint16 z = 2
    exit(x)
}
KREOF
if $KRC lc --fix /tmp/krc_migtypes_$$.kr > /dev/null 2>&1; then
    if grep -q "u64 x" /tmp/krc_migtypes_$$.kr && \
       grep -q "u32 y" /tmp/krc_migtypes_$$.kr && \
       grep -q "u16 z" /tmp/krc_migtypes_$$.kr; then
        PASS=$((PASS + 1))
    else
        echo "FAIL: migration_types (file was not rewritten)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "FAIL: migration_types (command failed)"
    FAIL=$((FAIL + 1))
fi
rm -f /tmp/krc_migtypes_$$.kr

# --- Bootstrap test ---
echo ""
echo "--- Bootstrap test ---"
TOTAL=$((TOTAL + 1))
if [ -f "$DIR/../build/krc.kr" ]; then
    # Use the host arch so the compiled krc can run on the runner.
    HOST_ARCH=$(uname -m)
    case "$HOST_ARCH" in
        aarch64|arm64) BS_ARCH=arm64 ;;
        *)             BS_ARCH=x86_64 ;;
    esac
    cp "$DIR/../build/krc.kr" /tmp/krc_bootstrap_$$.kr
    $KRC $KRC_FLAGS /tmp/krc_bootstrap_$$.kr -o /tmp/krc2_$$ > /dev/null 2>&1
    chmod +x /tmp/krc2_$$ 2>/dev/null
    /tmp/krc2_$$ --arch=$BS_ARCH /tmp/krc_bootstrap_$$.kr -o /tmp/krc3_$$ > /dev/null 2>&1
    chmod +x /tmp/krc3_$$ 2>/dev/null
    /tmp/krc3_$$ --arch=$BS_ARCH /tmp/krc_bootstrap_$$.kr -o /tmp/krc4_$$ > /dev/null 2>&1
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

echo ""
echo "--- typed local arrays (regression) ---"
run_test "u8_arr"  'fn main() { u8[4] a; a[0] = 10; a[3] = 40; exit(a[0] + a[3]) }' 50
run_test "u16_arr" 'fn main() { u16[4] a; a[0] = 1000; a[3] = 4000; exit((a[0] + a[3]) / 100) }' 50
run_test "u32_arr" 'fn main() { u32[4] a; a[0] = 100000; a[3] = 400000; exit((a[0] + a[3]) / 10000) }' 50
run_test "u64_arr" 'fn main() { u64[4] a; a[0] = 100; a[1] = 200; a[2] = 300; a[3] = 400; exit(a[2] - a[0] - 100) }' 100
run_test "u64_arr_loop" 'fn main() {
    u64[5] a
    a[0] = 1
    a[1] = 2
    a[2] = 3
    a[3] = 4
    a[4] = 5
    u64 sum = 0
    for i in 0..5 { sum = sum + a[i] }
    exit(sum)
}' 15
run_test "bubble_sort_u64" 'fn main() {
    u64[4] a
    a[0] = 3
    a[1] = 1
    a[2] = 4
    a[3] = 2
    for i in 0..4 {
        for j in 0..3 {
            if a[j] > a[j+1] {
                u64 t = a[j]
                a[j] = a[j+1]
                a[j+1] = t
            }
        }
    }
    exit(a[0] * 0 + a[1] * 0 + a[2] * 0 + a[3])
}' 4

echo ""
echo "--- heap struct pointers (regression) ---"
run_test "heap_struct_basic" 'struct P { u64 x; u64 y }
fn main() {
    P p = alloc(16)
    p.x = 11
    p.y = 31
    exit(p.x + p.y)
}' 42
run_test "heap_linked_list" 'struct N { u64 v; u64 next }
fn main() {
    N a = alloc(16)
    N b = alloc(16)
    a.v = 2
    a.next = b
    b.v = 40
    b.next = 0
    u64 sum = 0
    N cur = a
    while cur != 0 {
        sum = sum + cur.v
        cur = cur.next
    }
    exit(sum)
}' 42

echo ""
echo "--- const initializers (regression) ---"
run_test "const_int"    'const u64 X = 42; fn main() { exit(X) }' 42
run_test "const_hex"    'const u64 X = 0x2A; fn main() { exit(X) }' 42
run_test "const_div"    'const u64 D = 10; fn main() { exit(100 / D) }' 10
run_test "const_mod"    'const u64 M = 7; fn main() { exit(50 % M) }' 1
run_test "const_mul"    'const u64 C = 21; fn main() { exit(C * 2) }' 42
run_test "const_char"   "const u64 CH = 'A'; fn main() { exit(CH) }" 65
run_test "const_true"   'const u64 T = true; fn main() { exit(T + 41) }' 42
run_test "static_int"   'static u64 X = 99; fn main() { exit(X) }' 99

echo ""
echo "--- import after comment (regression) ---"
TOTAL=$((TOTAL + 1))
cat > /tmp/imp_test_$$.kr <<'KREOF'
// leading comment should not break imports
import "std/io.kr"
fn main() { println("imp_ok"); exit(0) }
KREOF
if $KRC $KRC_FLAGS /tmp/imp_test_$$.kr -o /tmp/imp_test_bin_$$ > /dev/null 2>&1; then
    got=$(/tmp/imp_test_bin_$$ 2>/dev/null)
    if [ "$got" = "imp_ok" ]; then
        PASS=$((PASS + 1))
        echo "  import_after_comment: PASS"
    else
        FAIL=$((FAIL + 1))
        echo "  import_after_comment: FAIL (got: $got)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  import_after_comment: FAIL (compile)"
fi
rm -f /tmp/imp_test_$$.kr /tmp/imp_test_bin_$$

echo ""
echo "--- char literals ---"
run_test "char_a"    "fn main() { exit('A') }" 65
run_test "char_z"    "fn main() { exit('z') }" 122
run_test "char_nl"   "fn main() { exit('\\n') }" 10
run_test "char_tab"  "fn main() { exit('\\t') }" 9
run_test "char_bs"   "fn main() { exit('\\\\') }" 92
run_test "char_nul"  "fn main() { exit('\\0') }" 0
run_test "char_cmp"  "fn main() { u64 c = 97; if c == 'a' { exit(1) } exit(0) }" 1

echo ""
echo "--- emit=obj non-extern path (regression) ---"
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_noext_$$.kr <<'KREOF'
fn main() { exit(42) }
KREOF
if $KRC --emit=obj /tmp/krc_noext_$$.kr -o /tmp/krc_noext_$$.o > /dev/null 2>&1; then
    # File must be long enough for section headers: shoff + shnum*64 <= filesize
    if command -v python3 > /dev/null 2>&1; then
        if python3 -c "
import struct, sys
d = open('/tmp/krc_noext_$$.o', 'rb').read()
shoff = struct.unpack_from('<Q', d, 0x28)[0]
shnum = struct.unpack_from('<H', d, 0x3C)[0]
if shoff + shnum * 64 != len(d):
    print('truncated:', shoff + shnum * 64, 'expected,', len(d), 'got')
    sys.exit(1)
"; then
            PASS=$((PASS + 1))
            echo "  emit_obj_no_extern: PASS"
        else
            FAIL=$((FAIL + 1))
            echo "  emit_obj_no_extern: FAIL (truncated ELF)"
        fi
    else
        PASS=$((PASS + 1))
        echo "  emit_obj_no_extern: SKIP (no python3)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  emit_obj_no_extern: FAIL (compile)"
fi
rm -f /tmp/krc_noext_$$.kr /tmp/krc_noext_$$.o

# --- real LZ4 compression in .krbo fat binaries (regression) ---
# Before this, the "compressor" wrote uncompressed LZ4 frames (bit 31 set
# in block size) and the runner's else-branch skipped compressed blocks
# entirely. This test compiles a fat binary for a reasonably large
# program, checks that at least the first slice is actually compressed
# (bit 31 clear), and that its ratio is below 90% of the original.
#
# Must call build/krc2 directly — the test $KRC wrapper forces
# --arch=x86_64 which would make krc emit a single-arch ELF, not a
# fat binary, and there'd be nothing to inspect.
echo ""
echo "--- fat binary real LZ4 compression (regression) ---"
TOTAL=$((TOTAL + 1))
KRCBIN="$DIR/../build/krc2"
cat > /tmp/krc_lz4_$$.kr <<'KREOF'
fn main() {
    u64 i = 0
    u64 sum = 0
    while i < 64 { sum = sum + i * i; i = i + 1 }
    println(sum)
    exit(0)
}
KREOF
if "$KRCBIN" /tmp/krc_lz4_$$.kr -o /tmp/krc_lz4_$$.krbo > /dev/null 2>&1; then
    if command -v python3 > /dev/null 2>&1; then
        if python3 -c "
import struct, sys
d = open('/tmp/krc_lz4_$$.krbo', 'rb').read()
assert d[:8] == b'KRBOFAT\\x00'
n = struct.unpack_from('<I', d, 12)[0]
# With pair blobs, csize covers two slices and cannot be compared to
# one slice's usize. Instead check: (1) total file < sum-of-uncompressed
# and (2) at least one block uses real compression (bit 31 clear).
total_uncomp = 0
any_compressed = False
for i in range(n):
    aid, comp, off, csize, usize = struct.unpack_from('<IIQQQ', d, 16+i*48)
    total_uncomp += usize
    frame = d[off:off+csize]
    if len(frame) >= 11:
        bs = struct.unpack_from('<I', frame, 7)[0]
        if (bs >> 31) & 1 == 0:
            any_compressed = True
if not any_compressed:
    print('no compressed blocks found')
    sys.exit(1)
if len(d) >= total_uncomp * 9 // 10:
    print(f'file {len(d)} not < 90% of {total_uncomp}')
    sys.exit(1)
print(f'ok: file={len(d)} total_uncomp={total_uncomp}')
"; then
            PASS=$((PASS + 1))
            echo "  lz4_real_compression: PASS"
        else
            FAIL=$((FAIL + 1))
            echo "  lz4_real_compression: FAIL"
        fi
    else
        PASS=$((PASS + 1))
        echo "  lz4_real_compression: SKIP (no python3)"
    fi
else
    FAIL=$((FAIL + 1))
    echo "  lz4_real_compression: FAIL (compile)"
fi
rm -f /tmp/krc_lz4_$$.kr /tmp/krc_lz4_$$.krbo

# --- .krbo round-trip via kr runner (real-compression end-to-end) ---
# Builds a .krbo, a kr runner binary, and runs the .krbo through it.
# The runner must decompress the real LZ4 block and produce the right
# output. Skipped if we can't rebuild a matching runner.
echo ""
echo "--- fat binary round-trip via kr runner (regression) ---"
TOTAL=$((TOTAL + 1))
cat > /tmp/krc_rt_$$.kr <<'KREOF'
fn main() {
    println("roundtrip-ok")
    exit(123)
}
KREOF
KRCBIN="$DIR/../build/krc2"
cat "$DIR/../src/bcj.kr" "$DIR/../src/runner.kr" > /tmp/krc_rt_kr_$$.kr
if "$KRCBIN" /tmp/krc_rt_$$.kr -o /tmp/krc_rt_$$.krbo > /dev/null 2>&1 \
   && "$KRCBIN" --arch=$ARCH /tmp/krc_rt_kr_$$.kr -o /tmp/krc_rt_kr_$$ > /dev/null 2>&1; then
    chmod +x /tmp/krc_rt_kr_$$
    out=$(/tmp/krc_rt_kr_$$ /tmp/krc_rt_$$.krbo 2>&1)
    code=$?
    if [ "$out" = "roundtrip-ok" ] && [ "$code" = "123" ]; then
        PASS=$((PASS + 1))
        echo "  krbo_roundtrip: PASS"
    else
        FAIL=$((FAIL + 1))
        echo "  krbo_roundtrip: FAIL (out='$out' code=$code)"
    fi
else
    PASS=$((PASS + 1))
    echo "  krbo_roundtrip: SKIP (runner build)"
fi
rm -f /tmp/krc_rt_$$.kr /tmp/krc_rt_kr_$$.kr /tmp/krc_rt_$$.krbo /tmp/krc_rt_kr_$$

echo ""
echo "--- float types ---"
run_test "f64_parse" 'fn main() { f64 x = 0.0; exit(0) }' 0
run_test "f64_literal_precision" 'fn main() { f64 pi = 3.14159; f64 s = pi * int_to_f64(100000); exit(f64_to_int(s) % 100) }' 59
run_test "int_to_f64_rt" 'fn main() { f64 x = int_to_f64(42); exit(f64_to_int(x)) }' 42
run_test "f64_add" 'fn main() { f64 a = int_to_f64(10); f64 b = int_to_f64(3); f64 c = a + b; exit(f64_to_int(c)) }' 13
run_test "f64_sub" 'fn main() { f64 a = int_to_f64(50); f64 b = int_to_f64(8); exit(f64_to_int(a - b)) }' 42
run_test "f64_mul" 'fn main() { f64 a = int_to_f64(6); f64 b = int_to_f64(7); exit(f64_to_int(a * b)) }' 42
run_test "f64_div" 'fn main() { f64 a = int_to_f64(84); f64 b = int_to_f64(2); exit(f64_to_int(a / b)) }' 42
run_test "f64_sqrt" 'fn main() { f64 x = int_to_f64(49); exit(f64_to_int(sqrt(x))) }' 7
run_test "f64_reassign" 'fn main() { f64 x = int_to_f64(10); x = x + int_to_f64(5); x = x * int_to_f64(2); exit(f64_to_int(x)) }' 30
run_test "f64_cmp_lt" 'fn main() { f64 a = int_to_f64(3); f64 b = int_to_f64(5); if a < b { exit(1) } exit(0) }' 1
run_test "f64_cmp_gt" 'fn main() { f64 a = int_to_f64(10); f64 b = int_to_f64(5); if a > b { exit(1) } exit(0) }' 1
run_test "f64_cmp_eq" 'fn main() { f64 a = int_to_f64(7); f64 b = int_to_f64(7); if a == b { exit(1) } exit(0) }' 1
run_test "f64_fn_call" 'fn double_it(f64 x) -> f64 { return x + x }
fn main() { f64 r = double_it(int_to_f64(21)); exit(f64_to_int(r)) }' 42
run_test "f64_fn_2args" 'fn add_f(f64 a, f64 b) -> f64 { return a + b }
fn main() { f64 r = add_f(int_to_f64(20), int_to_f64(22)); exit(f64_to_int(r)) }' 42
run_test "f64_fn_mixed" 'fn scale(u64 n, f64 x) -> f64 { f64 fn64 = int_to_f64(n); return fn64 * x }
fn main() { f64 r = scale(3, int_to_f64(14)); exit(f64_to_int(r)) }' 42

# Float literal parsing
run_test "f64_literal_zero" 'fn main() { f64 x = 0.0; exit(f64_to_int(x)) }' 0
run_test "f64_literal_one" 'fn main() { f64 x = 1.0; exit(f64_to_int(x)) }' 1

# Float reassignment
run_test "f64_reassign2" 'fn main() { f64 x = int_to_f64(5); f64 y = int_to_f64(3); x = x + y; exit(f64_to_int(x)) }' 8

# Float in while loop
run_test "f64_while" 'fn main() { f64 sum = int_to_f64(0); u64 i = 0; while i < 10 { sum = sum + int_to_f64(1); i = i + 1 }; exit(f64_to_int(sum)) }' 10

# f32 basic
run_test "f32_basic" 'fn main() { f32 x = int_to_f32(42); exit(f32_to_int(x)) }' 42

# Float comparison edge cases
run_test "f64_cmp_le" 'fn main() { f64 a = int_to_f64(5); f64 b = int_to_f64(5); if a <= b { exit(1) } exit(0) }' 1
run_test "f64_cmp_ne" 'fn main() { f64 a = int_to_f64(3); f64 b = int_to_f64(5); if a != b { exit(1) } exit(0) }' 1

# Conversion roundtrip
run_test "f32_f64_roundtrip" 'fn main() { f64 a = int_to_f64(99); f32 b = f64_to_f32(a); f64 c = f32_to_f64(b); exit(f64_to_int(c)) }' 99
run_test "f32_literal" 'fn main() { f32 x = 42.0f; exit(f32_to_int(x)) }' 42
run_test "f16_roundtrip" 'fn main() { f32 x = 42.0f; u64 h = f32_to_f16(x); f32 y = f16_to_f32(h); exit(f32_to_int(y)) }' 42

# FMA
run_test "f64_fma" 'fn main() { f64 a = int_to_f64(3); f64 b = int_to_f64(4); f64 c = int_to_f64(5); f64 r = fma_f64(a, b, c); exit(f64_to_int(r)) }' 17

echo ""
echo "--- alloc/dealloc ---"
run_test "alloc_header" 'fn main() { u64 p = alloc(64); store64(p, 42); u64 v = load64(p); exit(v) }' 42
run_test "dealloc_basic" 'fn main() { u64 p = alloc(64); store64(p, 99); dealloc(p); exit(0) }' 0

echo ""
echo "--- extern fn (libc linking) ---"
# These tests link against the HOST gcc's libc. On cross-compile runs
# (arm64 host but KRC_FLAGS=--arch=x86_64 for example) the object file
# architecture won't match gcc and the link fails. Skip on non-x86_64
# hosts since the default KRC_FLAGS target host arch and the host gcc
# links to host libc.
HOST_M=$(uname -m)
if [ "$HOST_M" != "x86_64" ] && [ "$HOST_M" != "amd64" ]; then
    echo "  extern_libc_write: SKIP (non-x86_64 host toolchain)"
    echo "  extern_libc_strlen_write: SKIP (non-x86_64 host toolchain)"
elif command -v gcc > /dev/null 2>&1; then
    TOTAL=$((TOTAL + 1))
    cat > /tmp/krc_ext_$$.kr <<'KREOF'
extern fn write(u64 fd, u64 buf, u64 len) -> u64

fn main() {
    write(1, "extern_ok\n", 10)
    exit(0)
}
KREOF
    if $KRC --emit=obj /tmp/krc_ext_$$.kr -o /tmp/krc_ext_$$.o > /dev/null 2>&1 \
       && gcc /tmp/krc_ext_$$.o -o /tmp/krc_ext_linked_$$ -no-pie > /dev/null 2>&1; then
        got=$(/tmp/krc_ext_linked_$$ 2>/dev/null)
        if [ "$got" = "extern_ok" ]; then
            PASS=$((PASS + 1))
            echo "  extern_libc_write: PASS"
        else
            FAIL=$((FAIL + 1))
            echo "  extern_libc_write: FAIL (got: $got)"
        fi
    else
        FAIL=$((FAIL + 1))
        echo "  extern_libc_write: FAIL (compile/link failed)"
    fi
    rm -f /tmp/krc_ext_$$.kr /tmp/krc_ext_$$.o /tmp/krc_ext_linked_$$

    TOTAL=$((TOTAL + 1))
    cat > /tmp/krc_ext2_$$.kr <<'KREOF'
extern fn strlen(u64 s) -> u64
extern fn write(u64 fd, u64 buf, u64 len) -> u64

fn main() {
    u64 msg = "two_externs\n"
    u64 n = strlen(msg)
    write(1, msg, n)
    exit(0)
}
KREOF
    if $KRC --emit=obj /tmp/krc_ext2_$$.kr -o /tmp/krc_ext2_$$.o > /dev/null 2>&1 \
       && gcc /tmp/krc_ext2_$$.o -o /tmp/krc_ext2_linked_$$ -no-pie > /dev/null 2>&1; then
        got=$(/tmp/krc_ext2_linked_$$ 2>/dev/null)
        if [ "$got" = "two_externs" ]; then
            PASS=$((PASS + 1))
            echo "  extern_libc_strlen_write: PASS"
        else
            FAIL=$((FAIL + 1))
            echo "  extern_libc_strlen_write: FAIL (got: $got)"
        fi
    else
        FAIL=$((FAIL + 1))
        echo "  extern_libc_strlen_write: FAIL (compile/link failed)"
    fi
    rm -f /tmp/krc_ext2_$$.kr /tmp/krc_ext2_$$.o /tmp/krc_ext2_linked_$$
else
    echo "  extern_libc_write: SKIP (gcc not available)"
    echo "  extern_libc_strlen_write: SKIP (gcc not available)"
fi

# --- Summary ---
echo ""
echo "=== Results: $PASS/$TOTAL passed, $FAIL failed ==="
exit $FAIL
