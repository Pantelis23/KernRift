//! Linux AArch64 runtime blob — hand-assembled machine code.
//! Implements `_start` and all `__kr_*` functions using Linux syscalls via `svc #0`.
//!
//! Layout (1072 bytes total):
//!   0x000 .. 0x24f  executable code (11 original functions)
//!   0x250 .. 0x25a  inline strings ("/bin/sh\0", "-c\0")
//!   0x25b .. 0x25f  padding (5 bytes)
//!   0x260 .. 0x277  data area: envp(8) + heap_ptr(8) + heap_remaining(8) = 24 bytes
//!   0x278 .. 0x2df  file I/O functions (5 functions via openat/read/write/close/fstat)
//!   0x2e0 .. 0x42f  formatting functions (3 functions)
//!
//! AArch64 Linux syscall convention:
//!   syscall nr in x8, args in x0-x5, return in x0
//!   `svc #0` (0xD4000001)
//!
//! Calling convention: args in x0-x7, return in x0, callee-saved x19-x28+x29(fp)+x30(lr).
//! `bl` saves return address in x30 (lr).

use super::RuntimeBlob;

// AArch64 instruction encoding helpers (compile-time).
// All instructions are 4 bytes, little-endian.

// Data area layout (24 bytes at end of blob):
//   +0:  envp           (u64)
//   +8:  heap_ptr       (u64)
//   +16: heap_remaining (u64)

// We build the blob as a flat byte array with comments on each 4-byte instruction.
// For data access we use ADR (PC-relative ±1MB) to get the address of the data area.

// Linux AArch64 syscall numbers:
//   write      = 64
//   exit_group = 94
//   mmap       = 222
//   clone      = 220
//   execve     = 221
//   wait4      = 260

pub static BLOB: RuntimeBlob = RuntimeBlob {
    code: &CODE,
    symbols: &[
        ("_start", 0x000),
        ("__kr_exit", OFF_EXIT),
        ("__kr_write", OFF_WRITE),
        ("__kr_mmap_alloc", OFF_MMAP),
        ("__kr_alloc", OFF_ALLOC),
        ("__kr_dealloc", OFF_DEALLOC),
        ("__kr_getenv", OFF_GETENV),
        ("__kr_exec", OFF_EXEC),
        ("__kr_str_copy", OFF_STR_COPY),
        ("__kr_str_cat", OFF_STR_CAT),
        ("__kr_str_len", OFF_STR_LEN),
        ("__kr_file_open", 0x278),
        ("__kr_file_read", 0x2a0),
        ("__kr_file_write", 0x2ac),
        ("__kr_file_close", 0x2b8),
        ("__kr_file_size", 0x2c4),
        ("__kr_fmt_uint", 0x2e0),
        ("__kr_fmt_int", 0x368),
        ("__kr_fmt_hex", 0x3a0),
    ],
    // main_call_fixup: byte offset of the BL main instruction.
    // The linker computes imm26 = ((target - bl_offset) >> 2) & 0x03FFFFFF
    // and writes (0x94000000 | imm26) at this offset.
    main_call_fixup: OFF_BL_MAIN,
    iat_base_data_offset: None,
};

// We define offsets as u32 constants so we can reference them in the symbol table.
// Each AArch64 instruction is exactly 4 bytes.

// === _start (offset 0x00) ===
// Linux kernel entry: SP -> argc, argv, envp on stack (same layout as x86_64).
// x29 = 0 (frame pointer), save envp to data slot, bl main, bl __kr_exit
//
// Stack at entry: [SP+0]=argc, [SP+8]=argv[0], ..., [SP+8+8*argc]=NULL, envp...
// envp = SP + 8 + 8*(argc+1)

// Instruction count for _start:
//   0x00: mov x29, #0           (frame pointer sentinel)
//   0x04: mov x30, #0           (link register sentinel)
//   0x08: ldr x0, [sp]          ; argc -> x0 (first arg to main)
//   0x0c: add x1, sp, #8        ; argv -> x1
//   0x10: add x2, x0, #1        ; argc+1
//   0x14: add x2, x1, x2, lsl #3 ; envp = argv + (argc+1)*8
//   0x18: adr x9, <data_envp>   ; address of envp slot
//   0x1c: str x2, [x9]          ; save envp
//   0x20: bl main               ; FIXUP
//   0x24: bl __kr_exit           ; exit(x0) - return value in x0

const OFF_BL_MAIN: u32 = 0x20;

// === __kr_exit (offset 0x28) ===
const OFF_EXIT: u32 = 0x28;
// 0x28: mov x8, #94             ; __NR_exit_group
// 0x2c: svc #0

// === __kr_write (offset 0x30) ===
const OFF_WRITE: u32 = 0x30;
// 0x30: mov x8, #64             ; __NR_write
// 0x34: svc #0
// 0x38: ret

// === __kr_mmap_alloc (offset 0x3c) ===
const OFF_MMAP: u32 = 0x3c;
// x0 = size (from caller's x0)
// 0x3c: mov x1, x0              ; len = size
// 0x40: mov x0, #0              ; addr = NULL
// 0x44: mov x2, #3              ; PROT_READ|PROT_WRITE
// 0x48: mov x3, #0x22           ; MAP_PRIVATE|MAP_ANONYMOUS
// 0x4c: mov x4, #-1 (movn)     ; fd = -1
// 0x50: mov x5, #0              ; offset = 0
// 0x54: mov x8, #222            ; __NR_mmap
// 0x58: svc #0
// 0x5c: ret

// === __kr_alloc (offset 0x60) ===
const OFF_ALLOC: u32 = 0x60;
// x0 = requested size
// 0x60: add x0, x0, #15         ; align up to 16
// 0x64: and x0, x0, #~15        ; ~15 = 0xFFFF...FFF0
// 0x68: adr x9, <data_heap_remaining>
// 0x6c: ldr x1, [x9]            ; heap_remaining
// 0x70: cmp x0, x1
// 0x74: b.ls .have_space
// --- need more space ---
// 0x78: stp x0, x30, [sp, #-16]! ; save aligned size + lr
// 0x7c: mov x0, #0x400000       ; 4MB  (movz x0, #0x40, lsl #16)
// 0x80: bl __kr_mmap_alloc
// 0x84: adr x9, <data_heap_ptr>
// 0x88: str x0, [x9]            ; heap_ptr = result
// 0x8c: mov x1, #0x400000       ; 4MB
// 0x90: adr x9, <data_heap_remaining>
// 0x94: str x1, [x9]            ; heap_remaining = 4MB
// 0x98: ldp x0, x30, [sp], #16  ; restore aligned size + lr
// .have_space:
// 0x9c: adr x9, <data_heap_ptr>
// 0xa0: ldr x1, [x9]            ; return value = heap_ptr
// 0xa4: add x2, x1, x0          ; new heap_ptr
// 0xa8: str x2, [x9]            ; heap_ptr += size
// 0xac: adr x9, <data_heap_remaining>
// 0xb0: ldr x2, [x9]
// 0xb4: sub x2, x2, x0          ; heap_remaining -= size
// 0xb8: str x2, [x9]
// 0xbc: mov x0, x1               ; return old heap_ptr
// 0xc0: ret

// === __kr_dealloc (offset 0xc4) ===
const OFF_DEALLOC: u32 = 0xc4;
// 0xc4: ret

// === __kr_getenv (offset 0xc8) ===
const OFF_GETENV: u32 = 0xc8;

// === __kr_exec (offset ~) ===
// We'll compute this after getenv.
// getenv is about 15 instructions.

// AArch64 instruction encoding reference (used to compute byte values below):
//
// MOVZ Xd, #imm16        = 0xD2800000 | (imm16 << 5) | Rd
// MOVZ Xd, #imm16 LSL16  = 0xD2A00000 | (imm16 << 5) | Rd
// MOVN Xd, #imm16        = 0x92800000 | (imm16 << 5) | Rd
// MOV Xd, Xn (ORR)       = 0xAA0003E0 | (Rn << 16) | Rd
// ADD Xd, Xn, #imm12     = 0x91000000 | (imm12 << 10) | (Rn << 5) | Rd
// ADD Xd, Xn, Xm LSL #s  = 0x8B000000 | (Rm << 16) | (s << 10) | (Rn << 5) | Rd
// SUB Xd, Xn, Xm         = 0xCB000000 | (Rm << 16) | (Rn << 5) | Rd
// AND Xd, Xn, #~15       = 0x9240F000 | (Rn << 5) | Rd
// LDR Xd, [Xn]           = 0xF9400000 | (Rn << 5) | Rd
// STR Xd, [Xn]           = 0xF9000000 | (Rn << 5) | Rd
// LDRB Wd, [Xn]          = 0x39400000 | (Rn << 5) | Rd
// STRB Wd, [Xn]          = 0x39000000 | (Rn << 5) | Rd
// CMP Xn, Xm             = 0xEB00001F | (Rm << 16) | (Rn << 5)
// CMP Xn, #imm12         = 0xF100001F | (imm12 << 10) | (Rn << 5)
// CBZ Xn, off             = 0xB4000000 | ((off>>2) << 5) | Rn
// CBZ Wn, off             = 0x34000000 | ((off>>2) << 5) | Rn
// B.LS off                = 0x54000000 | ((off>>2) << 5) | 9
// B.NE off                = 0x54000000 | ((off>>2) << 5) | 1
// B off                   = 0x14000000 | ((off>>2) & 0x03FFFFFF)
// BL off                  = 0x94000000 | ((off>>2) & 0x03FFFFFF)
// ADR Xd, off             = ((off&3)<<29) | 0x10000000 | ((off>>2)<<5) | Rd
// SVC #0                  = 0xD4000001
// RET                     = 0xD65F03C0
// STP Xt1,Xt2,[Xn,#imm]! = 0xA9800000 | ((imm/8)<<15) | (Xt2<<10) | (Xn<<5) | Xt1
// LDP Xt1,Xt2,[Xn],#imm  = 0xA8C00000 | ((imm/8)<<15) | (Xt2<<10) | (Xn<<5) | Xt1

// After careful counting, here's the complete layout:

// DATA_AREA offset = total_code_and_strings rounded up to 8.
// Let me count instructions precisely:

// _start: 10 instructions = 40 bytes (0x00-0x27)
// __kr_exit: 2 = 8 bytes (0x28-0x2f)
// __kr_write: 3 = 12 bytes (0x30-0x3b)
// __kr_mmap_alloc: 9 = 36 bytes (0x3c-0x5f)
// __kr_alloc: 25 = 100 bytes (0x60-0xc3)
// __kr_dealloc: 1 = 4 bytes (0xc4-0xc7)
// __kr_getenv: 18 = 72 bytes (0xc8-0x10f)
// __kr_exec: 28 = 112 bytes (0x110-0x17f)
// __kr_str_copy: 8 = 32 bytes (0x180-0x19f)
// __kr_str_cat: 12 = 48 bytes (0x1a0-0x1cf)
// __kr_str_len: 7 = 28 bytes (0x1d0-0x1eb)
// strings: "/bin/sh\0" (8) + "-c\0" (3) = 11 bytes (0x1ec-0x1f6)
// padding: 1 byte to align to 8 (0x1f7)
// data area: 24 bytes (0x1f8-0x20f)
// Total: 528 bytes

// Wait, I need to re-count more carefully and adjust ADR offsets.
// Let me do a precise layout.

// I'll build the blob step by step, writing out the actual bytes.

// Offset tracking:
// 0x000: _start
// 0x028: __kr_exit
// 0x030: __kr_write
// 0x03c: __kr_mmap_alloc
// 0x060: __kr_alloc
// 0x0c4: __kr_dealloc
// 0x0c8: __kr_getenv
// 0x110: __kr_exec
// 0x180: __kr_str_copy
// 0x1a0: __kr_str_cat
// 0x1d0: __kr_str_len
// 0x1ec: strings
// 0x1f8: data_area (padded to 8)

// Now I need to recount __kr_alloc. Let me be very precise.

// __kr_alloc at 0x60:
// 0x60: add x0, x0, #15           ; 1
// 0x64: and x0, x0, #~15          ; 2
// 0x68: adr x9, data+16           ; 3 (heap_remaining)
// 0x6c: ldr x1, [x9]              ; 4
// 0x70: cmp x0, x1                ; 5
// 0x74: b.ls .have_space           ; 6  -> 0x9c
// 0x78: stp x0, x30, [sp, #-16]!  ; 7
// 0x7c: movz x0, #0x40, lsl #16   ; 8  (0x400000)
// 0x80: bl __kr_mmap_alloc         ; 9  -> 0x3c (offset = 0x3c - 0x80 = -0x44)
// 0x84: adr x9, data+8            ; 10 (heap_ptr)
// 0x88: str x0, [x9]              ; 11
// 0x8c: movz x1, #0x40, lsl #16   ; 12 (0x400000)
// 0x90: adr x9, data+16           ; 13 (heap_remaining)
// 0x94: str x1, [x9]              ; 14
// 0x98: ldp x0, x30, [sp], #16    ; 15
// .have_space:
// 0x9c: adr x9, data+8            ; 16 (heap_ptr)
// 0xa0: ldr x1, [x9]              ; 17
// 0xa4: add x2, x1, x0            ; 18
// 0xa8: str x2, [x9]              ; 19
// 0xac: adr x9, data+16           ; 20 (heap_remaining)
// 0xb0: ldr x2, [x9]              ; 21
// 0xb4: sub x2, x2, x0            ; 22
// 0xb8: str x2, [x9]              ; 23
// 0xbc: mov x0, x1                ; 24
// 0xc0: ret                       ; 25
// That's 25 instructions = 100 bytes. 0x60 + 100 = 0xc4. Good.

// __kr_getenv at 0xc8:
// x0 = name pointer
// 0xc8: adr x9, data              ; envp slot
// 0xcc: ldr x1, [x9]              ; x1 = envp
// .env_loop:
// 0xd0: ldr x2, [x1]              ; entry = envp[i]
// 0xd4: cbz x2, .not_found        ; -> +14 instr = +56 = 0x10c
// 0xd8: mov x3, x0                ; save name ptr
// 0xdc: mov x4, x2                ; save entry ptr
// .cmp_loop:
// 0xe0: ldrb w5, [x3]             ; *name
// 0xe4: cbz x5, .check_eq         ; -> +4 instr = +16 = 0xf4  (name ended)
// 0xe8: ldrb w6, [x4]             ; *entry
// 0xec: cmp x5, x6
// 0xf0: b.ne .next                ; -> +5 instr = +20 = 0x104
// 0xf4: add x3, x3, #1
// 0xf8: add x4, x4, #1
// 0xfc: b .cmp_loop               ; -> 0xe0 (offset = 0xe0 - 0xfc = -0x1c)
// .check_eq:
// 0x100: ldrb w5, [x4]
// 0x104: cmp x5, #0x3d             ; '='
// 0x108: b.ne .next                ; -> 0x10c? No, .next adds 8 to x1 then loops
//   Actually, if '=' doesn't match, we go to .next
//   .next is: add x1, x1, #8; b .env_loop
// Let me re-lay this out:
//
// 0xc8: adr x9, data              ; envp slot
// 0xcc: ldr x1, [x9]              ; x1 = envp
// .env_loop: (0xd0)
// 0xd0: ldr x2, [x1]              ; entry
// 0xd4: cbz x2, .not_found
// 0xd8: mov x3, x0                ; name copy
// 0xdc: mov x4, x2                ; entry copy
// .cmp: (0xe0)
// 0xe0: ldrb w5, [x3]
// 0xe4: cbz x5, .check_eq         ; if *name == 0
// 0xe8: ldrb w6, [x4]
// 0xec: cmp x5, x6
// 0xf0: b.ne .next
// 0xf4: add x3, x3, #1
// 0xf8: add x4, x4, #1
// 0xfc: b .cmp
// .check_eq: (0x100)
// 0x100: ldrb w5, [x4]
// 0x104: cmp x5, #0x3d            ; '='
// 0x108: b.ne .next
// 0x10c: add x0, x4, #1           ; return past '='
// 0x110: ret
// .next: (0x114)
// 0x114: add x1, x1, #8
// 0x118: b .env_loop
// .not_found: (0x11c)
// 0x11c: mov x0, #0
// 0x120: ret
// That's 23 instructions = 92 bytes. 0xc8 + 92 = 0x124.
// But this pushes everything down. Let me redefine:

