//! macOS x86_64 runtime blob — hand-assembled machine code.
//! Implements `_start` and all `__kr_*` functions using macOS BSD syscalls.
//!
//! Layout (704 bytes total):
//!   0x000 .. 0x182  executable code (11 original functions)
//!   0x183 .. 0x18d  inline string data ("/bin/sh\0", "-c\0")
//!   0x18e .. 0x18f  padding (2 bytes)
//!   0x190 .. 0x1a7  data area: envp(8) + heap_ptr(8) + heap_remaining(8)
//!   0x1a8 .. 0x1ce  executable code (5 file I/O functions)
//!   0x1cf .. 0x2cf  executable code (3 string formatting functions)
//!
//! macOS x86_64 syscall convention:
//!   syscall nr in rax (BSD layer 0x2000000), args in rdi, rsi, rdx, r10, r8, r9.
//!   Return in rax, carry flag set on error.
//!
//! _start: dyld calls entry as C function: rdi=argc, rsi=argv, rdx=envp.
//!
//! Syscall numbers: exit=0x2000001, fork=0x2000002, write=0x2000004,
//!   waitpid=0x2000007, execve=0x200003B, mmap=0x20000C5
//!
//! MAP_PRIVATE|MAP_ANON = 0x1002

use super::RuntimeBlob;

// Data area: envp=0x190, heap_ptr=0x198, heap_remaining=0x1a0

