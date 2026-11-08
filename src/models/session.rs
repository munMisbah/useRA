//! Login session model.

/// A login session reported by the system (loginctl where available).
#[derive(Debug, Clone)]
pub struct Session {
    /// Session identifier (e.g. "c1", "3").
    pub id: String,
    /// Owning username.
    pub user: String,
    /// Seat or tty identifier (e.g. "seat0", "tty/pts/0").
    pub seat: String,
    /// Whether the session is currently active.
    pub active: bool,
}
