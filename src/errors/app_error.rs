//! Centralized application error type.

use std::fmt;

/// Errors that can occur during application execution.
///
/// Constructed from Phase 2 onward (system integration); behavior is
/// unit-tested now so the contract is fixed before first use.
#[allow(dead_code)]
#[derive(Debug)]
pub enum AppError {
    /// Terminal initialization, restoration, or I/O failure.
    Terminal(String),
    /// Generic I/O failure outside terminal handling.
    Io(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Terminal(msg) => write!(f, "terminal error: {msg}"),
            AppError::Io(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_variant_context() {
        let err = AppError::Terminal("raw mode failed".to_string());
        assert_eq!(err.to_string(), "terminal error: raw mode failed");
    }

    #[test]
    fn io_error_converts_into_app_error() {
        let err: AppError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing file").into();
        assert!(matches!(err, AppError::Io(_)));
        assert!(err.to_string().contains("missing file"));
    }
}