// Hmm, but I already set OFF_GETENV = 0xc8 above. Let me recount from the beginning.
// Let me just use actual offset constants computed from instruction counts.

// __kr_exec: let me count
// 0: stp x19, x20, [sp, #-32]!  ; save callee-saved + lr
// 1: stp x21, x30, [sp, #16]
// 2: mov x19, x0                ; save cmd
// 3: mov x8, #220               ; __NR_clone
// 4: mov x0, #17                ; SIGCHLD
// 5: mov x1, #0
// 6: mov x2, #0
// 7: mov x3, #0
// 8: mov x4, #0
// 9: svc #0
// 10: cbz x0, .child
// 11: mov x19, x0               ; wait, x19 already used for cmd. Use x20 for pid.
//     Actually let me restructure:
// 0: stp x19, x30, [sp, #-16]!
// 1: mov x19, x0                ; save cmd
// 2: mov x8, #220               ; __NR_clone
// 3: mov x0, #17                ; SIGCHLD
// 4: mov x1, #0
// 5: mov x2, #0
// 6: mov x3, #0
// 7: mov x4, #0
// 8: svc #0
// 9: cbz x0, .child             ; child returns 0
// parent:
// 10: mov x20, x0               ; pid -- oops x20 not saved. Let me save x19,x20,x30.
//
// Let me use a simpler prologue:
// 0: stp x29, x30, [sp, #-32]!
// 1: stp x19, x20, [sp, #16]
// 2: mov x19, x0                ; save cmd
// 3: mov x8, #220               ; __NR_clone
// 4: mov x0, #17                ; SIGCHLD
// 5: mov x1, #0                 ; child_stack
// 6: mov x2, #0                 ; parent_tidptr
// 7: mov x3, #0                 ; tls
// 8: mov x4, #0                 ; child_tidptr
// 9: svc #0
// 10: cbz x0, .child
// 11: mov x20, x0               ; save pid
// 12: sub sp, sp, #16           ; space for status
// 13: mov x0, x20               ; pid
// 14: mov x1, sp                ; &status
// 15: mov x2, #0                ; options
// 16: mov x3, #0                ; rusage
// 17: mov x8, #260              ; __NR_wait4
// 18: svc #0
// 19: ldr w0, [sp]              ; status (32-bit)
// 20: add sp, sp, #16
// 21: and w1, w0, #0xFF         ; status & 0xFF (signal)
// 22: cbnz w1, .signal_death
// 23: lsr w0, w0, #8            ; exit code
// 24: and w0, w0, #0xFF
// 25: b .done
// .signal_death:
// 26: mov w0, #1
// .done:
// 27: ldp x19, x20, [sp, #16]
// 28: ldp x29, x30, [sp], #32
// 29: ret
// .child: (from instruction 10 branch)
// 30: adr x0, .sh_path          ; "/bin/sh"
// 31: mov x3, #0                ; NULL terminator for argv
// 32: str x3, [sp, #-32]!       ; push NULL  (use sp space)
//     Actually building argv on stack for execve:
//     argv = ["/bin/sh", "-c", cmd, NULL]
//     Let me use SP-based array:
// 30: adr x0, .sh_path          ; "/bin/sh"
// 31: adr x1, .dash_c           ; "-c"
// 32: mov x2, x19               ; cmd
// 33: mov x3, #0                ; NULL
// 34: stp x2, x3, [sp, #-32]!  ; sp[0]=cmd, sp[8]=NULL
// 35: stp x0, x1, [sp, #-16]!  ; wait, this is wrong. Let me think about stack layout.
//     I need argv to be: [ptr_to_sh, ptr_to_c, ptr_to_cmd, NULL]
//     That's 4 pointers = 32 bytes.
//     SP-=32: [sp]=sh, [sp+8]=-c, [sp+16]=cmd, [sp+24]=NULL
// 30: adr x5, .sh_path
// 31: adr x6, .dash_c
// 32: sub sp, sp, #32
// 33: str x5, [sp]              ; argv[0] = "/bin/sh"
// 34: str x6, [sp, #8]          ; argv[1] = "-c"
// 35: str x19, [sp, #16]        ; argv[2] = cmd
// 36: str xzr, [sp, #24]        ; argv[3] = NULL
// 37: mov x0, x5                ; filename = "/bin/sh"
// 38: mov x1, sp                ; argv
// 39: adr x9, data              ; envp slot
// 40: ldr x2, [x9]              ; envp
// 41: mov x8, #221              ; __NR_execve
// 42: svc #0
// 43: mov x0, #127              ; exit 127 on failure
// 44: mov x8, #94               ; __NR_exit_group
// 45: svc #0
//
// That's 46 instructions total for __kr_exec = 184 bytes. That's big but OK.
// Let me recount: instructions 0-29 (parent) + 30-45 (child) = 46 instructions.
// But wait, instructions 0-29 is 30 instructions, plus 30-45 is 16 = 46 total.
// 46 * 4 = 184 bytes.

// Hmm, that's a lot. Let me optimize:
// The CBZ in instruction 10 branches to .child.
// .child starts at instruction 30. So offset = (30-10)*4 = 80 bytes.

// Let me also handle the W register AND for signal check.
// AND W1, W0, #0xFF: This is a 32-bit logical immediate.
// For W registers: 0 00 100100 N(0) immr(6) imms(6) Rn(5) Rd(5)
// For #0xFF: N=0, immr=0, imms=0b000111 (7)
// = 0x12000000 | (0 << 22) | (0 << 16) | (7 << 10) | (Rn << 5) | Rd
// = 0x12001C00 | (Rn << 5) | Rd

// LSR W0, W0, #8: UBFM W0, W0, #8, #31
// = 0x53000000 | (8 << 16) | (31 << 10) | (Rn << 5) | Rd
// = 0x53087C00 | (Rn << 5) | Rd

// LDR W0, [SP]: 32-bit load
// = 0xB9400000 | (SP << 5) | Rd

// CBNZ W1, offset: 32-bit compare-branch
// = 0x35000000 | (imm19 << 5) | Rn

// STR XZR, [Xn, #off]: XZR is register 31
// = 0xF9000000 | ((off/8) << 10) | (Rn << 5) | 31

// For STP with pre-index for frame save:
// STP X29, X30, [SP, #-32]!
// imm7 = -32/8 = -4, 7-bit = 0x7C
// = 0xA9800000 | (0x7C << 15) | (X30 << 10) | (SP << 5) | X29
// = 0xA9800000 | (0x7C << 15) | (30 << 10) | (31 << 5) | 29
// Let me compute: 0x7C << 15 = 0x003E0000
// 30 << 10 = 0x7800
// 31 << 5 = 0x3E0
// = 0xA9800000 | 0x003E0000 | 0x7800 | 0x3E0 | 29
// = 0xA9BE7BFD

// STP X19, X20, [SP, #16]
// Signed offset: 16/8=2
// STP (signed offset): 0xA9000000 | (imm7 << 15) | (Xt2 << 10) | (Xn << 5) | Xt1
// = 0xA9000000 | (2 << 15) | (20 << 10) | (31 << 5) | 19
// = 0xA9000000 | 0x10000 | 0x5000 | 0x3E0 | 19
// = 0xA90153F3

// LDP X19, X20, [SP, #16]
// LDP (signed offset): 0xA9400000 | (imm7 << 15) | (Xt2 << 10) | (Xn << 5) | Xt1
// = 0xA9400000 | (2 << 15) | (20 << 10) | (31 << 5) | 19
// = 0xA94153F3

// LDP X29, X30, [SP], #32
// Post-index: 0xA8C00000 | (imm7 << 15) | (Xt2 << 10) | (Xn << 5) | Xt1
// imm7 = 32/8 = 4
// = 0xA8C00000 | (4 << 15) | (30 << 10) | (31 << 5) | 29
// = 0xA8C00000 | 0x20000 | 0x7800 | 0x3E0 | 29
// = 0xA8C27BFD

// OK this is getting complex. Let me just carefully write out all the bytes
// for the entire blob, using the const helper functions where possible
// but verifying every encoding.

// STRATEGY: I'll define the blob as a series of u32 constants (one per instruction),
// then convert to bytes at the end. This is cleaner.

// After the instructions come the string data and data area.

// Let me define the final layout with exact offsets:

// Since I can't easily build a &[u8] from computed u32s in const context in stable Rust,
// I'll write out the hex bytes directly, but compute them from the instruction encodings.

// Let me write a more manageable version. I'll reduce __kr_exec instruction count
// by being more efficient.

// FINAL PLAN - precise instruction list with offsets:
//
// 0x000: _start (10 instrs, 40 bytes)
// 0x028: __kr_exit (2 instrs, 8 bytes)
// 0x030: __kr_write (3 instrs, 12 bytes)
// 0x03C: __kr_mmap_alloc (9 instrs, 36 bytes)
// 0x060: __kr_alloc (25 instrs, 100 bytes)
// 0x0C4: __kr_dealloc (1 instr, 4 bytes)
// 0x0C8: __kr_getenv (23 instrs, 92 bytes)
// 0x124: __kr_exec (38 instrs, 152 bytes)
// 0x1BC: __kr_str_copy (8 instrs, 32 bytes)
// 0x1DC: __kr_str_cat (12 instrs, 48 bytes)
// 0x20C: __kr_str_len (7 instrs, 28 bytes)
// 0x228: sh_path "/bin/sh\0" (8 bytes)
// 0x230: dash_c "-c\0" (3 bytes)
// 0x233: padding (5 bytes to align to 0x238)
// 0x238: data_area (24 bytes)
// 0x250: end (total 592 bytes)

// Hmm, let me recount __kr_exec more carefully. I had 46 instructions before,
// but I think I can do it in fewer.

// __kr_exec:
// Prologue:
// 0: stp x29, x30, [sp, #-32]!   ; save frame
// 1: stp x19, x20, [sp, #16]     ; save callee-saved
// 2: mov x19, x0                  ; save cmd
// Clone:
// 3: mov x0, #17                  ; SIGCHLD (flags)
// 4: mov x1, #0                   ; child_stack
// 5: mov x2, #0                   ; parent_tidptr
// 6: mov x3, #0                   ; tls
// 7: mov x4, #0                   ; child_tidptr
// 8: mov x8, #220                 ; __NR_clone
// 9: svc #0
// 10: cbz x0, .child
// Parent (wait):
// 11: mov x20, x0                 ; save pid
// 12: sub sp, sp, #16             ; space for status
// 13: mov x0, x20                 ; pid
// 14: mov x1, sp                  ; &status
// 15: mov x2, #0                  ; options
// 16: mov x3, #0                  ; rusage
// 17: mov x8, #260                ; __NR_wait4
// 18: svc #0
// 19: ldr w0, [sp]                ; status
// 20: add sp, sp, #16
// Check signal:
// 21: tst w0, #0xFF               ; ANDS WZR, W0, #0xFF
// 22: b.ne .signal_death
// 23: ubfx w0, w0, #8, #8         ; (status >> 8) & 0xFF
// 24: b .done
// .signal_death:
// 25: mov w0, #1
// .done:
// 26: ldp x19, x20, [sp, #16]
// 27: ldp x29, x30, [sp], #32
// 28: ret
// .child:
// 29: adr x5, sh_path
// 30: adr x6, dash_c
// 31: sub sp, sp, #32
// 32: str x5, [sp]                ; argv[0]
// 33: str x6, [sp, #8]            ; argv[1]
// 34: str x19, [sp, #16]          ; argv[2] = cmd
// 35: str xzr, [sp, #24]          ; argv[3] = NULL
// 36: mov x0, x5                  ; filename
// 37: mov x1, sp                  ; argv
// 38: adr x9, data_envp
// 39: ldr x2, [x9]                ; envp
// 40: mov x8, #221                ; __NR_execve
// 41: svc #0
// 42: mov x0, #127                ; exit 127
// 43: mov x8, #94                 ; __NR_exit_group
// 44: svc #0
//
// 45 instructions = 180 bytes

// Revised offsets:
// 0x000: _start (10 instrs, 40 bytes) -> ends at 0x028
// 0x028: __kr_exit (2 instrs, 8 bytes) -> ends at 0x030
// 0x030: __kr_write (3 instrs, 12 bytes) -> ends at 0x03C
// 0x03C: __kr_mmap_alloc (9 instrs, 36 bytes) -> ends at 0x060
// 0x060: __kr_alloc (25 instrs, 100 bytes) -> ends at 0x0C4
// 0x0C4: __kr_dealloc (1 instr, 4 bytes) -> ends at 0x0C8
// 0x0C8: __kr_getenv (23 instrs, 92 bytes) -> ends at 0x124
// 0x124: __kr_exec (45 instrs, 180 bytes) -> ends at 0x1D8
// 0x1D8: __kr_str_copy (8 instrs, 32 bytes) -> ends at 0x1F8
// 0x1F8: __kr_str_cat (12 instrs, 48 bytes) -> ends at 0x228
// 0x228: __kr_str_len (7 instrs, 28 bytes) -> ends at 0x244
// 0x244: sh_path "/bin/sh\0" (8 bytes) -> ends at 0x24C
// 0x24C: dash_c "-c\0" (3 bytes) -> ends at 0x24F
// 0x24F: padding (1 byte) -> ends at 0x250
// 0x250: data_area (24 bytes) -> ends at 0x268
// Total: 616 bytes

// Now I can compute all ADR offsets.
// ADR Xd, target: offset = target_addr - instr_addr

// For _start:
// Instruction at 0x18 (adr x9, data_envp): offset = 0x250 - 0x18 = 0x238
// Instruction at 0x20 (bl main): placeholder, fixup offset = 0x20
// Instruction at 0x24 (bl __kr_exit): offset = 0x28 - 0x24 = 4, imm26 = 1

// For __kr_alloc:
// 0x68: adr x9, data+16 (heap_remaining at 0x260): offset = 0x260 - 0x68 = 0x1F8
// 0x84: adr x9, data+8 (heap_ptr at 0x258): offset = 0x258 - 0x84 = 0x1D4
// 0x90: adr x9, data+16 (heap_remaining at 0x260): offset = 0x260 - 0x90 = 0x1D0
// 0x9C: adr x9, data+8 (heap_ptr at 0x258): offset = 0x258 - 0x9C = 0x1BC
// 0xAC: adr x9, data+16 (heap_remaining at 0x260): offset = 0x260 - 0xAC = 0x1B4
// 0x80: bl __kr_mmap_alloc (at 0x3C): offset = 0x3C - 0x80 = -0x44

