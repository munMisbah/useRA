//! Standard POSIX permission (`chmod`) and ownership (`chown`) runners.
//!
//! Uses `std::process::Command` with explicit `--` argument separation.

use crate::errors::AppError;
use crate::system::command;
use std::path::PathBuf;

/// Runs `chmod [--|-R --] <mode> <paths...>`.
pub fn chmod(paths: &[PathBuf], mode: &str, recursive: bool) -> Result<(), AppError> {
    for chunk in paths.chunks(200) {
        let mut args: Vec<&str> = Vec::with_capacity(3 + chunk.len());
        if recursive {
            args.push("-R");
        }
        args.push("--");
        args.push(mode);
        let strs: Vec<String> = chunk
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        for s in &strs {
            args.push(s.as_str());
        }
        run_checked("chmod", &args)?;
    }
    Ok(())
}

/// Runs `chown [-h|-R -h] -- <owner> <paths...>`.
///
/// Always passes `-h` so symlinks themselves are targeted rather than
/// unexpectedly dereferencing into external paths.
pub fn chown(paths: &[PathBuf], owner: &str, recursive: bool) -> Result<(), AppError> {
    for chunk in paths.chunks(200) {
        let mut args: Vec<&str> = Vec::with_capacity(4 + chunk.len());
        if recursive {
            args.push("-R");
        }
        args.push("-h");
        args.push("--");
        args.push(owner);
        let strs: Vec<String> = chunk
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        for s in &strs {
            args.push(s.as_str());
        }
        run_checked("chown", &args)?;
    }
    Ok(())
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
        chmod(&[path.clone()], "0640", false).expect("chmod succeeds on owned temp file");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        let _ = fs::remove_file(&path);
    }
}
