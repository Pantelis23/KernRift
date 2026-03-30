//! Windows x86_64 runtime blob — hand-assembled machine code.
//! Implements `_start` and all `__kr_*` functions using Win32 API via IAT.
//!
//! Layout (541 bytes total):
//!   0x000 .. 0x113  executable code (11 original functions)
//!   0x114 .. 0x117  padding (4 bytes)
//!   0x118 .. 0x13f  data area: envp(8) + heap_ptr(8) + heap_remaining(8)
//!                               + stdout_handle(8) + iat_base(8) = 40 bytes
//!   0x140 .. 0x151  file I/O stubs (5 functions, returns error)
//!   0x152 .. 0x21c  formatting functions (3 functions)
//!
//! Windows x86_64 calling convention (Microsoft x64):
//!   Args: rcx, rdx, r8, r9, stack. Return in rax.
//!   Caller must reserve 32 bytes shadow space.
//!   Callee-saved: rbx, rbp, rdi, rsi, rsp, r12-r15.
//!
//! IAT (Import Address Table) layout — the PE linker patches iat_base with the
//! address of a table of function pointers:
//!   [iat_base + 0x00]: GetStdHandle
//!   [iat_base + 0x08]: WriteFile
//!   [iat_base + 0x10]: ExitProcess
//!   [iat_base + 0x18]: VirtualAlloc
//!   [iat_base + 0x20]: GetEnvironmentVariableA
//!   [iat_base + 0x28]: CreateProcessA
//!   [iat_base + 0x30]: WaitForSingleObject
//!   [iat_base + 0x38]: GetExitCodeProcess
//!
//! KernRift ABI: The compiler emits calls using the SysV-like convention
//!   (args in rdi, rsi, rdx, rcx, r8, r9). The blob functions translate
//!   from this ABI to the Windows x64 convention internally.
//!
//! NOTE: The iat_base data slot is patched by the PE hostexe linker at link time.

use super::RuntimeBlob;

// Data area at 0x1f0 (40 bytes):
//   envp           = 0x1f0  (unused on Windows; environment accessed via API)
//   heap_ptr       = 0x1f8
//   heap_remaining = 0x200
//   stdout_handle  = 0x208
//   iat_base       = 0x210

const OFF_FILE_OPEN: u32 = 0x140;
const OFF_FILE_READ: u32 = 0x148;
const OFF_FILE_WRITE: u32 = 0x14b;
const OFF_FILE_CLOSE: u32 = 0x14e;
const OFF_FILE_SIZE: u32 = 0x14f;
const OFF_FMT_UINT: u32 = 0x152;
const OFF_FMT_INT: u32 = 0x1a7;
const OFF_FMT_HEX: u32 = 0x1c0;

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
        ("__kr_file_open", OFF_FILE_OPEN),
        ("__kr_file_read", OFF_FILE_READ),
        ("__kr_file_write", OFF_FILE_WRITE),
        ("__kr_file_close", OFF_FILE_CLOSE),
        ("__kr_file_size", OFF_FILE_SIZE),
        ("__kr_fmt_uint", OFF_FMT_UINT),
        ("__kr_fmt_int", OFF_FMT_INT),
        ("__kr_fmt_hex", OFF_FMT_HEX),
    ],
    main_call_fixup: OFF_BL_MAIN,
    iat_base_data_offset: Some(0x138),
};

