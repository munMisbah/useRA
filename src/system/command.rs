//! Safe command execution helpers.
//!
//! AGENTS.md §5: never construct shell commands by concatenating
//! untrusted input; use `std::process::Command` with separate
//! arguments. All helpers here enforce that.

use crate::errors::AppError;
use std::process::{Command, Output};

/// Runs a command with arguments, capturing stdout/stderr.
///
/// Fails with `AppError::Io` when the binary cannot be spawned.
/// Exit-status handling is left to the caller (read-only commands
/// treat non-zero exit as "no data" rather than a hard error).
pub fn run(program: &str, args: &[&str]) -> Result<Output, AppError> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|e| AppError::Io(format!("failed to run {program}: {e}")))
}

/// Returns trimmed stdout of a successful run, or `None` on spawn
/// failure or non-zero exit.
pub fn run_trimmed(program: &str, args: &[&str]) -> Option<String> {
    let output = run(program, args).ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_known_binary_with_separate_args() {
        let out = run("echo", &["hello", "world"]).expect("echo exists");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello world");
    }

    #[test]
    fn trimmed_output_skips_failure_and_empty() {
        assert!(run_trimmed("false", &[]).is_none());
        assert!(run_trimmed("true", &[]).is_none());
        assert_eq!(run_trimmed("echo", &["  x  "]).as_deref(), Some("x"));
    }

    #[test]
    fn missing_binary_maps_to_io_error() {
        let err = run("definitely-not-a-binary-xyz", &[]).unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
        assert!(err.to_string().contains("definitely-not-a-binary-xyz"));
    }
}