pub static BLOB: RuntimeBlob = RuntimeBlob {
    code: &[
        // === _start (offset 0x00) ===
        // dyld calls us: rdi=argc, rsi=argv, rdx=envp
        // mov [rip+0x189], rdx           ; save envp -> 0x190
        0x48, 0x89, 0x15, 0x89, 0x01, 0x00, 0x00,
        // call main (rel32 placeholder)  ; FIXUP at offset 0x08
        0xe8, 0x00, 0x00, 0x00, 0x00, // mov edi, eax                   ; exit code
        0x89, 0xc7,
        // === __kr_exit (offset 0x0e) ===
        // mov eax, 0x2000001             ; SYS_exit
        0xb8, 0x01, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05,
        // === __kr_write (offset 0x15) ===
        // mov eax, 0x2000004             ; SYS_write
        0xb8, 0x04, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_mmap_alloc (offset 0x1d) ===
        // mov rsi, rdi                   ; len = size
        0x48, 0x89, 0xfe, // xor edi, edi                   ; addr = NULL
        0x31, 0xff, // mov edx, 3                     ; PROT_READ|PROT_WRITE
        0xba, 0x03, 0x00, 0x00, 0x00,
        // mov r10d, 0x1002               ; MAP_PRIVATE|MAP_ANON (macOS)
        0x41, 0xba, 0x02, 0x10, 0x00, 0x00, // mov r8, -1                     ; fd = -1
        0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff,
        // xor r9d, r9d                   ; offset = 0
        0x45, 0x31, 0xc9, // mov eax, 0x20000C5             ; SYS_mmap
        0xb8, 0xc5, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_alloc (offset 0x3f) ===
        // add rdi, 15                    ; align to 16
        0x48, 0x83, 0xc7, 0x0f, // and rdi, -16
        0x48, 0x83, 0xe7, 0xf0,
        // mov rax, [rip+0x152]           ; heap_remaining (0x1a0 - 0x4e)
        0x48, 0x8b, 0x05, 0x52, 0x01, 0x00, 0x00, // cmp rdi, rax
        0x48, 0x39, 0xc7, // jbe .have_space (+0x1e)
        0x76, 0x1e, // push rdi                       ; save aligned size
        0x57, // mov edi, 0x400000              ; 4MB
        0xbf, 0x00, 0x00, 0x40, 0x00,
        // call __kr_mmap_alloc (rel32)   ; 0x1d - 0x5e = -0x41
        0xe8, 0xbf, 0xff, 0xff, 0xff,
        // mov [rip+0x133], rax           ; heap_ptr (0x198 - 0x65)
        0x48, 0x89, 0x05, 0x33, 0x01, 0x00, 0x00,
        // mov qword [rip+0x130], 0x400000 ; heap_remaining (0x1a0 - 0x70)
        0x48, 0xc7, 0x05, 0x30, 0x01, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, // pop rdi
        0x5f,
        // .have_space:
        // mov rax, [rip+0x120]           ; heap_ptr (0x198 - 0x78)
        0x48, 0x8b, 0x05, 0x20, 0x01, 0x00, 0x00,
        // add [rip+0x119], rdi           ; heap_ptr (0x198 - 0x7f)
        0x48, 0x01, 0x3d, 0x19, 0x01, 0x00, 0x00,
        // sub [rip+0x11a], rdi           ; heap_remaining (0x1a0 - 0x86)
        0x48, 0x29, 0x3d, 0x1a, 0x01, 0x00, 0x00, // ret
        0xc3, // === __kr_dealloc (offset 0x87) ===
        // ret
        0xc3,
        // === __kr_getenv (offset 0x88) ===
        // mov rsi, [rip+0x101]           ; envp (0x190 - 0x8f)
        0x48, 0x8b, 0x35, 0x01, 0x01, 0x00, 0x00, // .env_loop:
        // mov rdx, [rsi]
        0x48, 0x8b, 0x16, // test rdx, rdx
        0x48, 0x85, 0xd2, // je .env_not_found (+0x2b)
        0x74, 0x2b, // mov rcx, rdi
        0x48, 0x89, 0xf9, // mov r8, rdx
        0x49, 0x89, 0xd0, // .env_cmp:
        // movzx eax, byte [rcx]
        0x0f, 0xb6, 0x01, // test al, al
        0x84, 0xc0, // je .env_check_eq (+0x0d)
        0x74, 0x0d, // cmp al, [r8]
        0x41, 0x3a, 0x00, // jne .env_next (+0x13)
        0x75, 0x13, // inc rcx
        0x48, 0xff, 0xc1, // inc r8
        0x49, 0xff, 0xc0, // jmp .env_cmp (-0x14)
        0xeb, 0xec, // .env_check_eq:
        // cmp byte [r8], 0x3d
        0x41, 0x80, 0x38, 0x3d, // jne .env_next (+0x05)
        0x75, 0x05, // lea rax, [r8+1]
        0x49, 0x8d, 0x40, 0x01, // ret
        0xc3, // .env_next:
        // add rsi, 8
        0x48, 0x83, 0xc6, 0x08, // jmp .env_loop (-0x33)
        0xeb, 0xcd, // .env_not_found:
        // xor eax, eax
        0x31, 0xc0, // ret
        0xc3, // === __kr_exec (offset 0xc5) ===
        // push rbx
        0x53, // push r12
        0x41, 0x54, // mov rbx, rdi                   ; save cmd
        0x48, 0x89, 0xfb, // mov eax, 0x2000002             ; SYS_fork
        0xb8, 0x02, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // test eax, eax
        0x85, 0xc0, // je .child (+0x38)
        0x74, 0x38, // mov r12d, eax                  ; save pid
        0x41, 0x89, 0xc4, // sub rsp, 8                     ; status on stack
        0x48, 0x83, 0xec, 0x08, // mov edi, r12d                  ; pid
        0x44, 0x89, 0xe7, // lea rsi, [rsp]                 ; &status
        0x48, 0x8d, 0x34, 0x24, // xor edx, edx                   ; options = 0
        0x31, 0xd2, // xor r10d, r10d                 ; rusage = NULL
        0x45, 0x31, 0xd2, // mov eax, 0x2000007             ; SYS_wait4
        0xb8, 0x07, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // mov eax, [rsp]                 ; status
        0x8b, 0x04, 0x24, // add rsp, 8
        0x48, 0x83, 0xc4, 0x08, // test al, al
        0x84, 0xc0, // jne .signal_death (+0x0a)
        0x75, 0x0a, // shr eax, 8
        0xc1, 0xe8, 0x08, // and eax, 0xff
        0x25, 0xff, 0x00, 0x00, 0x00, // jmp .exec_done (+0x05)
        0xeb, 0x05, // .signal_death:
        // mov eax, 1
        0xb8, 0x01, 0x00, 0x00, 0x00, // .exec_done:
        // pop r12
        0x41, 0x5c, // pop rbx
        0x5b, // ret
        0xc3,
        // .child:
        // lea rdi, [rip+0x6e]            ; "/bin/sh" (0x183 - 0x115)
        0x48, 0x8d, 0x3d, 0x6e, 0x00, 0x00, 0x00, // xor eax, eax
        0x31, 0xc0, // push rax                       ; NULL
        0x50, // push rbx                       ; cmd
        0x53, // lea rax, [rip+0x6b]            ; "-c" (0x18b - 0x120)
        0x48, 0x8d, 0x05, 0x6b, 0x00, 0x00, 0x00, // push rax
        0x50, // push rdi                       ; "/bin/sh"
        0x57, // mov rsi, rsp                   ; argv
        0x48, 0x89, 0xe6, // mov rdx, [rip+0x64]            ; envp (0x190 - 0x12c)
        0x48, 0x8b, 0x15, 0x64, 0x00, 0x00, 0x00,
        // mov eax, 0x200003B             ; SYS_execve
        0xb8, 0x3b, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // mov edi, 127                   ; exit 127 on failure
        0xbf, 0x7f, 0x00, 0x00, 0x00, // mov eax, 0x2000001             ; SYS_exit
        0xb8, 0x01, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // === __kr_str_copy (offset 0x13f) ===
        // mov rax, rdi
        0x48, 0x89, 0xf8, // .copy_loop:
        // movzx ecx, byte [rsi]
        0x0f, 0xb6, 0x0e, // mov [rdi], cl
        0x88, 0x0f, // test cl, cl
        0x84, 0xc9, // je .copy_done (+0x08)
        0x74, 0x08, // inc rdi
        0x48, 0xff, 0xc7, // inc rsi
        0x48, 0xff, 0xc6, // jmp .copy_loop (-0x11)
        0xeb, 0xef, // .copy_done:
        // ret
        0xc3, // === __kr_str_cat (offset 0x154) ===
        // mov rax, rdi
        0x48, 0x89, 0xf8, // .cat_find_end:
        // cmp byte [rdi], 0
        0x80, 0x3f, 0x00, // je .cat_copy (+0x05)
        0x74, 0x05, // inc rdi
        0x48, 0xff, 0xc7, // jmp .cat_find_end (-0x0a)
        0xeb, 0xf6, // .cat_copy:
        // movzx ecx, byte [rsi]
        0x0f, 0xb6, 0x0e, // mov [rdi], cl
        0x88, 0x0f, // test cl, cl
        0x84, 0xc9, // je .cat_done (+0x08)
        0x74, 0x08, // inc rdi
        0x48, 0xff, 0xc7, // inc rsi
        0x48, 0xff, 0xc6, // jmp .cat_copy (-0x11)
        0xeb, 0xef, // .cat_done:
        // ret
        0xc3, // === __kr_str_len (offset 0x173) ===
        // xor eax, eax
        0x31, 0xc0, // .len_loop:
        // cmp byte [rdi], 0
        0x80, 0x3f, 0x00, // je .len_done (+0x08)
        0x74, 0x08, // inc rdi
        0x48, 0xff, 0xc7, // inc rax
        0x48, 0xff, 0xc0, // jmp .len_loop (-0x0d)
        0xeb, 0xf3, // .len_done:
        // ret
        0xc3,
        // === Inline string data ===
        // .sh_path (offset 0x183): "/bin/sh\0"
        0x2f, 0x62, 0x69, 0x6e, 0x2f, 0x73, 0x68, 0x00, // .dash_c (offset 0x18b): "-c\0"
        0x2d, 0x63, 0x00, // === Padding (offset 0x18e, 2 bytes) ===
        0x90, 0x90,
        // === Data area (offset 0x190, 24 bytes) ===
        // envp (8 bytes)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // heap_ptr (8 bytes)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // heap_remaining (8 bytes)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // === __kr_file_open (offset 0x1a8) ===
        // args: rdi=path, rsi=flags (0=rdonly, 1=wronly+creat+trunc, 2=rdwr)
        // cmp esi, 1
        0x83, 0xfe, 0x01, // jne .do_open (+5)
        0x75, 0x05, // mov esi, 0x241               ; O_WRONLY|O_CREAT|O_TRUNC
        0xbe, 0x41, 0x02, 0x00, 0x00,
        // .do_open:
        // mov edx, 0x1a4               ; mode = 0644
        0xba, 0xa4, 0x01, 0x00, 0x00, // mov eax, 0x2000005           ; SYS_open
        0xb8, 0x05, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_file_read (offset 0x1bf) ===
        // args: rdi=fd, rsi=buf, rdx=count
        // mov eax, 0x2000003           ; SYS_read
        0xb8, 0x03, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_file_write (offset 0x1c7) ===
        // args: rdi=fd, rsi=buf, rdx=count
        // mov eax, 0x2000004           ; SYS_write
        0xb8, 0x04, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_file_close (offset 0x1cf) ===
        // args: rdi=fd
        // mov eax, 0x2000006           ; SYS_close
        0xb8, 0x06, 0x00, 0x00, 0x02, // syscall
        0x0f, 0x05, // ret
        0xc3,
        // === __kr_file_size (offset 0x1d7) ===
        // args: rdi=fd
        // sub rsp, 144                 ; allocate struct stat on stack
        0x48, 0x81, 0xec, 0x90, 0x00, 0x00, 0x00,
        // mov rsi, rsp                 ; &stat_buf
        0x48, 0x89, 0xe6, // mov eax, 0x2000153           ; SYS_fstat64
        0xb8, 0x53, 0x01, 0x00, 0x02, // syscall
        0x0f, 0x05, // mov rax, [rsp + 96]          ; st_size (offset 96 = 0x60 on macOS)
        0x48, 0x8b, 0x44, 0x24, 0x60, // add rsp, 144
        0x48, 0x81, 0xc4, 0x90, 0x00, 0x00, 0x00, // ret
        0xc3,
        // === __kr_fmt_uint (offset 0x1f5) ===
        // Convert uint64 to decimal string. rdi=buf, rsi=value, returns length in rax.
        // push rbx
        0x53, // push r12
        0x41, 0x54, // mov rbx, rdi                   ; save buf
        0x48, 0x89, 0xfb, // mov rax, rsi                   ; value
        0x48, 0x89, 0xf0, // mov r12, rdi                   ; save buf start
        0x49, 0x89, 0xfc, // test rax, rax                  ; if value == 0
        0x48, 0x85, 0xc0, // jne .div_loop (+0x0e)
        0x75, 0x0e, // mov byte [rbx], '0'
        0xc6, 0x03, 0x30, // mov rax, 1                     ; return 1
        0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // pop r12
        0x41, 0x5c, // pop rbx
        0x5b, // ret
        0xc3, // .div_loop:
        // xor ecx, ecx                  ; digit count = 0
        0x31, 0xc9, // .loop_body:
        // test rax, rax
        0x48, 0x85, 0xc0, // jz .reverse (+0x14)
        0x74, 0x14, // xor edx, edx
        0x31, 0xd2, // mov r8, 10
        0x49, 0xc7, 0xc0, 0x0a, 0x00, 0x00, 0x00,
        // div r8                         ; rax=quotient, rdx=remainder
        0x49, 0xf7, 0xf0, // add dl, '0'
        0x80, 0xc2, 0x30, // push rdx
        0x52, // inc ecx
        0xff, 0xc1, // jmp .loop_body (-0x19)
        0xeb, 0xe7, // .reverse:
        // mov rdi, r12
        0x4c, 0x89, 0xe7, // .rev_loop:
        // test ecx, ecx
        0x85, 0xc9, // jz .done (+0x0a)
        0x74, 0x0a, // pop rax
        0x58, // mov [rdi], al
        0x88, 0x07, // inc rdi
        0x48, 0xff, 0xc7, // dec ecx
        0xff, 0xc9, // jmp .rev_loop (-0x0e)
        0xeb, 0xf2, // .done:
        // mov rax, rdi
        0x48, 0x89, 0xf8, // sub rax, r12                   ; length = end - start
        0x4c, 0x29, 0xe0, // pop r12
        0x41, 0x5c, // pop rbx
        0x5b, // ret
        0xc3,
        // === __kr_fmt_int (offset 0x24a) ===
        // Convert int32 to decimal string. rdi=buf, rsi=int32 value, returns length.
        // test esi, esi
        0x85, 0xf6, // js .negative (+0x02)
        0x78, 0x02, // jmp __kr_fmt_uint (rel8 -0x5b)
        0xeb, 0xa5, // .negative:
        // mov byte [rdi], '-'
        0xc6, 0x07, 0x2d, // neg esi
        0xf7, 0xde, // mov esi, esi                   ; zero-extend to rsi
        0x89, 0xf6, // inc rdi                        ; advance past '-'
        0x48, 0xff, 0xc7, // call __kr_fmt_uint (rel32 = -0x6a)
        0xe8, 0x96, 0xff, 0xff, 0xff, // inc rax                        ; +1 for '-'
        0x48, 0xff, 0xc0, // ret
        0xc3,
        // === __kr_fmt_hex (offset 0x263) ===
        // Convert uint64 to hex string (no 0x prefix). rdi=buf, rsi=value, returns length.
        // push rbx
        0x53, // push r12
        0x41, 0x54, // mov rbx, rdi
        0x48, 0x89, 0xfb, // mov rax, rsi
        0x48, 0x89, 0xf0, // mov r12, rdi
        0x49, 0x89, 0xfc, // test rax, rax
        0x48, 0x85, 0xc0, // jne .hex_loop (+0x0e)
        0x75, 0x0e, // mov byte [rbx], '0'
        0xc6, 0x03, 0x30, // mov rax, 1
        0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // pop r12
        0x41, 0x5c, // pop rbx
        0x5b, // ret
        0xc3, // .hex_loop:
        // xor ecx, ecx
        0x31, 0xc9, // .hloop:
        // test rax, rax
        0x48, 0x85, 0xc0, // jz .hreverse (+0x1c)
        0x74, 0x1c, // mov rdx, rax
        0x48, 0x89, 0xc2, // and edx, 0xf
        0x83, 0xe2, 0x0f, // shr rax, 4
        0x48, 0xc1, 0xe8, 0x04, // cmp dl, 10
        0x80, 0xfa, 0x0a, // jb .digit (+0x05)
        0x72, 0x05, // add dl, 0x57                   ; 'a' - 10
        0x80, 0xc2, 0x57, // jmp .hpush (+0x03)
        0xeb, 0x03, // .digit:
        // add dl, '0'
        0x80, 0xc2, 0x30, // .hpush:
        // push rdx
        0x52, // inc ecx
        0xff, 0xc1, // jmp .hloop (-0x21)
        0xeb, 0xdf, // .hreverse:
        // mov rdi, r12
        0x4c, 0x89, 0xe7, // .hrev:
        // test ecx, ecx
        0x85, 0xc9, // jz .hdone (+0x0a)
        0x74, 0x0a, // pop rax
        0x58, // mov [rdi], al
        0x88, 0x07, // inc rdi
        0x48, 0xff, 0xc7, // dec ecx
        0xff, 0xc9, // jmp .hrev (-0x0e)
        0xeb, 0xf2, // .hdone:
        // mov rax, rdi
        0x48, 0x89, 0xf8, // sub rax, r12
        0x4c, 0x29, 0xe0, // pop r12
        0x41, 0x5c, // pop rbx
        0x5b, // ret
        0xc3,
    ],
    symbols: &[
        ("_start", 0x00),
        ("__kr_exit", 0x0e),
        ("__kr_write", 0x15),
        ("__kr_mmap_alloc", 0x1d),
        ("__kr_alloc", 0x3f),
        ("__kr_dealloc", 0x87),
        ("__kr_getenv", 0x88),
        ("__kr_exec", 0xc5),
        ("__kr_str_copy", 0x13f),
        ("__kr_str_cat", 0x154),
        ("__kr_str_len", 0x173),
        ("__kr_file_open", 0x1a8),
        ("__kr_file_read", 0x1bf),
        ("__kr_file_write", 0x1c7),
        ("__kr_file_close", 0x1cf),
        ("__kr_file_size", 0x1d7),
        ("__kr_fmt_uint", 0x1f5),
        ("__kr_fmt_int", 0x24a),
        ("__kr_fmt_hex", 0x263),
    ],
    // `call main` at offset 0x07, rel32 at offset 0x08.
    main_call_fixup: 0x08,
    iat_base_data_offset: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_size_is_correct() {
        assert_eq!(BLOB.code.len(), 704);
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
        assert!(
            (BLOB.main_call_fixup as usize + 4) <= BLOB.code.len(),
            "main_call_fixup + 4 = {} exceeds blob len {}",
            BLOB.main_call_fixup as usize + 4,
            BLOB.code.len()
        );
    }

    #[test]
    fn call_main_opcode_is_e8() {
        let fixup = BLOB.main_call_fixup as usize;
        assert!(fixup >= 1);
        assert_eq!(
            BLOB.code[fixup - 1],
            0xE8,
            "expected 0xE8 at offset {}, found 0x{:02X}",
            fixup - 1,
            BLOB.code[fixup - 1]
        );
    }

    #[test]
    fn data_area_is_zeroed() {
        // Data area at 0x190, 24 bytes (no longer at end of blob)
        let data_start = 0x190;
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
        assert_eq!(BLOB.symbol_offset("__kr_exit").unwrap(), 0x0e);
    }

    #[test]
    fn dealloc_is_just_ret() {
        let off = BLOB.symbol_offset("__kr_dealloc").unwrap() as usize;
        assert_eq!(BLOB.code[off], 0xC3);
    }

    #[test]
    fn inline_strings_present() {
        assert_eq!(&BLOB.code[0x183..0x18b], b"/bin/sh\0");
        assert_eq!(&BLOB.code[0x18b..0x18e], b"-c\0");
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
    fn macos_syscall_numbers() {
        let exit_off = BLOB.symbol_offset("__kr_exit").unwrap() as usize;
        assert_eq!(BLOB.code[exit_off], 0xB8);
        assert_eq!(
            u32::from_le_bytes([
                BLOB.code[exit_off + 1],
                BLOB.code[exit_off + 2],
                BLOB.code[exit_off + 3],
                BLOB.code[exit_off + 4],
            ]),
            0x2000001
        );
    }

    #[test]
    fn macos_mmap_flags() {
        let mmap_off = BLOB.symbol_offset("__kr_mmap_alloc").unwrap() as usize;
        // mov r10d, 0x1002 at offset mmap+10: 41 BA 02 10 00 00
        // (after mov rsi,rdi=3 + xor edi,edi=2 + mov edx,3=5 = 10 bytes)
        assert_eq!(BLOB.code[mmap_off + 10], 0x41);
        assert_eq!(BLOB.code[mmap_off + 11], 0xBA);
        assert_eq!(
            u32::from_le_bytes([
                BLOB.code[mmap_off + 12],
                BLOB.code[mmap_off + 13],
                BLOB.code[mmap_off + 14],
                BLOB.code[mmap_off + 15],
            ]),
            0x1002
        );
    }
}