// Instruction layout with exact offsets:
//
// _start at 0x00:
//   0x00: sub rsp, 40          ; 4 bytes  (shadow + align)
//   0x04: mov rax, [rip+IAT]   ; 7 bytes  load iat_base
//   0x0b: mov ecx, -11         ; 5 bytes  STD_OUTPUT_HANDLE
//   0x10: call [rax]            ; 2 bytes  GetStdHandle
//   0x12: mov [rip+STDOUT], rax ; 7 bytes  save handle
//   0x19: call main             ; 5 bytes  FIXUP
//   0x1e: mov ecx, eax          ; 2 bytes  exit code -> ecx for ExitProcess
//   total: 32 bytes, ends at 0x20
//
// __kr_exit at 0x20:
//   0x20: mov rax, [rip+IAT]   ; 7 bytes
//   0x27: jmp [rax+0x10]       ; 3 bytes  ExitProcess(ecx)
//   total: 10 bytes, ends at 0x2a
//
// __kr_write at 0x2a:
//   KR ABI: rdi=fd(ignored), rsi=buf, rdx=len
//   Win64: WriteFile(rcx=handle, rdx=buf, r8d=len, r9=&written, [rsp+32]=NULL)
//   0x2a: push rbx              ; 1
//   0x2b: sub rsp, 48           ; 4  (32 shadow + 8 written + 8 align)
//   0x2f: mov rcx, [rip+STDOUT] ; 7
//   0x36: mov r8d, edx          ; 3  len
//   0x39: mov rdx, rsi           ; 3  buf
//   0x3c: lea r9, [rsp+32]      ; 5  &written
//   0x41: mov qword [rsp+40], 0  ; 8  lpOverlapped = NULL
//   0x49: mov rax, [rip+IAT]    ; 7
//   0x50: call [rax+8]           ; 3  WriteFile
//   0x53: add rsp, 48            ; 4
//   0x57: pop rbx                ; 1
//   0x58: ret                    ; 1
//   total: 47 bytes, ends at 0x59
//
// __kr_mmap_alloc at 0x59:
//   KR ABI: rdi=size
//   Win64: VirtualAlloc(rcx=NULL, rdx=size, r8=MEM_COMMIT|MEM_RESERVE, r9=PAGE_READWRITE)
//   0x59: sub rsp, 40           ; 4
//   0x5d: xor ecx, ecx          ; 2  lpAddress = NULL
//   0x5f: mov rdx, rdi           ; 3  dwSize
//   0x62: mov r8d, 0x3000        ; 6  MEM_COMMIT|MEM_RESERVE
//   0x68: mov r9d, 4             ; 6  PAGE_READWRITE
//   0x6e: mov rax, [rip+IAT]    ; 7
//   0x75: call [rax+0x18]        ; 3  VirtualAlloc
//   0x78: add rsp, 40            ; 4
//   0x7c: ret                    ; 1
//   total: 36 bytes, ends at 0x7d
//
// __kr_alloc at 0x7d:
//   Same logic as Linux, but calls our __kr_mmap_alloc (which uses VirtualAlloc).
//   0x7d: add rdi, 15           ; 4
//   0x81: and rdi, -16          ; 4
//   0x85: mov rax, [rip+REM]   ; 7  heap_remaining -> ends at 0x8c
//   0x8c: cmp rdi, rax          ; 3
//   0x8f: jbe .have_space       ; 2  +0x1e -> 0xaf
//   0x91: push rdi              ; 1
//   0x92: mov edi, 0x400000     ; 5
//   0x97: call __kr_mmap_alloc  ; 5  -> 0x59 (0x59-0x9c = -0x43)
//   0x9c: mov [rip+PTR], rax   ; 7  -> ends at 0xa3
//   0xa3: mov qword [rip+REM], 0x400000 ; 11  -> ends at 0xae
//   0xae: pop rdi               ; 1
//   .have_space at 0xaf:
//   0xaf: mov rax, [rip+PTR]   ; 7  -> ends at 0xb6
//   0xb6: add [rip+PTR], rdi   ; 7  -> ends at 0xbd
//   0xbd: sub [rip+REM], rdi   ; 7  -> ends at 0xc4
//   0xc4: ret                   ; 1
//   total: 72 bytes, ends at 0xc5
//
// __kr_dealloc at 0xc5:
//   0xc5: ret                   ; 1
//
// __kr_getenv at 0xc6:
//   KR ABI: rdi=name
//   Win64: GetEnvironmentVariableA(rcx=name, rdx=buf, r8=size)
//   For getenv, we allocate a small buffer on the stack.
//   0xc6: sub rsp, 296          ; 4  (256 buf + 32 shadow + 8 align)
//   0xca: mov rcx, rdi           ; 3  lpName
//   0xcd: lea rdx, [rsp+32]     ; 5  lpBuffer (after shadow)
//   0xd2: mov r8d, 256           ; 6  nSize
//   0xd8: mov rax, [rip+IAT]    ; 7
//   0xdf: call [rax+0x20]        ; 3  GetEnvironmentVariableA
//   0xe2: test eax, eax          ; 2
//   0xe4: je .not_found          ; 2  +0x08 -> 0xee
//   0xe6: lea rax, [rsp+32]     ; 5  return buffer pointer
//   0xeb: add rsp, 296           ; 4  (wait, this is a problem — buffer is on stack!)
//   Actually this won't work because the buffer is stack-allocated and we return
//   a pointer to it. We need to use a static buffer or heap-allocate.
//   Let me use a static buffer in the data area. Or better: allocate via __kr_alloc.
//   For simplicity, use a fixed 256-byte static buffer. But that's a lot of data area.
//   Alternative: call __kr_alloc first, then GetEnvironmentVariableA into that buffer.
//   But that's complex. Let me use a simpler approach: scan the environment block
//   ourselves (like Unix), since Windows stores environment as a block of
//   VAR=VALUE\0 strings terminated by \0\0.
//   But we already have environment via PEB. Let me just store the environment
//   block pointer and scan it like the Unix getenv.
//
// OK let me redesign. Since the KR runtime needs getenv to return a pointer to
// the value that persists, we should scan the environment block (which is already
// in the process memory). Windows environment block: a series of "VAR=VALUE\0"
// strings, terminated by an extra \0.
//
// _start will save the environment block pointer from PEB.
// __kr_getenv scans it like a linear search.

// Let me redo the layout with this approach. The environment block is
// a wide string (UTF-16) on Windows. Actually, GetEnvironmentStringsA returns
// ANSI. Or we use the PEB -> ProcessParameters -> Environment which is UTF-16.
// For simplicity, use GetEnvironmentStringsA at startup if available, or just
// use GetEnvironmentVariableA for each lookup.
//
// Actually, let me just implement getenv as a call to GetEnvironmentVariableA
// with a heap-allocated buffer. Or even simpler: since the KR programs are
// simple, we can use a fixed-size data area buffer.
//
// REVISED PLAN: Keep it simple. Use a 256-byte static buffer at the end of the
// data area for getenv results (making data area 40 + 256 = 296 bytes).
// Or just accept the limitation that getenv returns a stack pointer that's only
// valid during the current call chain (which is usually fine for KR programs).

// Actually, the simplest correct approach for a minimal runtime:
// __kr_getenv: not supported on Windows (returns NULL).
// This is the same approach many embedded runtimes take.
// The linker can be enhanced later to support it.

// FINAL PLAN for Windows blob - keep it minimal and correct:
// - _start: get stdout handle, call main, call ExitProcess
// - __kr_exit: ExitProcess
// - __kr_write: WriteFile to stdout
// - __kr_mmap_alloc: VirtualAlloc
// - __kr_alloc: bump allocator (same as Unix)
// - __kr_dealloc: no-op
// - __kr_getenv: returns NULL (not implemented)
// - __kr_exec: CreateProcessA + WaitForSingleObject + GetExitCodeProcess
// - String functions: same pure byte loops

// Recalculated offsets with this simplified approach:
// _start: 32 bytes (0x00-0x1f)
// __kr_exit: 10 bytes (0x20-0x29)
// __kr_write: 47 bytes (0x2a-0x58)
// __kr_mmap_alloc: 36 bytes (0x59-0x7c)
// __kr_alloc: 72 bytes (0x7d-0xc4)
// __kr_dealloc: 1 byte (0xc5)
// __kr_getenv: 4 bytes (0xc6-0xc9) — xor eax,eax; ret
// __kr_exec: complicated, ~100 bytes (0xca-0x12d)
// __kr_str_copy: 21 bytes
// __kr_str_cat: 31 bytes
// __kr_str_len: 16 bytes

// Actually let me count __kr_exec carefully:
// CreateProcessA has 10 parameters (!) — significant stack setup.
// For simplicity, use system("cmd /c command") equivalent:
// Actually Windows doesn't have system() in kernel32. Use CreateProcessA.
//
// CreateProcessA(
//   lpApplicationName = NULL,
//   lpCommandLine = "cmd.exe /c <user_cmd>",
//   ... 8 more params mostly NULL/0 ...
//   lpProcessInformation = &pi (16 bytes)
// )
// Then WaitForSingleObject(pi.hProcess, INFINITE)
// Then GetExitCodeProcess(pi.hProcess, &exit_code)
//
// This requires building the command line string. Complex for a hand-assembled blob.
// For Phase 3a, let me keep __kr_exec as a stub that returns 1 (not implemented).
// It can be properly implemented when the PE linker is done.

