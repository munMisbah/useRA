//! Standard POSIX permission (`chmod`) and ownership (`chown`) runners.
//!
//! Uses `std::process::Command` with explicit `--` argument separation.

use crate::errors::AppError;
use crate::system::command;
use std::path::Path;

/// Runs `chmod [--|-R --] <mode> <path>`.
pub fn chmod(path: &Path, mode: &str, recursive: bool) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if recursive {
        vec!["-R", "--", mode, &path_str]
    } else {
        vec!["--", mode, &path_str]
    };
    run_checked("chmod", &args)
}

/// Runs `chown [-h|-R -h] -- <owner> <path>`.
///
/// Always passes `-h` so symlinks themselves are targeted rather than
/// unexpectedly dereferencing into external paths.
pub fn chown(path: &Path, owner: &str, recursive: bool) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if recursive {
        vec!["-R", "-h", "--", owner, &path_str]
    } else {
        vec!["-h", "--", owner, &path_str]
    };
    run_checked("chown", &args)
}

fn run_checked(program: &str, args: &[&str]) -> Result<(), AppError> {
    let out = command::run(program, args)?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(AppError::Io(if stderr.is_empty() {
            format!("{program} exited with status {}", out.status)
        } else {
            stderr
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn chmod_updates_user_owned_temp_file() {
        let path = std::env::temp_dir().join(format!("usera_chmod_test_{}", std::process::id()));
        fs::write(&path, b"test").unwrap();
        chmod(&path, "0640", false).expect("chmod succeeds on owned temp file");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        let _ = fs::remove_file(&path);
    }
}