// For __kr_getenv:
// 0xC8: adr x9, data_envp (0x250): offset = 0x250 - 0xC8 = 0x188

// For __kr_exec:
// Instruction at 0x124 + 29*4 = 0x124 + 0x74 = 0x198: adr x5, sh_path (0x244)
//   offset = 0x244 - 0x198 = 0xAC
// Instruction at 0x124 + 30*4 = 0x19C: adr x6, dash_c (0x24C)
//   offset = 0x24C - 0x19C = 0xB0
// Instruction at 0x124 + 38*4 = 0x124 + 0x98 = 0x1BC: adr x9, data_envp (0x250)
//   offset = 0x250 - 0x1BC = 0x94

// CBZ offsets in __kr_exec:
// Instruction 10 at 0x124 + 10*4 = 0x14C: cbz x0, .child
//   .child is at instruction 29, offset 0x124 + 29*4 = 0x198
//   branch offset = 0x198 - 0x14C = 0x4C = 76 bytes

// B.NE in exec instruction 22 at 0x124 + 22*4 = 0x17C: b.ne .signal_death
//   .signal_death is instruction 25 at 0x124 + 25*4 = 0x188
//   offset = 0x188 - 0x17C = 0x0C = 12

// B in exec instruction 24 at 0x124 + 24*4 = 0x184: b .done
//   .done is instruction 26 at 0x124 + 26*4 = 0x18C
//   offset = 0x18C - 0x184 = 0x08 = 8

// TST W0, #0xFF at instruction 21 (offset 0x178):
// TST Wn, #imm = ANDS WZR, Wn, #imm
// For #0xFF: N=0, immr=0, imms=000111 (7)
// 32-bit ANDS: 0x72000000 | (N << 22) | (immr << 16) | (imms << 10) | (Rn << 5) | Rd(=WZR=31)
// = 0x72000000 | (0) | (0) | (7 << 10) | (0 << 5) | 31
// = 0x72001C1F

// UBFX W0, W0, #8, #8 = UBFM W0, W0, #8, #15
// 32-bit UBFM: 0x53000000 | (immr << 16) | (imms << 10) | (Rn << 5) | Rd
// = 0x53000000 | (8 << 16) | (15 << 10) | (0 << 5) | 0
// = 0x53083C00

// MOV W0, #1 = MOVZ W0, #1
// 32-bit MOVZ: 0x52800000 | (imm16 << 5) | Rd
// = 0x52800000 | (1 << 5) | 0 = 0x52800020

// MOV W0, #127 = MOVZ W0, #127
// = 0x52800000 | (127 << 5) | 0 = 0x52800FE0

// LDR W0, [SP] (32-bit load, unsigned offset 0):
// = 0xB9400000 | (SP << 5) | 0 = 0xB94003E0

// B.LS at __kr_alloc instruction 6 (offset 0x74): b.ls .have_space
//   .have_space is at instruction 16 (offset 0x9C)
//   branch offset = 0x9C - 0x74 = 0x28 = 40
//   B.LS: cond=9, imm19 = 40/4 = 10
//   = 0x54000000 | (10 << 5) | 9 = 0x54000149

// CBNZ W1, offset in getenv: I actually used CBZ/CBNZ for X registers.
// In getenv, cbz x2 at 0xD4 branching to .not_found at 0x11C:
//   offset = 0x11C - 0xD4 = 0x48 = 72, imm19 = 72/4 = 18
//   CBZ X2: 0xB4000000 | (18 << 5) | 2 = 0xB4000242

// cbz x5 at 0xE4 branching to .check_eq at 0x100:
//   offset = 0x100 - 0xE4 = 0x1C = 28, imm19 = 7
//   CBZ X5: 0xB4000000 | (7 << 5) | 5 = 0xB40000E5

// b.ne at 0xF0 branching to .next at 0x114:
//   offset = 0x114 - 0xF0 = 0x24 = 36, imm19 = 9
//   B.NE: 0x54000000 | (9 << 5) | 1 = 0x54000121

// b at 0xFC branching to .cmp at 0xE0:
//   offset = 0xE0 - 0xFC = -0x1C = -28, imm26 = (-28/4) & 0x03FFFFFF = (-7) & 0x03FFFFFF = 0x03FFFFF9
//   B: 0x14000000 | 0x03FFFFF9 = 0x17FFFFF9

// b.ne at 0x108 branching to .next at 0x114:
//   offset = 0x114 - 0x108 = 0x0C = 12, imm19 = 3
//   B.NE: 0x54000000 | (3 << 5) | 1 = 0x54000061

// b at 0x118 branching to .env_loop at 0xD0:
//   offset = 0xD0 - 0x118 = -0x48 = -72, imm26 = (-72/4) & 0x03FFFFFF = (-18) & 0x03FFFFFF = 0x03FFFFEE
//   B: 0x14000000 | 0x03FFFFEE = 0x17FFFFEE

// Now let me also verify the _start instructions:
// 0x00: mov x29, #0 = movz x29, #0 = 0xD2800000 | 29 = 0xD280001D
// 0x04: mov x30, #0 = movz x30, #0 = 0xD280001E
// 0x08: ldr x0, [sp] = ldr x0, [sp, #0] = 0xF94003E0
// 0x0C: add x1, sp, #8 = 0x91000000 | (8 << 10) | (31 << 5) | 1 = 0x910023E1
// 0x10: add x2, x0, #1 = 0x91000000 | (1 << 10) | (0 << 5) | 2 = 0x91000402
// 0x14: add x2, x1, x2, lsl #3 = add_reg_lsl(2, 1, 2, 3)
//   = 0x8B000000 | (2 << 16) | (3 << 10) | (1 << 5) | 2
//   = 0x8B020C22
// 0x18: adr x9, data_envp: offset = 0x250 - 0x18 = 0x238 = 568
//   immlo = 568 & 3 = 0, immhi = 568 >> 2 = 142
//   = (0 << 29) | 0x10000000 | (142 << 5) | 9 = 0x10000000 | 0x11C0 | 9 = 0x100011C9
// 0x1C: str x2, [x9] = 0xF9000000 | (9 << 5) | 2 = 0xF9000122
// 0x20: bl main (placeholder, imm26=0) = 0x94000000
// 0x24: bl __kr_exit: offset = 0x28 - 0x24 = 4, imm26 = 1
//   = 0x94000001

// __kr_exit:
// 0x28: mov x8, #94 = movz x8, #94 = 0xD2800000 | (94 << 5) | 8 = 0xD2800BC8
// 0x2C: svc #0 = 0xD4000001

// __kr_write:
// 0x30: mov x8, #64 = 0xD2800000 | (64 << 5) | 8 = 0xD2800808
// 0x34: svc #0 = 0xD4000001
// 0x38: ret = 0xD65F03C0

// __kr_mmap_alloc:
// 0x3C: mov x1, x0 = 0xAA0003E0 | (0 << 16) | 1 = 0xAA0003E1
// 0x40: mov x0, #0 = 0xD2800000
// 0x44: mov x2, #3 = 0xD2800000 | (3 << 5) | 2 = 0xD2800062
// 0x48: mov x3, #0x22 = 0xD2800000 | (0x22 << 5) | 3 = 0xD2800443
// 0x4C: movn x4, #0 = 0x92800000 | 4 = 0x92800004 (this gives x4 = ~0 = -1)
// 0x50: mov x5, #0 = 0xD2800000 | 5 = 0xD2800005
// 0x54: mov x8, #222 = 0xD2800000 | (222 << 5) | 8 = 0xD2801BC8
// 0x58: svc #0 = 0xD4000001
// 0x5C: ret = 0xD65F03C0

// __kr_alloc:
// 0x60: add x0, x0, #15 = 0x91000000 | (15 << 10) | (0 << 5) | 0 = 0x91003C00
// 0x64: and x0, x0, #~15 = and_imm_align16(0, 0) = 0x9240F000
// 0x68: adr x9, data+16: offset = 0x260 - 0x68 = 0x1F8 = 504
//   immlo = 504 & 3 = 0, immhi = 504 >> 2 = 126
//   = 0x10000000 | (126 << 5) | 9 = 0x10000FC9
// 0x6C: ldr x1, [x9] = 0xF9400000 | (9 << 5) | 1 = 0xF9400121
// 0x70: cmp x0, x1 = subs xzr, x0, x1 = 0xEB00001F | (1 << 16) | (0 << 5) = 0xEB01001F
// 0x74: b.ls .have_space (0x9C): offset = 0x9C - 0x74 = 40, imm19 = 10
//   = 0x54000000 | (10 << 5) | 9 = 0x54000149
// 0x78: stp x0, x30, [sp, #-16]! :
//   Pre-index STP: opc=10, V=0, type=pre(11), imm7=-16/8=-2, 7-bit signed = 0x7E
//   = 0xA9800000 | (0x7E << 15) | (30 << 10) | (31 << 5) | 0
//   = 0xA9800000 | 0x003F0000 | 0x7800 | 0x3E0 | 0
//   = 0xA9BF7BE0
// 0x7C: movz x0, #0x40, lsl #16 = 0xD2A00000 | (0x40 << 5) | 0 = 0xD2A00800
// 0x80: bl __kr_mmap_alloc: offset = 0x3C - 0x80 = -68, imm26 = (-68/4) & 0x03FFFFFF
//   = (-17) & 0x03FFFFFF = 0x03FFFFEF
//   = 0x94000000 | 0x03FFFFEF = 0x97FFFFEF
// 0x84: adr x9, data+8: offset = 0x258 - 0x84 = 0x1D4 = 468
//   immlo = 468 & 3 = 0, immhi = 468 >> 2 = 117
//   = 0x10000000 | (117 << 5) | 9 = 0x10000EA9
// 0x88: str x0, [x9] = 0xF9000000 | (9 << 5) | 0 = 0xF9000120
// 0x8C: movz x1, #0x40, lsl #16 = 0xD2A00000 | (0x40 << 5) | 1 = 0xD2A00801
// 0x90: adr x9, data+16: offset = 0x260 - 0x90 = 0x1D0 = 464
//   immlo = 464 & 3 = 0, immhi = 464 >> 2 = 116
//   = 0x10000000 | (116 << 5) | 9 = 0x10000E89
// 0x94: str x1, [x9] = 0xF9000000 | (9 << 5) | 1 = 0xF9000121
// 0x98: ldp x0, x30, [sp], #16 :
//   Post-index LDP: opc=10, V=0, type=post(01), imm7=16/8=2
//   = 0xA8C00000 | (2 << 15) | (30 << 10) | (31 << 5) | 0
//   = 0xA8C00000 | 0x10000 | 0x7800 | 0x3E0 | 0
//   = 0xA8C17BE0
// .have_space:
// 0x9C: adr x9, data+8: offset = 0x258 - 0x9C = 0x1BC = 444
//   immlo = 444 & 3 = 0, immhi = 444 >> 2 = 111
//   = 0x10000000 | (111 << 5) | 9 = 0x10000DE9
// 0xA0: ldr x1, [x9] = 0xF9400121
// 0xA4: add x2, x1, x0 = add_reg_lsl(2, 1, 0, 0) = 0x8B000000 | (0 << 16) | (0 << 10) | (1 << 5) | 2
//   = 0x8B000022
// 0xA8: str x2, [x9] = 0xF9000000 | (9 << 5) | 2 = 0xF9000122
// 0xAC: adr x9, data+16: offset = 0x260 - 0xAC = 0x1B4 = 436
//   immlo = 436 & 3 = 0, immhi = 436 >> 2 = 109
//   = 0x10000000 | (109 << 5) | 9 = 0x10000DA9
// 0xB0: ldr x2, [x9] = 0xF9400000 | (9 << 5) | 2 = 0xF9400122
// 0xB4: sub x2, x2, x0 = 0xCB000000 | (0 << 16) | (2 << 5) | 2 = 0xCB000042
// 0xB8: str x2, [x9] = 0xF9000122
// 0xBC: mov x0, x1 = mov_reg(0, 1) = 0xAA0003E0 | (1 << 16) | 0 = 0xAA0103E0
// 0xC0: ret = 0xD65F03C0

// __kr_dealloc:
// 0xC4: ret = 0xD65F03C0

// __kr_getenv:
// 0xC8: adr x9, data_envp (0x250): offset = 0x250 - 0xC8 = 0x188 = 392
//   immlo = 392 & 3 = 0, immhi = 392 >> 2 = 98
//   = 0x10000000 | (98 << 5) | 9 = 0x10000C49
// 0xCC: ldr x1, [x9] = 0xF9400121
// .env_loop:
// 0xD0: ldr x2, [x1] = 0xF9400000 | (1 << 5) | 2 = 0xF9400022
// 0xD4: cbz x2, .not_found (0x11C): offset = 0x11C - 0xD4 = 0x48 = 72
//   imm19 = 72/4 = 18 = 0xB4000000 | (18 << 5) | 2 = 0xB4000242
// 0xD8: mov x3, x0 = mov_reg(3, 0) = 0xAA0003E0 | (0 << 16) | 3 = 0xAA0003E3
// 0xDC: mov x4, x2 = mov_reg(4, 2) = 0xAA0003E0 | (2 << 16) | 4 = 0xAA0203E4
// .cmp:
// 0xE0: ldrb w5, [x3] = 0x39400000 | (3 << 5) | 5 = 0x39400065
// 0xE4: cbz x5, .check_eq (0x100): offset = 0x100 - 0xE4 = 0x1C = 28
//   imm19 = 7 = 0xB4000000 | (7 << 5) | 5 = 0xB40000E5
// 0xE8: ldrb w6, [x4] = 0x39400000 | (4 << 5) | 6 = 0x39400086
// 0xEC: cmp x5, x6 = subs xzr, x5, x6 = 0xEB00001F | (6 << 16) | (5 << 5)
//   = 0xEB06001F | (5 << 5) = 0xEB0600BF
// 0xF0: b.ne .next (0x114): offset = 0x114 - 0xF0 = 0x24 = 36, imm19 = 9
//   = 0x54000000 | (9 << 5) | 1 = 0x54000121
// 0xF4: add x3, x3, #1 = 0x91000000 | (1 << 10) | (3 << 5) | 3 = 0x91000463
// 0xF8: add x4, x4, #1 = 0x91000000 | (1 << 10) | (4 << 5) | 4 = 0x91000484
// 0xFC: b .cmp (0xE0): offset = 0xE0 - 0xFC = -28, imm26 = (-7) & 0x03FFFFFF = 0x03FFFFF9
//   = 0x14000000 | 0x03FFFFF9 = 0x17FFFFF9
// .check_eq:
// 0x100: ldrb w5, [x4] = 0x39400085
// 0x104: cmp x5, #0x3d = cmp_imm(5, 0x3d) = 0xF100001F | (0x3d << 10) | (5 << 5)
//   = 0xF100F4BF
// Wait, that's CMP X5, #0x3d. But x5 is only a byte value (0-255). CMP with 64-bit
// register is fine, since ldrb zero-extends.
// 0x108: b.ne .next (0x114): offset = 0x114 - 0x108 = 0x0C = 12, imm19 = 3
//   = 0x54000000 | (3 << 5) | 1 = 0x54000061
// 0x10C: add x0, x4, #1 = 0x91000000 | (1 << 10) | (4 << 5) | 0 = 0x91000480
// 0x110: ret = 0xD65F03C0
// .next:
// 0x114: add x1, x1, #8 = 0x91000000 | (8 << 10) | (1 << 5) | 1 = 0x91002021
// 0x118: b .env_loop (0xD0): offset = 0xD0 - 0x118 = -72, imm26 = (-18) & 0x03FFFFFF = 0x03FFFFEE
//   = 0x14000000 | 0x03FFFFEE = 0x17FFFFEE
// .not_found:
// 0x11C: mov x0, #0 = 0xD2800000
// 0x120: ret = 0xD65F03C0
// 23 instructions, 0xC8 to 0x120 inclusive = 92 bytes. Ends at 0x124. Good.