// FINAL simplified layout:
// _start at 0x00 (32 bytes)
// __kr_exit at 0x20 (10 bytes)
// __kr_write at 0x2a (47 bytes)
// __kr_mmap_alloc at 0x59 (36 bytes)
// __kr_alloc at 0x7d (72 bytes)
// __kr_dealloc at 0xc5 (1 byte)
// __kr_getenv at 0xc6 (3 bytes: xor eax,eax; ret)
// __kr_exec at 0xc9 (7 bytes: mov eax,1; ret)
// __kr_str_copy at 0xd0 (21 bytes)
// __kr_str_cat at 0xe5 (31 bytes)
// __kr_str_len at 0x104 (16 bytes)
// padding to 0x118 (4 bytes)
// data area at 0x118 (40 bytes)
// total: 0x140 = 320 bytes

// Data area positions:
// envp           = 0x118
// heap_ptr       = 0x120
// heap_remaining = 0x128
// stdout_handle  = 0x130
// iat_base       = 0x138

// RIP-relative displacements (computed from end of each instruction):
// _start:
//   0x04: mov rax, [rip+IAT]    ends at 0x0b, target=0x138, disp=0x138-0x0b=0x12d
//   0x12: mov [rip+STDOUT], rax  ends at 0x19, target=0x130, disp=0x130-0x19=0x117
//   0x19: call main              ends at 0x1e, FIXUP at 0x1a
// __kr_exit:
//   0x20: mov rax, [rip+IAT]    ends at 0x27, target=0x138, disp=0x138-0x27=0x111
// __kr_write:
//   0x2f: mov rcx, [rip+STDOUT] ends at 0x36, target=0x130, disp=0x130-0x36=0xfa
//   0x49: mov rax, [rip+IAT]    ends at 0x50, target=0x138, disp=0x138-0x50=0xe8
// __kr_mmap_alloc:
//   0x6e: mov rax, [rip+IAT]    ends at 0x75, target=0x138, disp=0x138-0x75=0xc3
// __kr_alloc:
//   0x85: mov rax, [rip+REM]    ends at 0x8c, target=0x128, disp=0x128-0x8c=0x9c
//   0x97: call mmap_alloc        ends at 0x9c, target=0x59, disp=0x59-0x9c=-0x43
//   0x9c: mov [rip+PTR], rax    ends at 0xa3, target=0x120, disp=0x120-0xa3=0x7d
//   0xa3: mov qword [rip+REM], 0x400000  ends at 0xae, target=0x128, disp=0x128-0xae=0x7a
//   0xaf: mov rax, [rip+PTR]    ends at 0xb6, target=0x120, disp=0x120-0xb6=0x6a
//   0xb6: add [rip+PTR], rdi    ends at 0xbd, target=0x120, disp=0x120-0xbd=0x63
//   0xbd: sub [rip+REM], rdi    ends at 0xc4, target=0x128, disp=0x128-0xc4=0x64

const OFF_EXIT: u32 = 0x20;
const OFF_WRITE: u32 = 0x2a;
const OFF_MMAP: u32 = 0x5a;
const OFF_ALLOC: u32 = 0x7e;
const OFF_DEALLOC: u32 = 0xc6;
const OFF_GETENV: u32 = 0xc7;
const OFF_EXEC: u32 = 0xca;
const OFF_STR_COPY: u32 = 0xd0;
const OFF_STR_CAT: u32 = 0xe5;
const OFF_STR_LEN: u32 = 0x104;
const OFF_BL_MAIN: u32 = 0x1a; // offset of rel32 in call main

