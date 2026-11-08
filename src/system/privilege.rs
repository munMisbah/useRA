//! Privilege detection for administrative operations.
//!
//! Phase 2 only *detects*; write operations arrive in Phase 3 and will
//! consult this before running admin commands (SECURITY.md §2).

/// Whether the current process runs as root.
pub fn is_root() -> bool {
    nix_free_uid() == 0
}

/// Reads the effective UID without external crates.
fn nix_free_uid() -> u32 {
    // SAFETY: geteuid is always safe to call on POSIX systems.
    unsafe { libc_geteuid() }
}

/// Thin declaration binding to libc's geteuid via the C ABI.
unsafe fn libc_geteuid() -> u32 {
    // geteuid() is declared in libc; link via the `libc` symbols already
    // present in every Rust std target (no external crate needed).
    extern "C" {
        fn geteuid() -> u32;
    }
    geteuid()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_root_returns_bool_consistently() {
        // Either value is valid; just pin the API contract.
        let _ = is_root();
    }
}