// __kr_exec at 0x124:
// 0x124: stp x29, x30, [sp, #-32]! :
//   imm7 = -32/8 = -4, 7-bit = 0x7C
//   = 0xA9800000 | (0x7C << 15) | (30 << 10) | (31 << 5) | 29
//   0x7C << 15 = 0x3E0000
//   30 << 10 = 0x7800
//   31 << 5 = 0x3E0
//   = 0xA9800000 | 0x3E0000 | 0x7800 | 0x3E0 | 29 = 0xA9BE7BFD
// 0x128: stp x19, x20, [sp, #16] :
//   Signed offset STP: 0xA9000000 | (imm7 << 15) | (Xt2 << 10) | (Xn << 5) | Xt1
//   imm7 = 16/8 = 2
//   = 0xA9000000 | (2 << 15) | (20 << 10) | (31 << 5) | 19
//   2 << 15 = 0x10000
//   20 << 10 = 0x5000
//   = 0xA9000000 | 0x10000 | 0x5000 | 0x3E0 | 19 = 0xA90153F3
// 0x12C: mov x19, x0 = mov_reg(19, 0) = 0xAA0003E0 | (0 << 16) | 19 = 0xAA0003F3
// 0x130: mov x0, #17 = movz(0, 17) = 0xD2800000 | (17 << 5) | 0 = 0xD2800220
// 0x134: mov x1, #0 = movz(1, 0) = 0xD2800001
// 0x138: mov x2, #0 = 0xD2800002
// 0x13C: mov x3, #0 = 0xD2800003
// 0x140: mov x4, #0 = 0xD2800004
// 0x144: mov x8, #220 = 0xD2800000 | (220 << 5) | 8 = 0xD2801B88
// 0x148: svc #0 = 0xD4000001
// 0x14C: cbz x0, .child: .child is at instruction 29 relative to start of exec
//   exec starts at 0x124, instruction 29 = 0x124 + 29*4 = 0x124 + 0x74 = 0x198
//   offset = 0x198 - 0x14C = 0x4C = 76, imm19 = 19
//   = 0xB4000000 | (19 << 5) | 0 = 0xB4000260
// 0x150: mov x20, x0 = mov_reg(20, 0) = 0xAA0003E0 | (0 << 16) | 20 = 0xAA0003F4
// 0x154: sub sp, sp, #16 = sub_imm(31, 31, 16) = 0xD1000000 | (16 << 10) | (31 << 5) | 31
//   = 0xD10043FF
// 0x158: mov x0, x20 = mov_reg(0, 20) = 0xAA0003E0 | (20 << 16) | 0 = 0xAA1403E0
// 0x15C: mov x1, sp = add_imm(1, 31, 0) -- actually MOV Xd, SP is ADD Xd, SP, #0
//   = 0x910003E1
// 0x160: mov x2, #0 = 0xD2800002
// 0x164: mov x3, #0 = 0xD2800003
// 0x168: mov x8, #260 = 0xD2800000 | (260 << 5) | 8 = 0xD2802088
// 0x16C: svc #0 = 0xD4000001
// 0x170: ldr w0, [sp] = 0xB94003E0
// 0x174: add sp, sp, #16 = add_imm(31, 31, 16) = 0x910043FF
// 0x178: tst w0, #0xFF = 0x72001C1F
//   Wait, let me recalculate: ANDS Wd, Wn, #imm
//   For 32-bit: sf=0: 0x72000000
//   N=0, immr=0b000000, imms=0b000111
//   = 0x72000000 | (0 << 22) | (0 << 16) | (7 << 10) | (0 << 5) | 31
//   = 0x72001C1F  ← correct (TST is ANDS with Rd=WZR=31)
// 0x17C: b.ne .signal_death: .signal_death at instruction 25 = 0x124 + 25*4 = 0x188
//   offset = 0x188 - 0x17C = 0x0C = 12, imm19 = 3
//   = 0x54000000 | (3 << 5) | 1 = 0x54000061
// 0x180: ubfx w0, w0, #8, #8 = ubfm w0, w0, #8, #15
//   = 0x53000000 | (8 << 16) | (15 << 10) | (0 << 5) | 0 = 0x53083C00
// 0x184: b .done: .done at instruction 26 = 0x124 + 26*4 = 0x18C
//   offset = 0x18C - 0x184 = 8, imm26 = 2
//   = 0x14000000 | 2 = 0x14000002
// .signal_death:
// 0x188: mov w0, #1 = 0x52800020
// .done:
// 0x18C: ldp x19, x20, [sp, #16] :
//   = 0xA9400000 | (2 << 15) | (20 << 10) | (31 << 5) | 19
//   = 0xA9400000 | 0x10000 | 0x5000 | 0x3E0 | 19 = 0xA94153F3
// 0x190: ldp x29, x30, [sp], #32 :
//   Post-index: 0xA8C00000 | (4 << 15) | (30 << 10) | (31 << 5) | 29
//   = 0xA8C00000 | 0x20000 | 0x7800 | 0x3E0 | 29 = 0xA8C27BFD
// 0x194: ret = 0xD65F03C0
// .child:
// 0x198: adr x5, sh_path (0x244): offset = 0x244 - 0x198 = 0xAC = 172
//   immlo = 172 & 3 = 0, immhi = 172 >> 2 = 43
//   = 0x10000000 | (43 << 5) | 5 = 0x10000565
// 0x19C: adr x6, dash_c (0x24C): offset = 0x24C - 0x19C = 0xB0 = 176
//   immlo = 176 & 3 = 0, immhi = 176 >> 2 = 44
//   = 0x10000000 | (44 << 5) | 6 = 0x10000586
// 0x1A0: sub sp, sp, #32 = 0xD10083FF
// 0x1A4: str x5, [sp] = str_off(5, 31, 0) = 0xF90003E5
// 0x1A8: str x6, [sp, #8] = str_off(6, 31, 8) = 0xF9000000 | ((8/8) << 10) | (31 << 5) | 6
//   = 0xF9000000 | (1 << 10) | 0x3E0 | 6 = 0xF90007E6
// 0x1AC: str x19, [sp, #16] = 0xF9000000 | ((16/8) << 10) | (31 << 5) | 19
//   = 0xF9000000 | (2 << 10) | 0x3E0 | 19 = 0xF9000BF3
// 0x1B0: str xzr, [sp, #24] = 0xF9000000 | ((24/8) << 10) | (31 << 5) | 31
//   = 0xF9000000 | (3 << 10) | 0x3E0 | 31 = 0xF9000FFF
// 0x1B4: mov x0, x5 = mov_reg(0, 5) = 0xAA0503E0
// 0x1B8: mov x1, sp = 0x910003E1
// 0x1BC: adr x9, data_envp (0x250): offset = 0x250 - 0x1BC = 0x94 = 148
//   immlo = 148 & 3 = 0, immhi = 148 >> 2 = 37
//   = 0x10000000 | (37 << 5) | 9 = 0x100004A9
// 0x1C0: ldr x2, [x9] = 0xF9400122
// 0x1C4: mov x8, #221 = 0xD2800000 | (221 << 5) | 8 = 0xD2801BA8
// 0x1C8: svc #0 = 0xD4000001
// 0x1CC: mov x0, #127 = movz(0, 127) = 0xD2800000 | (127 << 5) | 0 = 0xD2800FE0
// 0x1D0: mov x8, #94 = 0xD2800BC8
// 0x1D4: svc #0 = 0xD4000001
//
// exec: 0x124 to 0x1D4+3 = 0x1D7. That's 45 instructions = 180 bytes.
// 0x124 + 180 = 0x1D8. Good.

// __kr_str_copy at 0x1D8:
// 0x1D8: mov x2, x0 = mov_reg(2, 0) = 0xAA0003E2 (save dst for return)
// .copy_loop:
// 0x1DC: ldrb w3, [x1] = 0x39400023
// 0x1E0: strb w3, [x0] = 0x39000003
// 0x1E4: cbz w3, .copy_done: need 32-bit CBZ = 0x34000000
//   .copy_done at 0x1F4, offset = 0x1F4 - 0x1E4 = 0x10 = 16, imm19 = 4
//   = 0x34000000 | (4 << 5) | 3 = 0x34000083
// 0x1E8: add x0, x0, #1 = 0x91000400
// 0x1EC: add x1, x1, #1 = 0x91000421
// 0x1F0: b .copy_loop (0x1DC): offset = 0x1DC - 0x1F0 = -20, imm26 = (-5) & 0x03FFFFFF = 0x03FFFFFB
//   = 0x14000000 | 0x03FFFFFB = 0x17FFFFFB
// .copy_done:
// 0x1F4: mov x0, x2 = mov_reg(0, 2) = 0xAA0203E0
// 0x1F8: ret = 0xD65F03C0
// 9 instructions, 36 bytes. 0x1D8 + 36 = 0x1FC

// Hmm wait, that's 9 instructions now, not 8. Let me adjust.
// Actually the reference x86_64 returns dst at the start (mov rax, rdi) and the return
// value is already in rax when it returns. In AArch64 I can do the same:
// save x0 to x2 first, then at end move x2 back to x0.
// Or: just keep x0 as the return value by using a different register for the pointer.
//
// x0 = dst, x1 = src
// Save return value (original dst) first, then use x0 for iteration.
// Or better: use separate register for the running pointer.
//
// 0x1D8: mov x2, x0              ; save original dst
// 0x1DC: ldrb w3, [x1]           ; load src byte
// 0x1E0: strb w3, [x0]           ; store to dst
// 0x1E4: cbz w3, .done           ; if NUL, done
// 0x1E8: add x0, x0, #1
// 0x1EC: add x1, x1, #1
// 0x1F0: b .copy_loop (0x1DC)
// .done:
// 0x1F4: mov x0, x2              ; return original dst
// 0x1F8: ret
// 9 instrs, 36 bytes. 0x1D8 + 36 = 0x1FC.

// Recalculate downstream offsets:
// __kr_str_copy: 0x1D8, 9 instrs, 36 bytes -> ends 0x1FC
// __kr_str_cat: 0x1FC
// __kr_str_len: ?

// __kr_str_cat at 0x1FC:
// x0 = dst, x1 = src. Return dst (original x0).
// 0x1FC: mov x2, x0              ; save original dst
// .find_end:
// 0x200: ldrb w3, [x0]
// 0x204: cbz w3, .cat_copy       ; -> 0x210, offset = 0x210 - 0x204 = 12, imm19 = 3
//   = 0x34000000 | (3 << 5) | 3 = 0x34000063
// 0x208: add x0, x0, #1
// 0x20C: b .find_end (0x200): offset = 0x200 - 0x20C = -12, imm26 = (-3) & 0x03FFFFFF = 0x03FFFFFD
//   = 0x14000000 | 0x03FFFFFD = 0x17FFFFFD
// .cat_copy:
// 0x210: ldrb w3, [x1]
// 0x214: strb w3, [x0]
// 0x218: cbz w3, .cat_done       ; -> 0x228, offset = 0x228 - 0x218 = 16, imm19 = 4
//   = 0x34000000 | (4 << 5) | 3 = 0x34000083
// 0x21C: add x0, x0, #1
// 0x220: add x1, x1, #1
// 0x224: b .cat_copy (0x210): offset = 0x210 - 0x224 = -20, imm26 = (-5) & 0x03FFFFFF = 0x03FFFFFB
//   = 0x17FFFFFB
// .cat_done:
// 0x228: mov x0, x2 = 0xAA0203E0
// 0x22C: ret = 0xD65F03C0
// 13 instrs, 52 bytes. 0x1FC + 52 = 0x230.

// __kr_str_len at 0x230:
// x0 = str. Return length in x0.
// 0x230: mov x1, #0 = 0xD2800001 (counter)
// .len_loop:
// 0x234: ldrb w2, [x0]
// 0x238: cbz w2, .len_done       ; -> 0x248, offset = 0x248 - 0x238 = 16, imm19 = 4
//   = 0x34000000 | (4 << 5) | 2 = 0x34000082
// 0x23C: add x0, x0, #1
// 0x240: add x1, x1, #1
// 0x244: b .len_loop (0x234): offset = 0x234 - 0x244 = -16, imm26 = (-4) & 0x03FFFFFF = 0x03FFFFFC
//   = 0x17FFFFFC
// .len_done:
// 0x248: mov x0, x1 = mov_reg(0, 1) = 0xAA0103E0
// 0x24C: ret = 0xD65F03C0
// 8 instrs, 32 bytes. 0x230 + 32 = 0x250.

// Strings at 0x250:
// sh_path: "/bin/sh\0" = 8 bytes (0x250-0x257)
// dash_c: "-c\0" = 3 bytes (0x258-0x25A)
// padding: 5 bytes (0x25B-0x25F) to align to 0x260

// Data area at 0x260:
// envp (8 bytes): 0x260-0x267
// heap_ptr (8 bytes): 0x268-0x26F
// heap_remaining (8 bytes): 0x270-0x277
// Total: 0x278 = 632 bytes

// Now I need to RECALCULATE all ADR offsets because the string/data positions changed!
// sh_path = 0x250
// dash_c = 0x258
// data_envp = 0x260
// data_heap_ptr = 0x268
// data_heap_remaining = 0x270

// Recalculate ADR offsets:
// _start 0x18: adr x9, 0x260: offset = 0x260 - 0x18 = 0x248 = 584
//   immlo = 584 & 3 = 0, immhi = 584 >> 2 = 146
//   = 0x10000000 | (146 << 5) | 9 = 0x10001249

// __kr_alloc:
// 0x68: adr x9, 0x270: offset = 0x270 - 0x68 = 0x208 = 520
//   immlo = 520 & 3 = 0, immhi = 520 >> 2 = 130
//   = 0x10000000 | (130 << 5) | 9 = 0x10001049
// 0x84: adr x9, 0x268: offset = 0x268 - 0x84 = 0x1E4 = 484
//   immlo = 484 & 3 = 0, immhi = 484 >> 2 = 121
//   = 0x10000000 | (121 << 5) | 9 = 0x10000F29
// 0x90: adr x9, 0x270: offset = 0x270 - 0x90 = 0x1E0 = 480
//   immlo = 480 & 3 = 0, immhi = 480 >> 2 = 120
//   = 0x10000000 | (120 << 5) | 9 = 0x10000F09
// 0x9C: adr x9, 0x268: offset = 0x268 - 0x9C = 0x1CC = 460
//   immlo = 460 & 3 = 0, immhi = 460 >> 2 = 115
//   = 0x10000000 | (115 << 5) | 9 = 0x10000E69
// 0xAC: adr x9, 0x270: offset = 0x270 - 0xAC = 0x1C4 = 452
//   immlo = 452 & 3 = 0, immhi = 452 >> 2 = 113
//   = 0x10000000 | (113 << 5) | 9 = 0x10000E29