const CODE: [u8; 541] = {
    let mut c = [0u8; 541];

    // === _start at 0x00 ===
    // 0x00: sub rsp, 40 = 48 83 EC 28
    c[0x00] = 0x48;
    c[0x01] = 0x83;
    c[0x02] = 0xEC;
    c[0x03] = 0x28;
    // 0x04: mov rax, [rip+0x12d] (iat_base at 0x138) = 48 8B 05 2D 01 00 00
    c[0x04] = 0x48;
    c[0x05] = 0x8B;
    c[0x06] = 0x05;
    c[0x07] = 0x2D;
    c[0x08] = 0x01;
    c[0x09] = 0x00;
    c[0x0A] = 0x00;
    // 0x0b: mov ecx, -11 (STD_OUTPUT_HANDLE) = B9 F5 FF FF FF
    c[0x0B] = 0xB9;
    c[0x0C] = 0xF5;
    c[0x0D] = 0xFF;
    c[0x0E] = 0xFF;
    c[0x0F] = 0xFF;
    // 0x10: call [rax] (GetStdHandle) = FF 10
    c[0x10] = 0xFF;
    c[0x11] = 0x10;
    // 0x12: mov [rip+0x117], rax (stdout_handle at 0x130) = 48 89 05 17 01 00 00
    c[0x12] = 0x48;
    c[0x13] = 0x89;
    c[0x14] = 0x05;
    c[0x15] = 0x17;
    c[0x16] = 0x01;
    c[0x17] = 0x00;
    c[0x18] = 0x00;
    // 0x19: call main (placeholder) = E8 00 00 00 00
    c[0x19] = 0xE8;
    c[0x1A] = 0x00;
    c[0x1B] = 0x00;
    c[0x1C] = 0x00;
    c[0x1D] = 0x00;
    // 0x1e: mov ecx, eax = 89 C1
    c[0x1E] = 0x89;
    c[0x1F] = 0xC1;

    // === __kr_exit at 0x20 ===
    // 0x20: mov rax, [rip+0x111] (iat_base at 0x138) = 48 8B 05 11 01 00 00
    c[0x20] = 0x48;
    c[0x21] = 0x8B;
    c[0x22] = 0x05;
    c[0x23] = 0x11;
    c[0x24] = 0x01;
    c[0x25] = 0x00;
    c[0x26] = 0x00;
    // 0x27: jmp [rax+0x10] (ExitProcess) = FF 60 10
    c[0x27] = 0xFF;
    c[0x28] = 0x60;
    c[0x29] = 0x10;

    // === __kr_write at 0x2a ===
    // KR ABI: rdi=fd(ignored), rsi=buf, rdx=len
    // 0x2a: push rbx = 53
    c[0x2A] = 0x53;
    // 0x2b: sub rsp, 48 = 48 83 EC 30  (32 shadow + 8 written_var + 8 overlap param)
    c[0x2B] = 0x48;
    c[0x2C] = 0x83;
    c[0x2D] = 0xEC;
    c[0x2E] = 0x30;
    // 0x2f: mov rcx, [rip+0xfa] (stdout at 0x130) = 48 8B 0D FA 00 00 00
    c[0x2F] = 0x48;
    c[0x30] = 0x8B;
    c[0x31] = 0x0D;
    c[0x32] = 0xFA;
    c[0x33] = 0x00;
    c[0x34] = 0x00;
    c[0x35] = 0x00;
    // 0x36: mov r8d, edx = 44 89 D0
    // Wait, 44 89 D0 is mov eax, r10d. Let me use correct encoding.
    // mov r8d, edx: destination r8d (extended), source edx
    // REX.R: 41 89 D0 ? No:
    // mov r8d, edx = 41 89 D0 (REX.B=1 for r8, opcode 89, modrm D0=edx->r8)
    // Actually: MOV r/m32, r32: opcode 89, modrm=11 010 000 (reg=edx=2, rm=r8=0+REX.B)
    // = 41 89 D0
    c[0x36] = 0x41;
    c[0x37] = 0x89;
    c[0x38] = 0xD0;
    // 0x39: mov rdx, rsi = 48 89 F2
    c[0x39] = 0x48;
    c[0x3A] = 0x89;
    c[0x3B] = 0xF2;
    // 0x3c: lea r9, [rsp+32] = 4C 8D 4C 24 20
    c[0x3C] = 0x4C;
    c[0x3D] = 0x8D;
    c[0x3E] = 0x4C;
    c[0x3F] = 0x24;
    c[0x40] = 0x20;
    // 0x41: mov qword [rsp+40], 0 = 48 C7 44 24 28 00 00 00 00
    // (lpOverlapped = NULL, 5th arg at [rsp+0x28] after shadow)
    c[0x41] = 0x48;
    c[0x42] = 0xC7;
    c[0x43] = 0x44;
    c[0x44] = 0x24;
    c[0x45] = 0x28;
    c[0x46] = 0x00;
    c[0x47] = 0x00;
    c[0x48] = 0x00;
    c[0x49] = 0x00;
    // 0x4a: mov rax, [rip+0xe8] (iat at 0x138): wait, 0x4a+7=0x51, 0x138-0x51=0xe7
    // Let me recompute: instruction at 0x4a, 7 bytes, ends at 0x51
    // disp = 0x138 - 0x51 = 0xe7
    c[0x4A] = 0x48;
    c[0x4B] = 0x8B;
    c[0x4C] = 0x05;
    c[0x4D] = 0xE7;
    c[0x4E] = 0x00;
    c[0x4F] = 0x00;
    c[0x50] = 0x00;
    // 0x51: call [rax+8] (WriteFile) = FF 50 08
    c[0x51] = 0xFF;
    c[0x52] = 0x50;
    c[0x53] = 0x08;
    // 0x54: add rsp, 48 = 48 83 C4 30
    c[0x54] = 0x48;
    c[0x55] = 0x83;
    c[0x56] = 0xC4;
    c[0x57] = 0x30;
    // 0x58: pop rbx = 5B
    c[0x58] = 0x5B;
    // Wait, that ends at 0x59 = 47 bytes from 0x2a. Good.
    // But I skipped ret! WriteFile returns count, but __kr_write should return.
    // Let me add ret:
    // Actually the total should be 0x2a to 0x59 exclusive = 47 bytes. But we need
    // a ret at 0x59.
    // Let me recount: 0x2a to 0x58 inclusive is 47 bytes (0x2a..=0x58). pop at 0x58.
    // ret at 0x59:
    // But __kr_mmap_alloc starts at 0x59! I miscounted.
    // pop rbx=1byte at 0x58, so next byte is 0x59. ret needs to go before mmap.
    // push(1) + sub(4) + mov(7) + mov(3) + mov(3) + lea(5) + movq(9) + mov(7) + call(3) + add(4) + pop(1) = 47
    // 0x2a + 47 = 0x59. So I need ret at 0x59, pushing mmap to 0x5a.
    // Let me fix:
    c[0x59] = 0xC3; // ret
    // Now __kr_mmap_alloc starts at 0x5a.
    // I need to update all downstream offsets. This is cascading...

    // Rather than fixing everything, let me just adjust the constants and redo from
    // __kr_write being 48 bytes (including ret) instead of 47.

    // OK actually the issue is I forgot to account for ret in my initial layout.
    // Let me redefine everything:
    //
    // __kr_write: 0x2a .. 0x59 (push+sub+mov+mov+mov+lea+movq+mov+call+add+pop = 47 bytes)
    //   then ret at 0x59 = 48 bytes total, ends at 0x5a
    // __kr_mmap_alloc at 0x5a
    // etc.
    //
    // This shifts everything by 1. Let me just redo the offsets:

    // === __kr_mmap_alloc at 0x5a ===
    // 0x5a: sub rsp, 40 = 48 83 EC 28
    c[0x5A] = 0x48;
    c[0x5B] = 0x83;
    c[0x5C] = 0xEC;
    c[0x5D] = 0x28;
    // 0x5e: xor ecx, ecx = 31 C9  (lpAddress = NULL)
    c[0x5E] = 0x31;
    c[0x5F] = 0xC9;
    // 0x60: mov rdx, rdi = 48 89 FA  (dwSize)
    c[0x60] = 0x48;
    c[0x61] = 0x89;
    c[0x62] = 0xFA;
    // 0x63: mov r8d, 0x3000 = 41 B8 00 30 00 00  (MEM_COMMIT|MEM_RESERVE)
    c[0x63] = 0x41;
    c[0x64] = 0xB8;
    c[0x65] = 0x00;
    c[0x66] = 0x30;
    c[0x67] = 0x00;
    c[0x68] = 0x00;
    // 0x69: mov r9d, 4 = 41 B9 04 00 00 00  (PAGE_READWRITE)
    c[0x69] = 0x41;
    c[0x6A] = 0xB9;
    c[0x6B] = 0x04;
    c[0x6C] = 0x00;
    c[0x6D] = 0x00;
    c[0x6E] = 0x00;
    // 0x6f: mov rax, [rip+disp] (iat_base): ends at 0x76, disp = 0x138-0x76 = 0xc2
    c[0x6F] = 0x48;
    c[0x70] = 0x8B;
    c[0x71] = 0x05;
    c[0x72] = 0xC2;
    c[0x73] = 0x00;
    c[0x74] = 0x00;
    c[0x75] = 0x00;
    // 0x76: call [rax+0x18] (VirtualAlloc) = FF 50 18
    c[0x76] = 0xFF;
    c[0x77] = 0x50;
    c[0x78] = 0x18;
    // 0x79: add rsp, 40 = 48 83 C4 28
    c[0x79] = 0x48;
    c[0x7A] = 0x83;
    c[0x7B] = 0xC4;
    c[0x7C] = 0x28;
    // 0x7d: ret = C3
    c[0x7D] = 0xC3;
    // mmap ends at 0x7e

    // === __kr_alloc at 0x7e ===
    // 0x7e: add rdi, 15 = 48 83 C7 0F
    c[0x7E] = 0x48;
    c[0x7F] = 0x83;
    c[0x80] = 0xC7;
    c[0x81] = 0x0F;
    // 0x82: and rdi, -16 = 48 83 E7 F0
    c[0x82] = 0x48;
    c[0x83] = 0x83;
    c[0x84] = 0xE7;
    c[0x85] = 0xF0;
    // 0x86: mov rax, [rip+disp] (heap_remaining=0x128): ends at 0x8d, disp=0x128-0x8d=0x9b
    c[0x86] = 0x48;
    c[0x87] = 0x8B;
    c[0x88] = 0x05;
    c[0x89] = 0x9B;
    c[0x8A] = 0x00;
    c[0x8B] = 0x00;
    c[0x8C] = 0x00;
    // 0x8d: cmp rdi, rax = 48 39 C7
    c[0x8D] = 0x48;
    c[0x8E] = 0x39;
    c[0x8F] = 0xC7;
    // 0x90: jbe .have_space: target = ? We need to count...
    // push(1)+mov(5)+call(5)+mov(7)+movq(11)+pop(1) = 30 = 0x1e bytes from 0x92
    // .have_space = 0x92 + 0x1e = 0xb0
    // jbe +0x1e from next instr (0x92), target 0xb0
    c[0x90] = 0x76;
    c[0x91] = 0x1E;
    // 0x92: push rdi = 57
    c[0x92] = 0x57;
    // 0x93: mov edi, 0x400000 = BF 00 00 40 00
    c[0x93] = 0xBF;
    c[0x94] = 0x00;
    c[0x95] = 0x00;
    c[0x96] = 0x40;
    c[0x97] = 0x00;
    // 0x98: call __kr_mmap_alloc (at 0x5a): ends at 0x9d, disp = 0x5a-0x9d = -0x43
    c[0x98] = 0xE8;
    c[0x99] = 0xBD;
    c[0x9A] = 0xFF;
    c[0x9B] = 0xFF;
    c[0x9C] = 0xFF;
    // 0x9d: mov [rip+disp], rax (heap_ptr=0x120): ends at 0xa4, disp=0x120-0xa4=0x7c
    c[0x9D] = 0x48;
    c[0x9E] = 0x89;
    c[0x9F] = 0x05;
    c[0xA0] = 0x7C;
    c[0xA1] = 0x00;
    c[0xA2] = 0x00;
    c[0xA3] = 0x00;
    // 0xa4: mov qword [rip+disp], 0x400000 (heap_remaining=0x128):
    //   ends at 0xaf, disp=0x128-0xaf=0x79
    c[0xA4] = 0x48;
    c[0xA5] = 0xC7;
    c[0xA6] = 0x05;
    c[0xA7] = 0x79;
    c[0xA8] = 0x00;
    c[0xA9] = 0x00;
    c[0xAA] = 0x00;
    c[0xAB] = 0x00;
    c[0xAC] = 0x00;
    c[0xAD] = 0x40;
    c[0xAE] = 0x00;
    // 0xaf: pop rdi = 5F
    c[0xAF] = 0x5F;
    // .have_space at 0xb0:
    // 0xb0: mov rax, [rip+disp] (heap_ptr=0x120): ends at 0xb7, disp=0x120-0xb7=0x69
    c[0xB0] = 0x48;
    c[0xB1] = 0x8B;
    c[0xB2] = 0x05;
    c[0xB3] = 0x69;
    c[0xB4] = 0x00;
    c[0xB5] = 0x00;
    c[0xB6] = 0x00;
    // 0xb7: add [rip+disp], rdi (heap_ptr=0x120): ends at 0xbe, disp=0x120-0xbe=0x62
    c[0xB7] = 0x48;
    c[0xB8] = 0x01;
    c[0xB9] = 0x3D;
    c[0xBA] = 0x62;
    c[0xBB] = 0x00;
    c[0xBC] = 0x00;
    c[0xBD] = 0x00;
    // 0xbe: sub [rip+disp], rdi (heap_remaining=0x128): ends at 0xc5, disp=0x128-0xc5=0x63
    c[0xBE] = 0x48;
    c[0xBF] = 0x29;
    c[0xC0] = 0x3D;
    c[0xC1] = 0x63;
    c[0xC2] = 0x00;
    c[0xC3] = 0x00;
    c[0xC4] = 0x00;
    // 0xc5: ret = C3
    c[0xC5] = 0xC3;

    // === __kr_dealloc at 0xc6 ===
    c[0xC6] = 0xC3; // ret

    // === __kr_getenv at 0xc7 ===
    // Returns NULL (not implemented on Windows in Phase 3a)
    // xor eax, eax = 31 C0
    c[0xC7] = 0x31;
    c[0xC8] = 0xC0;
    // ret = C3
    c[0xC9] = 0xC3;

    // === __kr_exec at 0xca ===
    // Stub: returns 1 (not implemented in Phase 3a)
    // CreateProcessA requires complex setup (STARTUPINFO, PROCESS_INFORMATION structs).
    // Will be implemented when the PE linker matures.
    // mov eax, 1 = B8 01 00 00 00
    c[0xCA] = 0xB8;
    c[0xCB] = 0x01;
    c[0xCC] = 0x00;
    c[0xCD] = 0x00;
    c[0xCE] = 0x00;
    // ret = C3
    c[0xCF] = 0xC3;

    // === __kr_str_copy at 0xd0 ===
    // mov rax, rdi = 48 89 F8
    c[0xD0] = 0x48;
    c[0xD1] = 0x89;
    c[0xD2] = 0xF8;
    // .copy_loop:
    // movzx ecx, byte [rsi] = 0F B6 0E
    c[0xD3] = 0x0F;
    c[0xD4] = 0xB6;
    c[0xD5] = 0x0E;
    // mov [rdi], cl = 88 0F
    c[0xD6] = 0x88;
    c[0xD7] = 0x0F;
    // test cl, cl = 84 C9
    c[0xD8] = 0x84;
    c[0xD9] = 0xC9;
    // je .done (+0x08) = 74 08
    c[0xDA] = 0x74;
    c[0xDB] = 0x08;
    // inc rdi = 48 FF C7
    c[0xDC] = 0x48;
    c[0xDD] = 0xFF;
    c[0xDE] = 0xC7;
    // inc rsi = 48 FF C6
    c[0xDF] = 0x48;
    c[0xE0] = 0xFF;
    c[0xE1] = 0xC6;
    // jmp .copy_loop = EB EF
    c[0xE2] = 0xEB;
    c[0xE3] = 0xEF;
    // .done: ret = C3
    c[0xE4] = 0xC3;

    // === __kr_str_cat at 0xe5 ===
    // mov rax, rdi = 48 89 F8
    c[0xE5] = 0x48;
    c[0xE6] = 0x89;
    c[0xE7] = 0xF8;
    // .find_end: cmp byte [rdi], 0 = 80 3F 00
    c[0xE8] = 0x80;
    c[0xE9] = 0x3F;
    c[0xEA] = 0x00;
    // je .cat_copy (+0x05) = 74 05
    c[0xEB] = 0x74;
    c[0xEC] = 0x05;
    // inc rdi = 48 FF C7
    c[0xED] = 0x48;
    c[0xEE] = 0xFF;
    c[0xEF] = 0xC7;
    // jmp .find_end = EB F6
    c[0xF0] = 0xEB;
    c[0xF1] = 0xF6;
    // .cat_copy: movzx ecx, byte [rsi] = 0F B6 0E
    c[0xF2] = 0x0F;
    c[0xF3] = 0xB6;
    c[0xF4] = 0x0E;
    // mov [rdi], cl = 88 0F
    c[0xF5] = 0x88;
    c[0xF6] = 0x0F;
    // test cl, cl = 84 C9
    c[0xF7] = 0x84;
    c[0xF8] = 0xC9;
    // je .done (+0x08) = 74 08
    c[0xF9] = 0x74;
    c[0xFA] = 0x08;
    // inc rdi = 48 FF C7
    c[0xFB] = 0x48;
    c[0xFC] = 0xFF;
    c[0xFD] = 0xC7;
    // inc rsi = 48 FF C6
    c[0xFE] = 0x48;
    c[0xFF] = 0xFF;
    c[0x100] = 0xC6;
    // jmp .cat_copy = EB EF
    c[0x101] = 0xEB;
    c[0x102] = 0xEF;
    // .done: ret = C3
    c[0x103] = 0xC3;

    // === __kr_str_len at 0x104 ===
    // xor eax, eax = 31 C0
    c[0x104] = 0x31;
    c[0x105] = 0xC0;
    // .loop: cmp byte [rdi], 0 = 80 3F 00
    c[0x106] = 0x80;
    c[0x107] = 0x3F;
    c[0x108] = 0x00;
    // je .done (+0x08) = 74 08
    c[0x109] = 0x74;
    c[0x10A] = 0x08;
    // inc rdi = 48 FF C7
    c[0x10B] = 0x48;
    c[0x10C] = 0xFF;
    c[0x10D] = 0xC7;
    // inc rax = 48 FF C0
    c[0x10E] = 0x48;
    c[0x10F] = 0xFF;
    c[0x110] = 0xC0;
    // jmp .loop = EB F3
    c[0x111] = 0xEB;
    c[0x112] = 0xF3;
    // .done: ret = C3
    c[0x113] = 0xC3;

    // 0x114 .. 0x117: padding (4 bytes, NOP sled)
    c[0x114] = 0x90;
    c[0x115] = 0x90;
    c[0x116] = 0x90;
    c[0x117] = 0x90;

    // === Data area at 0x118 (40 bytes, all zeros) ===
    // envp           (0x118-0x11f): zero
    // heap_ptr       (0x120-0x127): zero
    // heap_remaining (0x128-0x12f): zero
    // stdout_handle  (0x130-0x137): zero
    // iat_base       (0x138-0x13f): zero (patched by PE linker)

    // === __kr_file_open at 0x140 (stub: returns -1) ===
    // File I/O through Win32 API requires extending the PE import table.
    // For now, return fd=-1 to signal error.
    // mov rax, -1 = 48 C7 C0 FF FF FF FF
    c[0x140] = 0x48;
    c[0x141] = 0xC7;
    c[0x142] = 0xC0;
    c[0x143] = 0xFF;
    c[0x144] = 0xFF;
    c[0x145] = 0xFF;
    c[0x146] = 0xFF;
    // ret = C3
    c[0x147] = 0xC3;

    // === __kr_file_read at 0x148 (stub: returns 0) ===
    // xor eax, eax = 31 C0
    c[0x148] = 0x31;
    c[0x149] = 0xC0;
    // ret = C3
    c[0x14A] = 0xC3;

    // === __kr_file_write at 0x14b (stub: returns 0) ===
    // xor eax, eax = 31 C0
    c[0x14B] = 0x31;
    c[0x14C] = 0xC0;
    // ret = C3
    c[0x14D] = 0xC3;

    // === __kr_file_close at 0x14e (stub: no-op) ===
    // ret = C3
    c[0x14E] = 0xC3;

    // === __kr_file_size at 0x14f (stub: returns 0) ===
    // xor eax, eax = 31 C0
    c[0x14F] = 0x31;
    c[0x150] = 0xC0;
    // ret = C3
    c[0x151] = 0xC3;

    // === __kr_fmt_uint at 0x152 ===
    // Identical to Linux x86_64 (pure computation, no syscalls).
    // push rbx
    c[0x152] = 0x53;
    // push r12
    c[0x153] = 0x41;
    c[0x154] = 0x54;
    // mov rbx, rdi
    c[0x155] = 0x48;
    c[0x156] = 0x89;
    c[0x157] = 0xFB;
    // mov rax, rsi
    c[0x158] = 0x48;
    c[0x159] = 0x89;
    c[0x15A] = 0xF0;
    // mov r12, rdi
    c[0x15B] = 0x49;
    c[0x15C] = 0x89;
    c[0x15D] = 0xFC;
    // test rax, rax
    c[0x15E] = 0x48;
    c[0x15F] = 0x85;
    c[0x160] = 0xC0;
    // jne .div_loop (+0x0e)
    c[0x161] = 0x75;
    c[0x162] = 0x0E;
    // mov byte [rbx], '0'
    c[0x163] = 0xC6;
    c[0x164] = 0x03;
    c[0x165] = 0x30;
    // mov rax, 1
    c[0x166] = 0x48;
    c[0x167] = 0xC7;
    c[0x168] = 0xC0;
    c[0x169] = 0x01;
    c[0x16A] = 0x00;
    c[0x16B] = 0x00;
    c[0x16C] = 0x00;
    // pop r12
    c[0x16D] = 0x41;
    c[0x16E] = 0x5C;
    // pop rbx
    c[0x16F] = 0x5B;
    // ret
    c[0x170] = 0xC3;
    // .div_loop: xor ecx, ecx
    c[0x171] = 0x31;
    c[0x172] = 0xC9;
    // .loop_body: test rax, rax
    c[0x173] = 0x48;
    c[0x174] = 0x85;
    c[0x175] = 0xC0;
    // jz .reverse (+0x14)
    c[0x176] = 0x74;
    c[0x177] = 0x14;
    // xor edx, edx
    c[0x178] = 0x31;
    c[0x179] = 0xD2;
    // mov r8, 10
    c[0x17A] = 0x49;
    c[0x17B] = 0xC7;
    c[0x17C] = 0xC0;
    c[0x17D] = 0x0A;
    c[0x17E] = 0x00;
    c[0x17F] = 0x00;
    c[0x180] = 0x00;
    // div r8
    c[0x181] = 0x49;
    c[0x182] = 0xF7;
    c[0x183] = 0xF0;
    // add dl, '0'
    c[0x184] = 0x80;
    c[0x185] = 0xC2;
    c[0x186] = 0x30;
    // push rdx
    c[0x187] = 0x52;
    // inc ecx
    c[0x188] = 0xFF;
    c[0x189] = 0xC1;
    // jmp .loop_body (-0x19)
    c[0x18A] = 0xEB;
    c[0x18B] = 0xE7;
    // .reverse: mov rdi, r12
    c[0x18C] = 0x4C;
    c[0x18D] = 0x89;
    c[0x18E] = 0xE7;
    // .rev_loop: test ecx, ecx
    c[0x18F] = 0x85;
    c[0x190] = 0xC9;
    // jz .done (+0x0a)
    c[0x191] = 0x74;
    c[0x192] = 0x0A;
    // pop rax
    c[0x193] = 0x58;
    // mov [rdi], al
    c[0x194] = 0x88;
    c[0x195] = 0x07;
    // inc rdi
    c[0x196] = 0x48;
    c[0x197] = 0xFF;
    c[0x198] = 0xC7;
    // dec ecx
    c[0x199] = 0xFF;
    c[0x19A] = 0xC9;
    // jmp .rev_loop (-0x0e)
    c[0x19B] = 0xEB;
    c[0x19C] = 0xF2;
    // .done: mov rax, rdi
    c[0x19D] = 0x48;
    c[0x19E] = 0x89;
    c[0x19F] = 0xF8;
    // sub rax, r12
    c[0x1A0] = 0x4C;
    c[0x1A1] = 0x29;
    c[0x1A2] = 0xE0;
    // pop r12
    c[0x1A3] = 0x41;
    c[0x1A4] = 0x5C;
    // pop rbx
    c[0x1A5] = 0x5B;
    // ret
    c[0x1A6] = 0xC3;

    // === __kr_fmt_int at 0x1a7 ===
    // test esi, esi
    c[0x1A7] = 0x85;
    c[0x1A8] = 0xF6;
    // js .negative (+0x02)
    c[0x1A9] = 0x78;
    c[0x1AA] = 0x02;
    // jmp __kr_fmt_uint (rel8 -0x5b)
    c[0x1AB] = 0xEB;
    c[0x1AC] = 0xA5;
    // .negative: mov byte [rdi], '-'
    c[0x1AD] = 0xC6;
    c[0x1AE] = 0x07;
    c[0x1AF] = 0x2D;
    // neg esi
    c[0x1B0] = 0xF7;
    c[0x1B1] = 0xDE;
    // mov esi, esi (zero-extend)
    c[0x1B2] = 0x89;
    c[0x1B3] = 0xF6;
    // inc rdi
    c[0x1B4] = 0x48;
    c[0x1B5] = 0xFF;
    c[0x1B6] = 0xC7;
    // call __kr_fmt_uint (rel32 = -0x6a)
    c[0x1B7] = 0xE8;
    c[0x1B8] = 0x96;
    c[0x1B9] = 0xFF;
    c[0x1BA] = 0xFF;
    c[0x1BB] = 0xFF;
    // inc rax
    c[0x1BC] = 0x48;
    c[0x1BD] = 0xFF;
    c[0x1BE] = 0xC0;
    // ret
    c[0x1BF] = 0xC3;

    // === __kr_fmt_hex at 0x1c0 ===
    // push rbx
    c[0x1C0] = 0x53;
    // push r12
    c[0x1C1] = 0x41;
    c[0x1C2] = 0x54;
    // mov rbx, rdi
    c[0x1C3] = 0x48;
    c[0x1C4] = 0x89;
    c[0x1C5] = 0xFB;
    // mov rax, rsi
    c[0x1C6] = 0x48;
    c[0x1C7] = 0x89;
    c[0x1C8] = 0xF0;
    // mov r12, rdi
    c[0x1C9] = 0x49;
    c[0x1CA] = 0x89;
    c[0x1CB] = 0xFC;
    // test rax, rax
    c[0x1CC] = 0x48;
    c[0x1CD] = 0x85;
    c[0x1CE] = 0xC0;
    // jne .hex_loop (+0x0e)
    c[0x1CF] = 0x75;
    c[0x1D0] = 0x0E;
    // mov byte [rbx], '0'
    c[0x1D1] = 0xC6;
    c[0x1D2] = 0x03;
    c[0x1D3] = 0x30;
    // mov rax, 1
    c[0x1D4] = 0x48;
    c[0x1D5] = 0xC7;
    c[0x1D6] = 0xC0;
    c[0x1D7] = 0x01;
    c[0x1D8] = 0x00;
    c[0x1D9] = 0x00;
    c[0x1DA] = 0x00;
    // pop r12
    c[0x1DB] = 0x41;
    c[0x1DC] = 0x5C;
    // pop rbx
    c[0x1DD] = 0x5B;
    // ret
    c[0x1DE] = 0xC3;
    // .hex_loop: xor ecx, ecx
    c[0x1DF] = 0x31;
    c[0x1E0] = 0xC9;
    // .hloop: test rax, rax
    c[0x1E1] = 0x48;
    c[0x1E2] = 0x85;
    c[0x1E3] = 0xC0;
    // jz .hreverse (+0x1c)
    c[0x1E4] = 0x74;
    c[0x1E5] = 0x1C;
    // mov rdx, rax
    c[0x1E6] = 0x48;
    c[0x1E7] = 0x89;
    c[0x1E8] = 0xC2;
    // and edx, 0xf
    c[0x1E9] = 0x83;
    c[0x1EA] = 0xE2;
    c[0x1EB] = 0x0F;
    // shr rax, 4
    c[0x1EC] = 0x48;
    c[0x1ED] = 0xC1;
    c[0x1EE] = 0xE8;
    c[0x1EF] = 0x04;
    // cmp dl, 10
    c[0x1F0] = 0x80;
    c[0x1F1] = 0xFA;
    c[0x1F2] = 0x0A;
    // jb .digit (+0x05)
    c[0x1F3] = 0x72;
    c[0x1F4] = 0x05;
    // add dl, 0x57
    c[0x1F5] = 0x80;
    c[0x1F6] = 0xC2;
    c[0x1F7] = 0x57;
    // jmp .hpush (+0x03)
    c[0x1F8] = 0xEB;
    c[0x1F9] = 0x03;
    // .digit: add dl, '0'
    c[0x1FA] = 0x80;
    c[0x1FB] = 0xC2;
    c[0x1FC] = 0x30;
    // .hpush: push rdx
    c[0x1FD] = 0x52;
    // inc ecx
    c[0x1FE] = 0xFF;
    c[0x1FF] = 0xC1;
    // jmp .hloop (-0x21)
    c[0x200] = 0xEB;
    c[0x201] = 0xDF;
    // .hreverse: mov rdi, r12
    c[0x202] = 0x4C;
    c[0x203] = 0x89;
    c[0x204] = 0xE7;
    // .hrev: test ecx, ecx
    c[0x205] = 0x85;
    c[0x206] = 0xC9;
    // jz .hdone (+0x0a)
    c[0x207] = 0x74;
    c[0x208] = 0x0A;
    // pop rax
    c[0x209] = 0x58;
    // mov [rdi], al
    c[0x20A] = 0x88;
    c[0x20B] = 0x07;
    // inc rdi
    c[0x20C] = 0x48;
    c[0x20D] = 0xFF;
    c[0x20E] = 0xC7;
    // dec ecx
    c[0x20F] = 0xFF;
    c[0x210] = 0xC9;
    // jmp .hrev (-0x0e)
    c[0x211] = 0xEB;
    c[0x212] = 0xF2;
    // .hdone: mov rax, rdi
    c[0x213] = 0x48;
    c[0x214] = 0x89;
    c[0x215] = 0xF8;
    // sub rax, r12
    c[0x216] = 0x4C;
    c[0x217] = 0x29;
    c[0x218] = 0xE0;
    // pop r12
    c[0x219] = 0x41;
    c[0x21A] = 0x5C;
    // pop rbx
    c[0x21B] = 0x5B;
    // ret
    c[0x21C] = 0xC3;

    c
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_size_is_correct() {
        assert_eq!(BLOB.code.len(), 541);
    }

    #[test]
    fn all_symbols_within_bounds() {
        assert_eq!(BLOB.symbols.len(), 19);
        for &(name, offset) in BLOB.symbols {
            assert!(
                (offset as usize) < BLOB.code.len(),
                "symbol {name} offset 0x{offset:X} out of bounds",
            );
        }
    }

    #[test]
    fn main_call_fixup_within_bounds() {
        assert!((BLOB.main_call_fixup as usize + 4) <= BLOB.code.len());
    }

    #[test]
    fn call_main_opcode_is_e8() {
        let fixup = BLOB.main_call_fixup as usize;
        assert!(fixup >= 1);
        assert_eq!(BLOB.code[fixup - 1], 0xE8);
    }

    #[test]
    fn data_area_is_zeroed() {
        // 40-byte data area at 0x118 (no longer at end of blob)
        let data_start = 0x118;
        for i in data_start..data_start + 40 {
            assert_eq!(
                BLOB.code[i], 0x00,
                "data area byte at offset 0x{i:X} is 0x{:02X}",
                BLOB.code[i]
            );
        }
    }

    #[test]
    fn dealloc_is_just_ret() {
        let off = BLOB.symbol_offset("__kr_dealloc").unwrap() as usize;
        assert_eq!(BLOB.code[off], 0xC3);
    }

    #[test]
    fn getenv_returns_null() {
        // getenv should be xor eax,eax; ret = 31 C0 C3
        let off = BLOB.symbol_offset("__kr_getenv").unwrap() as usize;
        assert_eq!(BLOB.code[off], 0x31);
        assert_eq!(BLOB.code[off + 1], 0xC0);
        assert_eq!(BLOB.code[off + 2], 0xC3);
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
}
