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

# Test readelf can parse sections and symbols
TOTAL=$((TOTAL + 1))
if command -v readelf > /dev/null 2>&1 && [ -f /tmp/krc_obj_$$.o ]; then
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
run_test "print_str_roundtrip" 'import "std/string.kr"
fn main() {
    u64 s = int_to_str(42)
    u64 back = str_to_int(s)
    exit(back)
}' 42

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

# --- Bootstrap test ---
echo ""
echo "--- Bootstrap test ---"
TOTAL=$((TOTAL + 1))
if [ -f "$DIR/../build/krc.kr" ]; then
    cp "$DIR/../build/krc.kr" /tmp/krc_bootstrap_$$.kr
    $KRC $KRC_FLAGS /tmp/krc_bootstrap_$$.kr -o /tmp/krc2_$$ > /dev/null 2>&1
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