// __kr_getenv:
// 0xC8: adr x9, 0x260: offset = 0x260 - 0xC8 = 0x198 = 408
//   immlo = 408 & 3 = 0, immhi = 408 >> 2 = 102
//   = 0x10000000 | (102 << 5) | 9 = 0x10000CC9

// __kr_exec:
// 0x198: adr x5, 0x250: offset = 0x250 - 0x198 = 0xB8 = 184
//   immlo = 184 & 3 = 0, immhi = 184 >> 2 = 46
//   = 0x10000000 | (46 << 5) | 5 = 0x100005C5
// 0x19C: adr x6, 0x258: offset = 0x258 - 0x19C = 0xBC = 188
//   immlo = 188 & 3 = 0, immhi = 188 >> 2 = 47
//   = 0x10000000 | (47 << 5) | 6 = 0x100005E6
// 0x1BC: adr x9, 0x260: offset = 0x260 - 0x1BC = 0xA4 = 164
//   immlo = 164 & 3 = 0, immhi = 164 >> 2 = 41
//   = 0x10000000 | (41 << 5) | 9 = 0x10000529

// Now I also need to fix the _start bl __kr_exit:
// 0x24: bl 0x28: offset = 0x28 - 0x24 = 4, imm26 = 1
//   = 0x94000001 ← correct

// And the __kr_alloc bl __kr_mmap_alloc:
// 0x80: bl 0x3C: offset = 0x3C - 0x80 = -68 = -0x44
//   imm26 = (-68/4) & 0x03FFFFFF = (-17) & 0x03FFFFFF = 0x03FFFFEF
//   = 0x94000000 | 0x03FFFFEF = 0x97FFFFEF

// Great! Now let me write out the full blob with all bytes.

