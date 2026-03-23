use std::fs;

pub fn run_krbo_file(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read '{}': {}", path, e))?;

    // Fat-first: fat magic starts with "KRBO" so check 8 bytes BEFORE 4-byte KRBO check.
    let fat_extracted: Vec<u8>;
    let krbo_bytes: &[u8] = if bytes.len() >= 8 && bytes[0..8] == krir::KRBO_FAT_MAGIC {
        // Fat binary — extract slice for this host architecture.
        #[cfg(target_arch = "x86_64")]
        let host_arch = krir::KRBO_FAT_ARCH_X86_64;
        #[cfg(target_arch = "aarch64")]
        let host_arch = krir::KRBO_FAT_ARCH_AARCH64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        compile_error!("kernrift: unsupported host architecture");

        fat_extracted = krir::parse_krbofat_slice(&bytes, host_arch, Some(path))?;
        &fat_extracted
    } else {
        // Not a fat binary — use bytes directly (single-arch path, unchanged).
        &bytes
    };

    let header = krir::parse_krbo_header(krbo_bytes)?;
    let code_start = 16usize;
    let code_end = code_start + header.code_length as usize;
    if code_end > krbo_bytes.len() {
        return Err("malformed .krbo: file truncated".to_string());
    }
    let code = &krbo_bytes[code_start..code_end];

    let uart_ptr = map_uart_buffer()?;
    let code_ptr = map_executable(code)?;

    unsafe {
        #[cfg(target_arch = "x86_64")]
        let entry_fn: unsafe extern "sysv64" fn() =
            std::mem::transmute(code_ptr.add(header.entry_offset as usize));
        #[cfg(target_arch = "aarch64")]
        let entry_fn: unsafe extern "C" fn() =
            std::mem::transmute(code_ptr.add(header.entry_offset as usize));
        entry_fn();
    }

    flush_uart(uart_ptr, 0x1000);
    Ok(())
}

#[cfg(unix)]
fn map_uart_buffer() -> Result<*mut u8, String> {
    use libc::*;
    #[cfg(target_os = "linux")]
    let flags = MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS;
    #[cfg(not(target_os = "linux"))]
    let flags = MAP_PRIVATE | MAP_FIXED | MAP_ANON;

    let ptr = unsafe {
        mmap(
            0x10000000usize as *mut _,
            0x1000,
            PROT_READ | PROT_WRITE,
            flags,
            -1,
            0,
        )
    };
    if ptr == MAP_FAILED {
        return Err(format!(
            "failed to map UART buffer at 0x10000000: {}",
            std::io::Error::last_os_error()
        ));
    }
    unsafe { std::ptr::write_bytes(ptr as *mut u8, 0, 0x1000) };
    Ok(ptr as *mut u8)
}

#[cfg(unix)]
fn map_executable(code: &[u8]) -> Result<*mut u8, String> {
    use libc::*;
    #[cfg(target_os = "linux")]
    let flags = MAP_PRIVATE | MAP_ANONYMOUS;
    #[cfg(not(target_os = "linux"))]
    let flags = MAP_PRIVATE | MAP_ANON;

    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            code.len(),
            PROT_READ | PROT_WRITE | PROT_EXEC,
            flags,
            -1,
            0,
        )
    };
    if ptr == MAP_FAILED {
        return Err(format!(
            "failed to map executable memory: {}",
            std::io::Error::last_os_error()
        ));
    }
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len()) };
    Ok(ptr as *mut u8)
}

#[cfg(unix)]
fn flush_uart(uart_ptr: *mut u8, buf_size: usize) {
    let buf = unsafe { std::slice::from_raw_parts(uart_ptr, buf_size) };
    let len = buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(buf_size)
        .min(buf_size);
    if len > 0 {
        unsafe { libc::write(1, buf.as_ptr() as *const _, len) };
    }
}

#[cfg(windows)]
fn map_uart_buffer() -> Result<*mut u8, String> {
    use windows_sys::Win32::System::Memory::*;
    let ptr = unsafe {
        VirtualAlloc(
            0x10000000usize as *mut _,
            0x1000,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    if ptr.is_null() {
        return Err("failed to map UART buffer at 0x10000000".to_string());
    }
    unsafe { std::ptr::write_bytes(ptr as *mut u8, 0, 0x1000) };
    Ok(ptr as *mut u8)
}

#[cfg(windows)]
fn map_executable(code: &[u8]) -> Result<*mut u8, String> {
    use windows_sys::Win32::System::Memory::*;
    let ptr = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            code.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    if ptr.is_null() {
        return Err("failed to map executable memory".to_string());
    }
    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len()) };
    Ok(ptr as *mut u8)
}

#[cfg(windows)]
fn flush_uart(uart_ptr: *mut u8, buf_size: usize) {
    use windows_sys::Win32::Storage::FileSystem::WriteFile;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
    let buf = unsafe { std::slice::from_raw_parts(uart_ptr, buf_size) };
    let len = buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(buf_size)
        .min(buf_size) as u32;
    if len > 0 {
        let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        let mut written = 0u32;
        unsafe {
            WriteFile(
                handle,
                buf.as_ptr(),
                len,
                &mut written,
                std::ptr::null_mut(),
            )
        };
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: kernrift <file.krbo>");
        std::process::exit(2);
    }
    let path = &args[1];
    if let Err(e) = run_krbo_file(path) {
        eprintln!("kernrift: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fat_file_detection_integration() {
        // Build a minimal x86_64 krbo slice and wrap it in a fat binary
        // Use krir::emit_krbofat_bytes and krir::parse_krbofat_slice to round-trip
        let x86_slice = b"KRBO\x01\x01\x00\x00\x04\x00\x00\x00\x04\x00\x00\x00\xc3___".to_vec();
        let fat = krir::emit_krbofat_bytes(&[
            (krir::KRBO_FAT_ARCH_X86_64, x86_slice.clone()),
        ]).expect("emit failed");

        // Extract and verify round-trip
        let extracted = krir::parse_krbofat_slice(&fat, krir::KRBO_FAT_ARCH_X86_64, Some("test.krbo"))
            .expect("slice not found");
        assert_eq!(extracted, x86_slice, "round-trip bytes must match");
    }
}
