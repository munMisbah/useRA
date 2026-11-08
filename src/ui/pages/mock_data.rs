//! Mock account data for Phase 1 UI development.
//!
//! Replaced by real system reads in Phase 2 (read-only integration).
//! The page modules consume this module through simple slices so the
//! swap is a data-source change, not a UI change.

/// Placeholder user row until `system/account_reader` exists.
pub struct MockUser {
    /// Login name.
    pub name: &'static str,
    /// POSIX UID.
    pub uid: u32,
    /// Primary GID.
    pub gid: u32,
    /// "regular" or "system".
    pub kind: &'static str,
    /// Home directory path.
    pub home: &'static str,
    /// Default shell.
    pub shell: &'static str,
}

/// Placeholder group row.
pub struct MockGroup {
    /// Group name.
    pub name: &'static str,
    /// POSIX GID.
    pub gid: u32,
    /// Member usernames.
    pub members: &'static [&'static str],
}

/// Placeholder login session row.
pub struct MockSession {
    /// Session identifier.
    pub id: &'static str,
    /// Owning username.
    pub user: &'static str,
    /// Seat/tty identifier.
    pub seat: &'static str,
}

/// Mock user list; includes system and regular accounts.
pub const USERS: &[MockUser] = &[
    MockUser {
        name: "root",
        uid: 0,
        gid: 0,
        kind: "system",
        home: "/root",
        shell: "/bin/bash",
    },
    MockUser {
        name: "systemd-coredump",
        uid: 999,
        gid: 999,
        kind: "system",
        home: "/",
        shell: "/usr/sbin/nologin",
    },
    MockUser {
        name: "ubuntu",
        uid: 1000,
        gid: 1000,
        kind: "regular",
        home: "/home/ubuntu",
        shell: "/bin/bash",
    },
    MockUser {
        name: "deploy",
        uid: 1001,
        gid: 1001,
        kind: "regular",
        home: "/home/deploy",
        shell: "/bin/bash",
    },
    MockUser {
        name: "www-data",
        uid: 33,
        gid: 33,
        kind: "system",
        home: "/var/www",
        shell: "/usr/sbin/nologin",
    },
    MockUser {
        name: "postgres",
        uid: 114,
        gid: 120,
        kind: "system",
        home: "/var/lib/postgresql",
        shell: "/bin/bash",
    },
];

/// Supplementary groups per mock user, by username.
pub const USER_GROUPS: &[(&str, &[&str])] = &[
    ("root", &["root"]),
    ("ubuntu", &["sudo", "docker"]),
    ("deploy", &["sudo", "docker", "www-data"]),
    ("www-data", &["www-data"]),
    ("postgres", &["postgres"]),
    ("systemd-coredump", &["systemd-coredump"]),
];

/// Mock group list.
pub const GROUPS: &[MockGroup] = &[
    MockGroup {
        name: "root",
        gid: 0,
        members: &["root"],
    },
    MockGroup {
        name: "sudo",
        gid: 27,
        members: &["ubuntu", "deploy"],
    },
    MockGroup {
        name: "docker",
        gid: 998,
        members: &["ubuntu", "deploy"],
    },
    MockGroup {
        name: "www-data",
        gid: 33,
        members: &["www-data", "deploy"],
    },
    MockGroup {
        name: "postgres",
        gid: 120,
        members: &["postgres"],
    },
    MockGroup {
        name: "users",
        gid: 100,
        members: &[],
    },
];

/// Mock login sessions.
pub const SESSIONS: &[MockSession] = &[
    MockSession {
        id: "c1",
        user: "ubuntu",
        seat: "seat0",
    },
    MockSession {
        id: "3",
        user: "deploy",
        seat: "tty/pts0",
    },
];

/// Supplementary groups of a mock user.
pub fn groups_of(user: &str) -> Vec<String> {
    USER_GROUPS
        .iter()
        .find(|(name, _)| *name == user)
        .map(|(_, groups)| groups.iter().map(|g| g.to_string()).collect())
        .unwrap_or_default()
}