const CODE: [u8; 1072] = {
    let mut c = [0u8; 1072];

    // Helper to write a u32 at byte offset (little-endian)
    // Can't use a closure in const context, so we'll use a macro-like approach.
    // Actually in Rust const context we can use a simple function call pattern
    // within the const block.

    // _start at 0x00
    // 0x00: mov x29, #0 = 0xD280001D
    c[0x00] = 0x1D;
    c[0x01] = 0x00;
    c[0x02] = 0x80;
    c[0x03] = 0xD2;
    // 0x04: mov x30, #0 = 0xD280001E
    c[0x04] = 0x1E;
    c[0x05] = 0x00;
    c[0x06] = 0x80;
    c[0x07] = 0xD2;
    // 0x08: ldr x0, [sp] = 0xF94003E0
    c[0x08] = 0xE0;
    c[0x09] = 0x03;
    c[0x0A] = 0x40;
    c[0x0B] = 0xF9;
    // 0x0C: add x1, sp, #8 = 0x910023E1
    c[0x0C] = 0xE1;
    c[0x0D] = 0x23;
    c[0x0E] = 0x00;
    c[0x0F] = 0x91;
    // 0x10: add x2, x0, #1 = 0x91000402
    c[0x10] = 0x02;
    c[0x11] = 0x04;
    c[0x12] = 0x00;
    c[0x13] = 0x91;
    // 0x14: add x2, x1, x2, lsl #3 = 0x8B020C22
    c[0x14] = 0x22;
    c[0x15] = 0x0C;
    c[0x16] = 0x02;
    c[0x17] = 0x8B;
    // 0x18: adr x9, data_envp (0x260) = 0x10001249
    c[0x18] = 0x49;
    c[0x19] = 0x12;
    c[0x1A] = 0x00;
    c[0x1B] = 0x10;
    // 0x1C: str x2, [x9] = 0xF9000122
    c[0x1C] = 0x22;
    c[0x1D] = 0x01;
    c[0x1E] = 0x00;
    c[0x1F] = 0xF9;
    // 0x20: bl main (placeholder) = 0x94000000
    c[0x20] = 0x00;
    c[0x21] = 0x00;
    c[0x22] = 0x00;
    c[0x23] = 0x94;
    // 0x24: bl __kr_exit = 0x94000001
    c[0x24] = 0x01;
    c[0x25] = 0x00;
    c[0x26] = 0x00;
    c[0x27] = 0x94;

    // __kr_exit at 0x28
    // 0x28: mov x8, #94 = 0xD2800BC8
    c[0x28] = 0xC8;
    c[0x29] = 0x0B;
    c[0x2A] = 0x80;
    c[0x2B] = 0xD2;
    // 0x2C: svc #0 = 0xD4000001
    c[0x2C] = 0x01;
    c[0x2D] = 0x00;
    c[0x2E] = 0x00;
    c[0x2F] = 0xD4;

    // __kr_write at 0x30
    // 0x30: mov x8, #64 = 0xD2800808
    c[0x30] = 0x08;
    c[0x31] = 0x08;
    c[0x32] = 0x80;
    c[0x33] = 0xD2;
    // 0x34: svc #0 = 0xD4000001
    c[0x34] = 0x01;
    c[0x35] = 0x00;
    c[0x36] = 0x00;
    c[0x37] = 0xD4;
    // 0x38: ret = 0xD65F03C0
    c[0x38] = 0xC0;
    c[0x39] = 0x03;
    c[0x3A] = 0x5F;
    c[0x3B] = 0xD6;

    // __kr_mmap_alloc at 0x3C
    // 0x3C: mov x1, x0 = 0xAA0003E1
    c[0x3C] = 0xE1;
    c[0x3D] = 0x03;
    c[0x3E] = 0x00;
    c[0x3F] = 0xAA;
    // 0x40: mov x0, #0 = 0xD2800000
    c[0x40] = 0x00;
    c[0x41] = 0x00;
    c[0x42] = 0x80;
    c[0x43] = 0xD2;
    // 0x44: mov x2, #3 = 0xD2800062
    c[0x44] = 0x62;
    c[0x45] = 0x00;
    c[0x46] = 0x80;
    c[0x47] = 0xD2;
    // 0x48: mov x3, #0x22 = 0xD2800443
    c[0x48] = 0x43;
    c[0x49] = 0x04;
    c[0x4A] = 0x80;
    c[0x4B] = 0xD2;
    // 0x4C: movn x4, #0 (= -1) = 0x92800004
    c[0x4C] = 0x04;
    c[0x4D] = 0x00;
    c[0x4E] = 0x80;
    c[0x4F] = 0x92;
    // 0x50: mov x5, #0 = 0xD2800005
    c[0x50] = 0x05;
    c[0x51] = 0x00;
    c[0x52] = 0x80;
    c[0x53] = 0xD2;
    // 0x54: mov x8, #222 = 0xD2801BC8
    c[0x54] = 0xC8;
    c[0x55] = 0x1B;
    c[0x56] = 0x80;
    c[0x57] = 0xD2;
    // 0x58: svc #0 = 0xD4000001
    c[0x58] = 0x01;
    c[0x59] = 0x00;
    c[0x5A] = 0x00;
    c[0x5B] = 0xD4;
    // 0x5C: ret = 0xD65F03C0
    c[0x5C] = 0xC0;
    c[0x5D] = 0x03;
    c[0x5E] = 0x5F;
    c[0x5F] = 0xD6;

    // __kr_alloc at 0x60
    // 0x60: add x0, x0, #15 = 0x91003C00
    c[0x60] = 0x00;
    c[0x61] = 0x3C;
    c[0x62] = 0x00;
    c[0x63] = 0x91;
    // 0x64: and x0, x0, #0xFFFFFFFFFFFFFFF0 = 0x9240F000
    c[0x64] = 0x00;
    c[0x65] = 0xF0;
    c[0x66] = 0x40;
    c[0x67] = 0x92;
    // 0x68: adr x9, heap_remaining (0x270): offset=0x208
    //   immlo=0, immhi=130 => 0x10001049
    c[0x68] = 0x49;
    c[0x69] = 0x10;
    c[0x6A] = 0x00;
    c[0x6B] = 0x10;
    // 0x6C: ldr x1, [x9] = 0xF9400121
    c[0x6C] = 0x21;
    c[0x6D] = 0x01;
    c[0x6E] = 0x40;
    c[0x6F] = 0xF9;
    // 0x70: cmp x0, x1 = 0xEB01001F
    c[0x70] = 0x1F;
    c[0x71] = 0x00;
    c[0x72] = 0x01;
    c[0x73] = 0xEB;
    // 0x74: b.ls .have_space (0x9C): offset=40, imm19=10
    //   = 0x54000149
    c[0x74] = 0x49;
    c[0x75] = 0x01;
    c[0x76] = 0x00;
    c[0x77] = 0x54;
    // 0x78: stp x0, x30, [sp, #-16]! = 0xA9BF7BE0
    c[0x78] = 0xE0;
    c[0x79] = 0x7B;
    c[0x7A] = 0xBF;
    c[0x7B] = 0xA9;
    // 0x7C: movz x0, #0x40, lsl #16 = 0xD2A00800
    c[0x7C] = 0x00;
    c[0x7D] = 0x08;
    c[0x7E] = 0xA0;
    c[0x7F] = 0xD2;
    // 0x80: bl __kr_mmap_alloc (0x3C): offset=-68 = 0x97FFFFEF
    c[0x80] = 0xEF;
    c[0x81] = 0xFF;
    c[0x82] = 0xFF;
    c[0x83] = 0x97;
    // 0x84: adr x9, heap_ptr (0x268): offset=0x1E4=484
    //   immlo=0, immhi=121 => 0x10000F29
    c[0x84] = 0x29;
    c[0x85] = 0x0F;
    c[0x86] = 0x00;
    c[0x87] = 0x10;
    // 0x88: str x0, [x9] = 0xF9000120
    c[0x88] = 0x20;
    c[0x89] = 0x01;
    c[0x8A] = 0x00;
    c[0x8B] = 0xF9;
    // 0x8C: movz x1, #0x40, lsl #16 = 0xD2A00801
    c[0x8C] = 0x01;
    c[0x8D] = 0x08;
    c[0x8E] = 0xA0;
    c[0x8F] = 0xD2;
    // 0x90: adr x9, heap_remaining (0x270): offset=0x1E0=480
    //   immlo=0, immhi=120 => 0x10000F09
    c[0x90] = 0x09;
    c[0x91] = 0x0F;
    c[0x92] = 0x00;
    c[0x93] = 0x10;
    // 0x94: str x1, [x9] = 0xF9000121
    c[0x94] = 0x21;
    c[0x95] = 0x01;
    c[0x96] = 0x00;
    c[0x97] = 0xF9;
    // 0x98: ldp x0, x30, [sp], #16 = 0xA8C17BE0
    c[0x98] = 0xE0;
    c[0x99] = 0x7B;
    c[0x9A] = 0xC1;
    c[0x9B] = 0xA8;
    // .have_space at 0x9C
    // 0x9C: adr x9, heap_ptr (0x268): offset=0x1CC=460
    //   immlo=0, immhi=115 => 0x10000E69
    c[0x9C] = 0x69;
    c[0x9D] = 0x0E;
    c[0x9E] = 0x00;
    c[0x9F] = 0x10;
    // 0xA0: ldr x1, [x9] = 0xF9400121
    c[0xA0] = 0x21;
    c[0xA1] = 0x01;
    c[0xA2] = 0x40;
    c[0xA3] = 0xF9;
    // 0xA4: add x2, x1, x0 = 0x8B000022
    c[0xA4] = 0x22;
    c[0xA5] = 0x00;
    c[0xA6] = 0x00;
    c[0xA7] = 0x8B;
    // 0xA8: str x2, [x9] = 0xF9000122
    c[0xA8] = 0x22;
    c[0xA9] = 0x01;
    c[0xAA] = 0x00;
    c[0xAB] = 0xF9;
    // 0xAC: adr x9, heap_remaining (0x270): offset=0x1C4=452
    //   immlo=0, immhi=113 => 0x10000E29
    c[0xAC] = 0x29;
    c[0xAD] = 0x0E;
    c[0xAE] = 0x00;
    c[0xAF] = 0x10;
    // 0xB0: ldr x2, [x9] = 0xF9400122
    c[0xB0] = 0x22;
    c[0xB1] = 0x01;
    c[0xB2] = 0x40;
    c[0xB3] = 0xF9;
    // 0xB4: sub x2, x2, x0 = 0xCB000042
    c[0xB4] = 0x42;
    c[0xB5] = 0x00;
    c[0xB6] = 0x00;
    c[0xB7] = 0xCB;
    // 0xB8: str x2, [x9] = 0xF9000122
    c[0xB8] = 0x22;
    c[0xB9] = 0x01;
    c[0xBA] = 0x00;
    c[0xBB] = 0xF9;
    // 0xBC: mov x0, x1 = 0xAA0103E0
    c[0xBC] = 0xE0;
    c[0xBD] = 0x03;
    c[0xBE] = 0x01;
    c[0xBF] = 0xAA;
    // 0xC0: ret = 0xD65F03C0
    c[0xC0] = 0xC0;
    c[0xC1] = 0x03;
    c[0xC2] = 0x5F;
    c[0xC3] = 0xD6;

    // __kr_dealloc at 0xC4
    // 0xC4: ret = 0xD65F03C0
    c[0xC4] = 0xC0;
    c[0xC5] = 0x03;
    c[0xC6] = 0x5F;
    c[0xC7] = 0xD6;

    // __kr_getenv at 0xC8
    // 0xC8: adr x9, data_envp (0x260): offset=0x198=408
    //   immlo=0, immhi=102 => 0x10000CC9
    c[0xC8] = 0xC9;
    c[0xC9] = 0x0C;
    c[0xCA] = 0x00;
    c[0xCB] = 0x10;
    // 0xCC: ldr x1, [x9] = 0xF9400121
    c[0xCC] = 0x21;
    c[0xCD] = 0x01;
    c[0xCE] = 0x40;
    c[0xCF] = 0xF9;
    // .env_loop at 0xD0
    // 0xD0: ldr x2, [x1] = 0xF9400022
    c[0xD0] = 0x22;
    c[0xD1] = 0x00;
    c[0xD2] = 0x40;
    c[0xD3] = 0xF9;
    // 0xD4: cbz x2, .not_found (0x11C): offset=72, imm19=18
    //   = 0xB4000242
    c[0xD4] = 0x42;
    c[0xD5] = 0x02;
    c[0xD6] = 0x00;
    c[0xD7] = 0xB4;
    // 0xD8: mov x3, x0 = 0xAA0003E3
    c[0xD8] = 0xE3;
    c[0xD9] = 0x03;
    c[0xDA] = 0x00;
    c[0xDB] = 0xAA;
    // 0xDC: mov x4, x2 = 0xAA0203E4
    c[0xDC] = 0xE4;
    c[0xDD] = 0x03;
    c[0xDE] = 0x02;
    c[0xDF] = 0xAA;
    // .cmp at 0xE0
    // 0xE0: ldrb w5, [x3] = 0x39400065
    c[0xE0] = 0x65;
    c[0xE1] = 0x00;
    c[0xE2] = 0x40;
    c[0xE3] = 0x39;
    // 0xE4: cbz x5, .check_eq (0x100): offset=28, imm19=7
    //   = 0xB40000E5
    c[0xE4] = 0xE5;
    c[0xE5] = 0x00;
    c[0xE6] = 0x00;
    c[0xE7] = 0xB4;
    // 0xE8: ldrb w6, [x4] = 0x39400086
    c[0xE8] = 0x86;
    c[0xE9] = 0x00;
    c[0xEA] = 0x40;
    c[0xEB] = 0x39;
    // 0xEC: cmp x5, x6 = 0xEB0600BF
    c[0xEC] = 0xBF;
    c[0xED] = 0x00;
    c[0xEE] = 0x06;
    c[0xEF] = 0xEB;
    // 0xF0: b.ne .next (0x114): offset=36, imm19=9
    //   = 0x54000121
    c[0xF0] = 0x21;
    c[0xF1] = 0x01;
    c[0xF2] = 0x00;
    c[0xF3] = 0x54;
    // 0xF4: add x3, x3, #1 = 0x91000463
    c[0xF4] = 0x63;
    c[0xF5] = 0x04;
    c[0xF6] = 0x00;
    c[0xF7] = 0x91;
    // 0xF8: add x4, x4, #1 = 0x91000484
    c[0xF8] = 0x84;
    c[0xF9] = 0x04;
    c[0xFA] = 0x00;
    c[0xFB] = 0x91;
    // 0xFC: b .cmp (0xE0): offset=-28 = 0x17FFFFF9
    c[0xFC] = 0xF9;
    c[0xFD] = 0xFF;
    c[0xFE] = 0xFF;
    c[0xFF] = 0x17;
    // .check_eq at 0x100
    // 0x100: ldrb w5, [x4] = 0x39400085
    c[0x100] = 0x85;
    c[0x101] = 0x00;
    c[0x102] = 0x40;
    c[0x103] = 0x39;
    // 0x104: cmp x5, #0x3d = 0xF100F4BF
    c[0x104] = 0xBF;
    c[0x105] = 0xF4;
    c[0x106] = 0x00;
    c[0x107] = 0xF1;
    // 0x108: b.ne .next (0x114): offset=12, imm19=3
    //   = 0x54000061
    c[0x108] = 0x61;
    c[0x109] = 0x00;
    c[0x10A] = 0x00;
    c[0x10B] = 0x54;
    // 0x10C: add x0, x4, #1 = 0x91000480
    c[0x10C] = 0x80;
    c[0x10D] = 0x04;
    c[0x10E] = 0x00;
    c[0x10F] = 0x91;
    // 0x110: ret = 0xD65F03C0
    c[0x110] = 0xC0;
    c[0x111] = 0x03;
    c[0x112] = 0x5F;
    c[0x113] = 0xD6;
    // .next at 0x114
    // 0x114: add x1, x1, #8 = 0x91002021
    c[0x114] = 0x21;
    c[0x115] = 0x20;
    c[0x116] = 0x00;
    c[0x117] = 0x91;
    // 0x118: b .env_loop (0xD0): offset=-72 = 0x17FFFFEE
    c[0x118] = 0xEE;
    c[0x119] = 0xFF;
    c[0x11A] = 0xFF;
    c[0x11B] = 0x17;
    // .not_found at 0x11C
    // 0x11C: mov x0, #0 = 0xD2800000
    c[0x11C] = 0x00;
    c[0x11D] = 0x00;
    c[0x11E] = 0x80;
    c[0x11F] = 0xD2;
    // 0x120: ret = 0xD65F03C0
    c[0x120] = 0xC0;
    c[0x121] = 0x03;
    c[0x122] = 0x5F;
    c[0x123] = 0xD6;

    // __kr_exec at 0x124
    // 0x124: stp x29, x30, [sp, #-32]! = 0xA9BE7BFD
    c[0x124] = 0xFD;
    c[0x125] = 0x7B;
    c[0x126] = 0xBE;
    c[0x127] = 0xA9;
    // 0x128: stp x19, x20, [sp, #16] = 0xA90153F3
    c[0x128] = 0xF3;
    c[0x129] = 0x53;
    c[0x12A] = 0x01;
    c[0x12B] = 0xA9;
    // 0x12C: mov x19, x0 = 0xAA0003F3
    c[0x12C] = 0xF3;
    c[0x12D] = 0x03;
    c[0x12E] = 0x00;
    c[0x12F] = 0xAA;
    // 0x130: mov x0, #17 (SIGCHLD) = 0xD2800220
    c[0x130] = 0x20;
    c[0x131] = 0x02;
    c[0x132] = 0x80;
    c[0x133] = 0xD2;
    // 0x134: mov x1, #0 = 0xD2800001
    c[0x134] = 0x01;
    c[0x135] = 0x00;
    c[0x136] = 0x80;
    c[0x137] = 0xD2;
    // 0x138: mov x2, #0 = 0xD2800002
    c[0x138] = 0x02;
    c[0x139] = 0x00;
    c[0x13A] = 0x80;
    c[0x13B] = 0xD2;
    // 0x13C: mov x3, #0 = 0xD2800003
    c[0x13C] = 0x03;
    c[0x13D] = 0x00;
    c[0x13E] = 0x80;
    c[0x13F] = 0xD2;
    // 0x140: mov x4, #0 = 0xD2800004
    c[0x140] = 0x04;
    c[0x141] = 0x00;
    c[0x142] = 0x80;
    c[0x143] = 0xD2;
    // 0x144: mov x8, #220 (__NR_clone) = 0xD2801B88
    c[0x144] = 0x88;
    c[0x145] = 0x1B;
    c[0x146] = 0x80;
    c[0x147] = 0xD2;
    // 0x148: svc #0 = 0xD4000001
    c[0x148] = 0x01;
    c[0x149] = 0x00;
    c[0x14A] = 0x00;
    c[0x14B] = 0xD4;
    // 0x14C: cbz x0, .child (0x198): offset=76, imm19=19
    //   = 0xB4000260
    c[0x14C] = 0x60;
    c[0x14D] = 0x02;
    c[0x14E] = 0x00;
    c[0x14F] = 0xB4;
    // Parent path:
    // 0x150: mov x20, x0 = 0xAA0003F4
    c[0x150] = 0xF4;
    c[0x151] = 0x03;
    c[0x152] = 0x00;
    c[0x153] = 0xAA;
    // 0x154: sub sp, sp, #16 = 0xD10043FF
    c[0x154] = 0xFF;
    c[0x155] = 0x43;
    c[0x156] = 0x00;
    c[0x157] = 0xD1;
    // 0x158: mov x0, x20 = 0xAA1403E0
    c[0x158] = 0xE0;
    c[0x159] = 0x03;
    c[0x15A] = 0x14;
    c[0x15B] = 0xAA;
    // 0x15C: mov x1, sp = add x1, sp, #0 = 0x910003E1
    c[0x15C] = 0xE1;
    c[0x15D] = 0x03;
    c[0x15E] = 0x00;
    c[0x15F] = 0x91;
    // 0x160: mov x2, #0 = 0xD2800002
    c[0x160] = 0x02;
    c[0x161] = 0x00;
    c[0x162] = 0x80;
    c[0x163] = 0xD2;
    // 0x164: mov x3, #0 = 0xD2800003
    c[0x164] = 0x03;
    c[0x165] = 0x00;
    c[0x166] = 0x80;
    c[0x167] = 0xD2;
    // 0x168: mov x8, #260 (__NR_wait4) = 0xD2802088
    c[0x168] = 0x88;
    c[0x169] = 0x20;
    c[0x16A] = 0x80;
    c[0x16B] = 0xD2;
    // 0x16C: svc #0 = 0xD4000001
    c[0x16C] = 0x01;
    c[0x16D] = 0x00;
    c[0x16E] = 0x00;
    c[0x16F] = 0xD4;
    // 0x170: ldr w0, [sp] = 0xB94003E0
    c[0x170] = 0xE0;
    c[0x171] = 0x03;
    c[0x172] = 0x40;
    c[0x173] = 0xB9;
    // 0x174: add sp, sp, #16 = 0x910043FF
    c[0x174] = 0xFF;
    c[0x175] = 0x43;
    c[0x176] = 0x00;
    c[0x177] = 0x91;
    // 0x178: tst w0, #0xFF = 0x72001C1F
    c[0x178] = 0x1F;
    c[0x179] = 0x1C;
    c[0x17A] = 0x00;
    c[0x17B] = 0x72;
    // 0x17C: b.ne .signal_death (0x188): offset=12, imm19=3
    //   = 0x54000061
    c[0x17C] = 0x61;
    c[0x17D] = 0x00;
    c[0x17E] = 0x00;
    c[0x17F] = 0x54;
    // 0x180: ubfx w0, w0, #8, #8 = ubfm w0, w0, #8, #15 = 0x53083C00
    c[0x180] = 0x00;
    c[0x181] = 0x3C;
    c[0x182] = 0x08;
    c[0x183] = 0x53;
    // 0x184: b .done (0x18C): offset=8, imm26=2
    //   = 0x14000002
    c[0x184] = 0x02;
    c[0x185] = 0x00;
    c[0x186] = 0x00;
    c[0x187] = 0x14;
    // .signal_death at 0x188
    // 0x188: mov w0, #1 = 0x52800020
    c[0x188] = 0x20;
    c[0x189] = 0x00;
    c[0x18A] = 0x80;
    c[0x18B] = 0x52;
    // .done at 0x18C
    // 0x18C: ldp x19, x20, [sp, #16] = 0xA94153F3
    c[0x18C] = 0xF3;
    c[0x18D] = 0x53;
    c[0x18E] = 0x41;
    c[0x18F] = 0xA9;
    // 0x190: ldp x29, x30, [sp], #32 = 0xA8C27BFD
    c[0x190] = 0xFD;
    c[0x191] = 0x7B;
    c[0x192] = 0xC2;
    c[0x193] = 0xA8;
    // 0x194: ret = 0xD65F03C0
    c[0x194] = 0xC0;
    c[0x195] = 0x03;
    c[0x196] = 0x5F;
    c[0x197] = 0xD6;
    // .child at 0x198
    // 0x198: adr x5, sh_path (0x250): offset=0xB8=184
    //   immlo=0, immhi=46 => 0x100005C5
    c[0x198] = 0xC5;
    c[0x199] = 0x05;
    c[0x19A] = 0x00;
    c[0x19B] = 0x10;
    // 0x19C: adr x6, dash_c (0x258): offset=0xBC=188
    //   immlo=0, immhi=47 => 0x100005E6
    c[0x19C] = 0xE6;
    c[0x19D] = 0x05;
    c[0x1A0] = 0x00;
    c[0x1A1] = 0x10;
    // WAIT - I made an indexing error! 0x19E and 0x19F are skipped!
    // Let me be more careful:
    c[0x19E] = 0x00;
    c[0x19F] = 0x10;
    // Hmm, that's wrong. Let me fix:
    // 0x19C: bytes are [0xE6, 0x05, 0x00, 0x10]
    // So c[0x19C]=0xE6, c[0x19D]=0x05, c[0x19E]=0x00, c[0x19F]=0x10
    // I had the wrong indices above. Let me redo from 0x19C:
    // Already set c[0x19C] = 0xE6 and c[0x19D] = 0x05 above, now fix:
    // The c[0x1A0] and c[0x1A1] I wrote above are wrong - those are for the NEXT instruction.
    // Let me just overwrite correctly:

    // Fix 0x19C-0x19F (adr x6, dash_c):
    // Already: c[0x19C] = 0xE6; c[0x19D] = 0x05;
    // Need: c[0x19E] = 0x00; c[0x19F] = 0x10;
    // These are already set above, good.

    // 0x1A0: sub sp, sp, #32 = 0xD10083FF
    c[0x1A0] = 0xFF;
    c[0x1A1] = 0x83;
    c[0x1A2] = 0x00;
    c[0x1A3] = 0xD1;
    // 0x1A4: str x5, [sp] = 0xF90003E5
    c[0x1A4] = 0xE5;
    c[0x1A5] = 0x03;
    c[0x1A6] = 0x00;
    c[0x1A7] = 0xF9;
    // 0x1A8: str x6, [sp, #8] = 0xF90007E6
    c[0x1A8] = 0xE6;
    c[0x1A9] = 0x07;
    c[0x1AA] = 0x00;
    c[0x1AB] = 0xF9;
    // 0x1AC: str x19, [sp, #16] = 0xF9000BF3
    c[0x1AC] = 0xF3;
    c[0x1AD] = 0x0B;
    c[0x1AE] = 0x00;
    c[0x1AF] = 0xF9;
    // 0x1B0: str xzr, [sp, #24] = 0xF9000FFF
    c[0x1B0] = 0xFF;
    c[0x1B1] = 0x0F;
    c[0x1B2] = 0x00;
    c[0x1B3] = 0xF9;
    // 0x1B4: mov x0, x5 = 0xAA0503E0
    c[0x1B4] = 0xE0;
    c[0x1B5] = 0x03;
    c[0x1B6] = 0x05;
    c[0x1B7] = 0xAA;
    // 0x1B8: mov x1, sp = 0x910003E1
    c[0x1B8] = 0xE1;
    c[0x1B9] = 0x03;
    c[0x1BA] = 0x00;
    c[0x1BB] = 0x91;
    // 0x1BC: adr x9, data_envp (0x260): offset=0xA4=164
    //   immlo=0, immhi=41 => 0x10000529
    c[0x1BC] = 0x29;
    c[0x1BD] = 0x05;
    c[0x1BE] = 0x00;
    c[0x1BF] = 0x10;
    // 0x1C0: ldr x2, [x9] = 0xF9400122
    c[0x1C0] = 0x22;
    c[0x1C1] = 0x01;
    c[0x1C2] = 0x40;
    c[0x1C3] = 0xF9;
    // 0x1C4: mov x8, #221 (__NR_execve) = 0xD2801BA8
    c[0x1C4] = 0xA8;
    c[0x1C5] = 0x1B;
    c[0x1C6] = 0x80;
    c[0x1C7] = 0xD2;
    // 0x1C8: svc #0 = 0xD4000001
    c[0x1C8] = 0x01;
    c[0x1C9] = 0x00;
    c[0x1CA] = 0x00;
    c[0x1CB] = 0xD4;
    // 0x1CC: mov x0, #127 = 0xD2800FE0
    c[0x1CC] = 0xE0;
    c[0x1CD] = 0x0F;
    c[0x1CE] = 0x80;
    c[0x1CF] = 0xD2;
    // 0x1D0: mov x8, #94 (__NR_exit_group) = 0xD2800BC8
    c[0x1D0] = 0xC8;
    c[0x1D1] = 0x0B;
    c[0x1D2] = 0x80;
    c[0x1D3] = 0xD2;
    // 0x1D4: svc #0 = 0xD4000001
    c[0x1D4] = 0x01;
    c[0x1D5] = 0x00;
    c[0x1D6] = 0x00;
    c[0x1D7] = 0xD4;

    // __kr_str_copy at 0x1D8
    // 0x1D8: mov x2, x0 (save dst) = 0xAA0003E2
    c[0x1D8] = 0xE2;
    c[0x1D9] = 0x03;
    c[0x1DA] = 0x00;
    c[0x1DB] = 0xAA;
    // .copy_loop at 0x1DC
    // 0x1DC: ldrb w3, [x1] = 0x39400023
    c[0x1DC] = 0x23;
    c[0x1DD] = 0x00;
    c[0x1DE] = 0x40;
    c[0x1DF] = 0x39;
    // 0x1E0: strb w3, [x0] = 0x39000003
    c[0x1E0] = 0x03;
    c[0x1E1] = 0x00;
    c[0x1E2] = 0x00;
    c[0x1E3] = 0x39;
    // 0x1E4: cbz w3, .copy_done (0x1F4): offset=16, imm19=4
    //   32-bit CBZ: 0x34000083
    c[0x1E4] = 0x83;
    c[0x1E5] = 0x00;
    c[0x1E6] = 0x00;
    c[0x1E7] = 0x34;
    // 0x1E8: add x0, x0, #1 = 0x91000400
    c[0x1E8] = 0x00;
    c[0x1E9] = 0x04;
    c[0x1EA] = 0x00;
    c[0x1EB] = 0x91;
    // 0x1EC: add x1, x1, #1 = 0x91000421
    c[0x1EC] = 0x21;
    c[0x1ED] = 0x04;
    c[0x1EE] = 0x00;
    c[0x1EF] = 0x91;
    // 0x1F0: b .copy_loop (0x1DC): offset=-20 = 0x17FFFFFB
    c[0x1F0] = 0xFB;
    c[0x1F1] = 0xFF;
    c[0x1F2] = 0xFF;
    c[0x1F3] = 0x17;
    // .copy_done at 0x1F4
    // 0x1F4: mov x0, x2 = 0xAA0203E0
    c[0x1F4] = 0xE0;
    c[0x1F5] = 0x03;
    c[0x1F6] = 0x02;
    c[0x1F7] = 0xAA;
    // 0x1F8: ret = 0xD65F03C0
    c[0x1F8] = 0xC0;
    c[0x1F9] = 0x03;
    c[0x1FA] = 0x5F;
    c[0x1FB] = 0xD6;

    // __kr_str_cat at 0x1FC
    // 0x1FC: mov x2, x0 (save dst) = 0xAA0003E2
    c[0x1FC] = 0xE2;
    c[0x1FD] = 0x03;
    c[0x1FE] = 0x00;
    c[0x1FF] = 0xAA;
    // .find_end at 0x200
    // 0x200: ldrb w3, [x0] = 0x39400003
    c[0x200] = 0x03;
    c[0x201] = 0x00;
    c[0x202] = 0x40;
    c[0x203] = 0x39;
    // 0x204: cbz w3, .cat_copy (0x210): offset=12, imm19=3
    //   = 0x34000063
    c[0x204] = 0x63;
    c[0x205] = 0x00;
    c[0x206] = 0x00;
    c[0x207] = 0x34;
    // 0x208: add x0, x0, #1 = 0x91000400
    c[0x208] = 0x00;
    c[0x209] = 0x04;
    c[0x20A] = 0x00;
    c[0x20B] = 0x91;
    // 0x20C: b .find_end (0x200): offset=-12 = 0x17FFFFFD
    c[0x20C] = 0xFD;
    c[0x20D] = 0xFF;
    c[0x20E] = 0xFF;
    c[0x20F] = 0x17;
    // .cat_copy at 0x210
    // 0x210: ldrb w3, [x1] = 0x39400023
    c[0x210] = 0x23;
    c[0x211] = 0x00;
    c[0x212] = 0x40;
    c[0x213] = 0x39;
    // 0x214: strb w3, [x0] = 0x39000003
    c[0x214] = 0x03;
    c[0x215] = 0x00;
    c[0x216] = 0x00;
    c[0x217] = 0x39;
    // 0x218: cbz w3, .cat_done (0x228): offset=16, imm19=4
    //   = 0x34000083
    c[0x218] = 0x83;
    c[0x219] = 0x00;
    c[0x21A] = 0x00;
    c[0x21B] = 0x34;
    // 0x21C: add x0, x0, #1 = 0x91000400
    c[0x21C] = 0x00;
    c[0x21D] = 0x04;
    c[0x21E] = 0x00;
    c[0x21F] = 0x91;
    // 0x220: add x1, x1, #1 = 0x91000421
    c[0x220] = 0x21;
    c[0x221] = 0x04;
    c[0x222] = 0x00;
    c[0x223] = 0x91;
    // 0x224: b .cat_copy (0x210): offset=-20 = 0x17FFFFFB
    c[0x224] = 0xFB;
    c[0x225] = 0xFF;
    c[0x226] = 0xFF;
    c[0x227] = 0x17;
    // .cat_done at 0x228
    // 0x228: mov x0, x2 = 0xAA0203E0
    c[0x228] = 0xE0;
    c[0x229] = 0x03;
    c[0x22A] = 0x02;
    c[0x22B] = 0xAA;
    // 0x22C: ret = 0xD65F03C0
    c[0x22C] = 0xC0;
    c[0x22D] = 0x03;
    c[0x22E] = 0x5F;
    c[0x22F] = 0xD6;

    // __kr_str_len at 0x230
    // 0x230: mov x1, #0 = 0xD2800001
    c[0x230] = 0x01;
    c[0x231] = 0x00;
    c[0x232] = 0x80;
    c[0x233] = 0xD2;
    // .len_loop at 0x234
    // 0x234: ldrb w2, [x0] = 0x39400002
    c[0x234] = 0x02;
    c[0x235] = 0x00;
    c[0x236] = 0x40;
    c[0x237] = 0x39;
    // 0x238: cbz w2, .len_done (0x248): offset=16, imm19=4
    //   = 0x34000082
    c[0x238] = 0x82;
    c[0x239] = 0x00;
    c[0x23A] = 0x00;
    c[0x23B] = 0x34;
    // 0x23C: add x0, x0, #1 = 0x91000400
    c[0x23C] = 0x00;
    c[0x23D] = 0x04;
    c[0x23E] = 0x00;
    c[0x23F] = 0x91;
    // 0x240: add x1, x1, #1 = 0x91000421
    c[0x240] = 0x21;
    c[0x241] = 0x04;
    c[0x242] = 0x00;
    c[0x243] = 0x91;
    // 0x244: b .len_loop (0x234): offset=-16 = 0x17FFFFFC
    c[0x244] = 0xFC;
    c[0x245] = 0xFF;
    c[0x246] = 0xFF;
    c[0x247] = 0x17;
    // .len_done at 0x248
    // 0x248: mov x0, x1 = 0xAA0103E0
    c[0x248] = 0xE0;
    c[0x249] = 0x03;
    c[0x24A] = 0x01;
    c[0x24B] = 0xAA;
    // 0x24C: ret = 0xD65F03C0
    c[0x24C] = 0xC0;
    c[0x24D] = 0x03;
    c[0x24E] = 0x5F;
    c[0x24F] = 0xD6;

    // Strings at 0x250
    // "/bin/sh\0"
    c[0x250] = 0x2F;
    c[0x251] = 0x62;
    c[0x252] = 0x69;
    c[0x253] = 0x6E;
    c[0x254] = 0x2F;
    c[0x255] = 0x73;
    c[0x256] = 0x68;
    c[0x257] = 0x00;
    // "-c\0"
    c[0x258] = 0x2D;
    c[0x259] = 0x63;
    c[0x25A] = 0x00;
    // Padding (5 bytes to align to 0x260)
    c[0x25B] = 0x00;
    c[0x25C] = 0x00;
    c[0x25D] = 0x00;
    c[0x25E] = 0x00;
    c[0x25F] = 0x00;

    // Data area at 0x260 (24 bytes, all zeros)
    // envp (0x260-0x267): already zero from initialization
    // heap_ptr (0x268-0x26F): already zero
    // heap_remaining (0x270-0x277): already zero

    // === __kr_file_open at 0x278 (10 instrs, 40 bytes) ===
    // Uses openat(AT_FDCWD, path, flags, mode) since AArch64 Linux has no plain open.
    // x0=path, x1=flags -> shuffle to openat(x0=AT_FDCWD, x1=path, x2=flags, x3=mode)
    c[0x278] = 0xE2;
    c[0x279] = 0x03;
    c[0x27A] = 0x01;
    c[0x27B] = 0xAA; // mov x2, x1
    c[0x27C] = 0xE1;
    c[0x27D] = 0x03;
    c[0x27E] = 0x00;
    c[0x27F] = 0xAA; // mov x1, x0
    c[0x280] = 0x60;
    c[0x281] = 0x0C;
    c[0x282] = 0x80;
    c[0x283] = 0x92; // movn x0, #99 (=-100)
    c[0x284] = 0x5F;
    c[0x285] = 0x04;
    c[0x286] = 0x00;
    c[0x287] = 0xF1; // cmp x2, #1
    c[0x288] = 0x41;
    c[0x289] = 0x00;
    c[0x28A] = 0x00;
    c[0x28B] = 0x54; // b.ne .do_open
    c[0x28C] = 0x21;
    c[0x28D] = 0x48;
    c[0x28E] = 0x80;
    c[0x28F] = 0xD2; // movz x2, #0x241 (renamed from x1)
    // Hmm wait, the user passed x1=1 which means wronly+creat+trunc.
    // We already moved original x1 to x2. So check x2==1, replace x2 with 0x241.
    c[0x290] = 0x83;
    c[0x291] = 0x34;
    c[0x292] = 0x80;
    c[0x293] = 0xD2; // movz x3, #0x1a4
    c[0x294] = 0x08;
    c[0x295] = 0x07;
    c[0x296] = 0x80;
    c[0x297] = 0xD2; // mov x8, #56 (openat)
    c[0x298] = 0x01;
    c[0x299] = 0x00;
    c[0x29A] = 0x00;
    c[0x29B] = 0xD4; // svc #0
    c[0x29C] = 0xC0;
    c[0x29D] = 0x03;
    c[0x29E] = 0x5F;
    c[0x29F] = 0xD6; // ret

    // === __kr_file_read at 0x2A0 (3 instrs, 12 bytes) ===
    c[0x2A0] = 0xE8;
    c[0x2A1] = 0x07;
    c[0x2A2] = 0x80;
    c[0x2A3] = 0xD2; // mov x8, #63
    c[0x2A4] = 0x01;
    c[0x2A5] = 0x00;
    c[0x2A6] = 0x00;
    c[0x2A7] = 0xD4; // svc #0
    c[0x2A8] = 0xC0;
    c[0x2A9] = 0x03;
    c[0x2AA] = 0x5F;
    c[0x2AB] = 0xD6; // ret

    // === __kr_file_write at 0x2AC (3 instrs, 12 bytes) ===
    c[0x2AC] = 0x08;
    c[0x2AD] = 0x08;
    c[0x2AE] = 0x80;
    c[0x2AF] = 0xD2; // mov x8, #64
    c[0x2B0] = 0x01;
    c[0x2B1] = 0x00;
    c[0x2B2] = 0x00;
    c[0x2B3] = 0xD4; // svc #0
    c[0x2B4] = 0xC0;
    c[0x2B5] = 0x03;
    c[0x2B6] = 0x5F;
    c[0x2B7] = 0xD6; // ret

    // === __kr_file_close at 0x2B8 (3 instrs, 12 bytes) ===
    c[0x2B8] = 0x28;
    c[0x2B9] = 0x07;
    c[0x2BA] = 0x80;
    c[0x2BB] = 0xD2; // mov x8, #57
    c[0x2BC] = 0x01;
    c[0x2BD] = 0x00;
    c[0x2BE] = 0x00;
    c[0x2BF] = 0xD4; // svc #0
    c[0x2C0] = 0xC0;
    c[0x2C1] = 0x03;
    c[0x2C2] = 0x5F;
    c[0x2C3] = 0xD6; // ret

    // === __kr_file_size at 0x2C4 (7 instrs, 28 bytes) ===
    c[0x2C4] = 0xFF;
    c[0x2C5] = 0x43;
    c[0x2C6] = 0x02;
    c[0x2C7] = 0xD1; // sub sp, sp, #144
    c[0x2C8] = 0xE1;
    c[0x2C9] = 0x03;
    c[0x2CA] = 0x00;
    c[0x2CB] = 0x91; // mov x1, sp
    c[0x2CC] = 0x08;
    c[0x2CD] = 0x0A;
    c[0x2CE] = 0x80;
    c[0x2CF] = 0xD2; // mov x8, #80 (fstat)
    c[0x2D0] = 0x01;
    c[0x2D1] = 0x00;
    c[0x2D2] = 0x00;
    c[0x2D3] = 0xD4; // svc #0
    c[0x2D4] = 0xE0;
    c[0x2D5] = 0x19;
    c[0x2D6] = 0x40;
    c[0x2D7] = 0xF9; // ldr x0, [sp, #48]
    c[0x2D8] = 0xFF;
    c[0x2D9] = 0x43;
    c[0x2DA] = 0x02;
    c[0x2DB] = 0x91; // add sp, sp, #144
    c[0x2DC] = 0xC0;
    c[0x2DD] = 0x03;
    c[0x2DE] = 0x5F;
    c[0x2DF] = 0xD6; // ret

    // === __kr_fmt_uint at 0x2E0 (34 instrs, 136 bytes) ===
    c[0x2E0] = 0xF3;
    c[0x2E1] = 0x53;
    c[0x2E2] = 0xBF;
    c[0x2E3] = 0xA9;
    c[0x2E4] = 0xF5;
    c[0x2E5] = 0x7B;
    c[0x2E6] = 0xBF;
    c[0x2E7] = 0xA9;
    c[0x2E8] = 0xF3;
    c[0x2E9] = 0x03;
    c[0x2EA] = 0x00;
    c[0x2EB] = 0xAA;
    c[0x2EC] = 0xF4;
    c[0x2ED] = 0x03;
    c[0x2EE] = 0x00;
    c[0x2EF] = 0xAA;
    c[0x2F0] = 0xE2;
    c[0x2F1] = 0x03;
    c[0x2F2] = 0x01;
    c[0x2F3] = 0xAA;
    c[0x2F4] = 0xE2;
    c[0x2F5] = 0x02;
    c[0x2F6] = 0x00;
    c[0x2F7] = 0xB4;
    c[0x2F8] = 0x03;
    c[0x2F9] = 0x00;
    c[0x2FA] = 0x80;
    c[0x2FB] = 0xD2;
    c[0x2FC] = 0x42;
    c[0x2FD] = 0x01;
    c[0x2FE] = 0x00;
    c[0x2FF] = 0xB4;
    c[0x300] = 0x44;
    c[0x301] = 0x01;
    c[0x302] = 0x80;
    c[0x303] = 0xD2;
    c[0x304] = 0x45;
    c[0x305] = 0x08;
    c[0x306] = 0xC4;
    c[0x307] = 0x9A;
    c[0x308] = 0xA6;
    c[0x309] = 0x88;
    c[0x30A] = 0x04;
    c[0x30B] = 0x9B;
    c[0x30C] = 0xC6;
    c[0x30D] = 0xC0;
    c[0x30E] = 0x00;
    c[0x30F] = 0x91;
    c[0x310] = 0xFF;
    c[0x311] = 0x43;
    c[0x312] = 0x00;
    c[0x313] = 0xD1;
    c[0x314] = 0xE6;
    c[0x315] = 0x03;
    c[0x316] = 0x00;
    c[0x317] = 0x39;
    c[0x318] = 0x63;
    c[0x319] = 0x04;
    c[0x31A] = 0x00;
    c[0x31B] = 0x91;
    c[0x31C] = 0xE2;
    c[0x31D] = 0x03;
    c[0x31E] = 0x05;
    c[0x31F] = 0xAA;
    c[0x320] = 0xF7;
    c[0x321] = 0xFF;
    c[0x322] = 0xFF;
    c[0x323] = 0x17;
    c[0x324] = 0xE3;
    c[0x325] = 0x00;
    c[0x326] = 0x00;
    c[0x327] = 0xB4;
    c[0x328] = 0xE5;
    c[0x329] = 0x03;
    c[0x32A] = 0x40;
    c[0x32B] = 0x39;
    c[0x32C] = 0xFF;
    c[0x32D] = 0x43;
    c[0x32E] = 0x00;
    c[0x32F] = 0x91;
    c[0x330] = 0x85;
    c[0x331] = 0x02;
    c[0x332] = 0x00;
    c[0x333] = 0x39;
    c[0x334] = 0x94;
    c[0x335] = 0x06;
    c[0x336] = 0x00;
    c[0x337] = 0x91;
    c[0x338] = 0x63;
    c[0x339] = 0x04;
    c[0x33A] = 0x00;
    c[0x33B] = 0xD1;
    c[0x33C] = 0xFA;
    c[0x33D] = 0xFF;
    c[0x33E] = 0xFF;
    c[0x33F] = 0x17;
    c[0x340] = 0x80;
    c[0x341] = 0x02;
    c[0x342] = 0x13;
    c[0x343] = 0xCB;
    c[0x344] = 0xF5;
    c[0x345] = 0x7B;
    c[0x346] = 0xC1;
    c[0x347] = 0xA8;
    c[0x348] = 0xF3;
    c[0x349] = 0x53;
    c[0x34A] = 0xC1;
    c[0x34B] = 0xA8;
    c[0x34C] = 0xC0;
    c[0x34D] = 0x03;
    c[0x34E] = 0x5F;
    c[0x34F] = 0xD6;
    c[0x350] = 0x05;
    c[0x351] = 0x06;
    c[0x352] = 0x80;
    c[0x353] = 0x52;
    c[0x354] = 0x05;
    c[0x355] = 0x00;
    c[0x356] = 0x00;
    c[0x357] = 0x39;
    c[0x358] = 0x20;
    c[0x359] = 0x00;
    c[0x35A] = 0x80;
    c[0x35B] = 0xD2;
    c[0x35C] = 0xF5;
    c[0x35D] = 0x7B;
    c[0x35E] = 0xC1;
    c[0x35F] = 0xA8;
    c[0x360] = 0xF3;
    c[0x361] = 0x53;
    c[0x362] = 0xC1;
    c[0x363] = 0xA8;
    c[0x364] = 0xC0;
    c[0x365] = 0x03;
    c[0x366] = 0x5F;
    c[0x367] = 0xD6;

    // === __kr_fmt_int at 0x368 (14 instrs, 56 bytes) ===
    c[0x368] = 0xFD;
    c[0x369] = 0x7B;
    c[0x36A] = 0xBF;
    c[0x36B] = 0xA9;
    c[0x36C] = 0x3F;
    c[0x36D] = 0x00;
    c[0x36E] = 0x01;
    c[0x36F] = 0x72;
    c[0x370] = 0x61;
    c[0x371] = 0x00;
    c[0x372] = 0x00;
    c[0x373] = 0x54;
    c[0x374] = 0xFD;
    c[0x375] = 0x7B;
    c[0x376] = 0xC1;
    c[0x377] = 0xA8;
    c[0x378] = 0xDA;
    c[0x379] = 0xFF;
    c[0x37A] = 0xFF;
    c[0x37B] = 0x17;
    c[0x37C] = 0xA2;
    c[0x37D] = 0x05;
    c[0x37E] = 0x80;
    c[0x37F] = 0x52;
    c[0x380] = 0x02;
    c[0x381] = 0x00;
    c[0x382] = 0x00;
    c[0x383] = 0x39;
    c[0x384] = 0xE1;
    c[0x385] = 0x03;
    c[0x386] = 0x01;
    c[0x387] = 0x4B;
    c[0x388] = 0xE1;
    c[0x389] = 0x03;
    c[0x38A] = 0x01;
    c[0x38B] = 0x2A;
    c[0x38C] = 0x00;
    c[0x38D] = 0x04;
    c[0x38E] = 0x00;
    c[0x38F] = 0x91;
    c[0x390] = 0xD4;
    c[0x391] = 0xFF;
    c[0x392] = 0xFF;
    c[0x393] = 0x97;
    c[0x394] = 0x00;
    c[0x395] = 0x04;
    c[0x396] = 0x00;
    c[0x397] = 0x91;
    c[0x398] = 0xFD;
    c[0x399] = 0x7B;
    c[0x39A] = 0xC1;
    c[0x39B] = 0xA8;
    c[0x39C] = 0xC0;
    c[0x39D] = 0x03;
    c[0x39E] = 0x5F;
    c[0x39F] = 0xD6;

    // === __kr_fmt_hex at 0x3A0 (36 instrs, 144 bytes) ===
    c[0x3A0] = 0xF3;
    c[0x3A1] = 0x53;
    c[0x3A2] = 0xBF;
    c[0x3A3] = 0xA9;
    c[0x3A4] = 0xF5;
    c[0x3A5] = 0x7B;
    c[0x3A6] = 0xBF;
    c[0x3A7] = 0xA9;
    c[0x3A8] = 0xF3;
    c[0x3A9] = 0x03;
    c[0x3AA] = 0x00;
    c[0x3AB] = 0xAA;
    c[0x3AC] = 0xF4;
    c[0x3AD] = 0x03;
    c[0x3AE] = 0x00;
    c[0x3AF] = 0xAA;
    c[0x3B0] = 0xE2;
    c[0x3B1] = 0x03;
    c[0x3B2] = 0x01;
    c[0x3B3] = 0xAA;
    c[0x3B4] = 0x22;
    c[0x3B5] = 0x03;
    c[0x3B6] = 0x00;
    c[0x3B7] = 0xB4;
    c[0x3B8] = 0x03;
    c[0x3B9] = 0x00;
    c[0x3BA] = 0x80;
    c[0x3BB] = 0xD2;
    c[0x3BC] = 0x82;
    c[0x3BD] = 0x01;
    c[0x3BE] = 0x00;
    c[0x3BF] = 0xB4;
    c[0x3C0] = 0x44;
    c[0x3C1] = 0x0C;
    c[0x3C2] = 0x40;
    c[0x3C3] = 0x92;
    c[0x3C4] = 0x42;
    c[0x3C5] = 0xFC;
    c[0x3C6] = 0x44;
    c[0x3C7] = 0xD3;
    c[0x3C8] = 0x9F;
    c[0x3C9] = 0x28;
    c[0x3CA] = 0x00;
    c[0x3CB] = 0xF1;
    c[0x3CC] = 0x63;
    c[0x3CD] = 0x00;
    c[0x3CE] = 0x00;
    c[0x3CF] = 0x54;
    c[0x3D0] = 0x84;
    c[0x3D1] = 0x5C;
    c[0x3D2] = 0x01;
    c[0x3D3] = 0x91;
    c[0x3D4] = 0x02;
    c[0x3D5] = 0x00;
    c[0x3D6] = 0x00;
    c[0x3D7] = 0x14;
    c[0x3D8] = 0x84;
    c[0x3D9] = 0xC0;
    c[0x3DA] = 0x00;
    c[0x3DB] = 0x91;
    c[0x3DC] = 0xFF;
    c[0x3DD] = 0x43;
    c[0x3DE] = 0x00;
    c[0x3DF] = 0xD1;
    c[0x3E0] = 0xE4;
    c[0x3E1] = 0x03;
    c[0x3E2] = 0x00;
    c[0x3E3] = 0x39;
    c[0x3E4] = 0x63;
    c[0x3E5] = 0x04;
    c[0x3E6] = 0x00;
    c[0x3E7] = 0x91;
    c[0x3E8] = 0xF5;
    c[0x3E9] = 0xFF;
    c[0x3EA] = 0xFF;
    c[0x3EB] = 0x17;
    c[0x3EC] = 0xE3;
    c[0x3ED] = 0x00;
    c[0x3EE] = 0x00;
    c[0x3EF] = 0xB4;
    c[0x3F0] = 0xE5;
    c[0x3F1] = 0x03;
    c[0x3F2] = 0x40;
    c[0x3F3] = 0x39;
    c[0x3F4] = 0xFF;
    c[0x3F5] = 0x43;
    c[0x3F6] = 0x00;
    c[0x3F7] = 0x91;
    c[0x3F8] = 0x85;
    c[0x3F9] = 0x02;
    c[0x3FA] = 0x00;
    c[0x3FB] = 0x39;
    c[0x3FC] = 0x94;
    c[0x3FD] = 0x06;
    c[0x3FE] = 0x00;
    c[0x3FF] = 0x91;
    c[0x400] = 0x63;
    c[0x401] = 0x04;
    c[0x402] = 0x00;
    c[0x403] = 0xD1;
    c[0x404] = 0xFA;
    c[0x405] = 0xFF;
    c[0x406] = 0xFF;
    c[0x407] = 0x17;
    c[0x408] = 0x80;
    c[0x409] = 0x02;
    c[0x40A] = 0x13;
    c[0x40B] = 0xCB;
    c[0x40C] = 0xF5;
    c[0x40D] = 0x7B;
    c[0x40E] = 0xC1;
    c[0x40F] = 0xA8;
    c[0x410] = 0xF3;
    c[0x411] = 0x53;
    c[0x412] = 0xC1;
    c[0x413] = 0xA8;
    c[0x414] = 0xC0;
    c[0x415] = 0x03;
    c[0x416] = 0x5F;
    c[0x417] = 0xD6;
    c[0x418] = 0x05;
    c[0x419] = 0x06;
    c[0x41A] = 0x80;
    c[0x41B] = 0x52;
    c[0x41C] = 0x05;
    c[0x41D] = 0x00;
    c[0x41E] = 0x00;
    c[0x41F] = 0x39;
    c[0x420] = 0x20;
    c[0x421] = 0x00;
    c[0x422] = 0x80;
    c[0x423] = 0xD2;
    c[0x424] = 0xF5;
    c[0x425] = 0x7B;
    c[0x426] = 0xC1;
    c[0x427] = 0xA8;
    c[0x428] = 0xF3;
    c[0x429] = 0x53;
    c[0x42A] = 0xC1;
    c[0x42B] = 0xA8;
    c[0x42C] = 0xC0;
    c[0x42D] = 0x03;
    c[0x42E] = 0x5F;
    c[0x42F] = 0xD6;

    c
};

