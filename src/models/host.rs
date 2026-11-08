//! Host information for the dashboard and header.

/// Static host facts read once at startup.
#[derive(Debug, Clone, Default)]
pub struct HostInfo {
    /// Kernel hostname.
    pub hostname: String,
    /// Distribution name (e.g. "Debian GNU/Linux 13.6").
    pub os: String,
    /// Kernel release (e.g. "6.12.113-amd64").
    pub kernel: String,
}
