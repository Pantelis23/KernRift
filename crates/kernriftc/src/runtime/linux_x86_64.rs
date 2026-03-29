//! Linux x86_64 runtime blob — hand-assembled machine code.
//! Implements _start and all __kr_* functions using Linux syscalls.

use super::RuntimeBlob;

/// Placeholder — will be filled with real machine code in Task 2.
pub static BLOB: RuntimeBlob = RuntimeBlob {
    code: &[0xCC], // int3 (trap if accidentally executed)
    symbols: &[("_start", 0)],
    main_call_fixup: 0,
};