// Offset constants for the symbol table
const OFF_STR_COPY: u32 = 0x1D8;
const OFF_STR_CAT: u32 = 0x1FC;
const OFF_STR_LEN: u32 = 0x230;
const OFF_EXEC: u32 = 0x124;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_size_is_correct() {
        // 1072 bytes total: code + strings + padding + data + file I/O + formatting
        assert_eq!(BLOB.code.len(), 1072);
    }

    #[test]
    fn all_symbols_within_bounds() {
        assert_eq!(BLOB.symbols.len(), 19);
        for &(name, offset) in BLOB.symbols {
            assert!(
                (offset as usize) < BLOB.code.len(),
                "symbol {name} offset 0x{offset:X} out of bounds (blob len {})",
                BLOB.code.len()
            );
        }
    }

    #[test]
    fn main_call_fixup_within_bounds() {
        // The fixup points to a 4-byte BL instruction.
        assert!(
            (BLOB.main_call_fixup as usize + 4) <= BLOB.code.len(),
            "main_call_fixup + 4 = {} exceeds blob len {}",
            BLOB.main_call_fixup as usize + 4,
            BLOB.code.len()
        );
    }

    #[test]
    fn bl_main_opcode_correct() {
        // The BL instruction at main_call_fixup should be 0x94000000 (BL with imm26=0).
        let off = BLOB.main_call_fixup as usize;
        let instr = u32::from_le_bytes([
            BLOB.code[off],
            BLOB.code[off + 1],
            BLOB.code[off + 2],
            BLOB.code[off + 3],
        ]);
        assert_eq!(
            instr, 0x94000000,
            "expected BL placeholder 0x94000000 at offset 0x{off:X}, found 0x{instr:08X}"
        );
    }

    #[test]
    fn data_area_is_zeroed() {
        // 24-byte data area at 0x260 (no longer at end of blob)
        let data_start = 0x260;
        for i in data_start..data_start + 24 {
            assert_eq!(
                BLOB.code[i], 0x00,
                "data area byte at offset {i} is 0x{:02X}, expected 0x00",
                BLOB.code[i]
            );
        }
    }

    #[test]
    fn exit_follows_start_fallthrough() {
        let exit_offset = BLOB.symbol_offset("__kr_exit").unwrap() as usize;
        assert_eq!(exit_offset, 0x28);
    }

    #[test]
    fn dealloc_is_just_ret() {
        let off = BLOB.symbol_offset("__kr_dealloc").unwrap() as usize;
        let instr = u32::from_le_bytes([
            BLOB.code[off],
            BLOB.code[off + 1],
            BLOB.code[off + 2],
            BLOB.code[off + 3],
        ]);
        assert_eq!(instr, 0xD65F03C0, "dealloc should be a single RET");
    }

    #[test]
    fn inline_strings_present() {
        assert_eq!(&BLOB.code[0x250..0x258], b"/bin/sh\0");
        assert_eq!(&BLOB.code[0x258..0x25B], b"-c\0");
    }

    #[test]
    fn symbol_names_complete() {
        let expected = [
            "_start",
            "__kr_exit",
            "__kr_write",
            "__kr_mmap_alloc",
            "__kr_alloc",
            "__kr_dealloc",
            "__kr_getenv",
            "__kr_exec",
            "__kr_str_copy",
            "__kr_str_cat",
            "__kr_str_len",
            "__kr_file_open",
            "__kr_file_read",
            "__kr_file_write",
            "__kr_file_close",
            "__kr_file_size",
            "__kr_fmt_uint",
            "__kr_fmt_int",
            "__kr_fmt_hex",
        ];
        for name in &expected {
            assert!(BLOB.symbol_offset(name).is_some(), "missing symbol: {name}");
        }
    }

    #[test]
    fn all_instructions_are_4byte_aligned() {
        // Every symbol should be at a 4-byte aligned offset (AArch64 requirement).
        for &(name, offset) in BLOB.symbols {
            assert_eq!(
                offset % 4,
                0,
                "symbol {name} at offset 0x{offset:X} is not 4-byte aligned"
            );
        }
    }

    #[test]
    fn svc_encoding_correct() {
        // Check that SVC #0 (0xD4000001) appears after the write syscall number load.
        let write_off = BLOB.symbol_offset("__kr_write").unwrap() as usize;
        // write: mov x8, #64; svc #0; ret
        let svc_off = write_off + 4;
        let instr = u32::from_le_bytes([
            BLOB.code[svc_off],
            BLOB.code[svc_off + 1],
            BLOB.code[svc_off + 2],
            BLOB.code[svc_off + 3],
        ]);
        assert_eq!(instr, 0xD4000001, "expected SVC #0 at write+4");
    }
}
