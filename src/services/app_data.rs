//! Aggregated application data snapshot loaded from the system.
//!
//! Load is read-only and cheap (a handful of `getent`/`uname` calls);
//! it runs synchronously on page entry and after future operations.

use crate::errors::AppError;
use crate::models::{Group, HostInfo, Session, User};
use crate::system::account_reader;

/// Everything the UI renders, read from the live system.
#[derive(Debug, Default)]
pub struct AppData {
    /// Host facts (hostname, OS, kernel).
    pub host: HostInfo,
    /// All users, sorted by UID.
    pub users: Vec<User>,
    /// All groups, sorted by GID.
    pub groups: Vec<Group>,
    /// Login sessions (empty without logind).
    pub sessions: Vec<Session>,
    /// True when the process runs as root.
    pub is_root: bool,
}

impl AppData {
    /// Loads a fresh snapshot from the system.
    pub fn load() -> Result<Self, AppError> {
        let users = account_reader::read_users()?;
        let groups = account_reader::read_groups()?;

        let mut users = users;
        users.sort_by_key(|u| u.uid);
        let mut groups = groups;
        groups.sort_by_key(|g| g.gid);

        Ok(AppData {
            host: account_reader::read_host_info(),
            users,
            groups,
            sessions: account_reader::read_sessions(),
            is_root: crate::system::privilege::is_root(),
        })
    }

    /// Total number of regular (UID >= 1000) accounts.
    pub fn regular_user_count(&self) -> usize {
        self.users
            .iter()
            .filter(|u| u.account_type() == crate::models::AccountType::Regular)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_live_snapshot_sorted() {
        let data = AppData::load().expect("read-only load works unprivileged");
        assert!(!data.users.is_empty());
        assert!(!data.groups.is_empty());
        assert!(data.users.windows(2).all(|w| w[0].uid <= w[1].uid));
        assert!(data.groups.windows(2).all(|w| w[0].gid <= w[1].gid));
        assert!(data.users.iter().any(|u| u.name == "root"));
    }

    #[test]
    fn regular_count_excludes_system_accounts() {
        let data = AppData::load().expect("read-only load works");
        let expected = data.users.iter().filter(|u| u.uid >= 1000).count();
        assert_eq!(data.regular_user_count(), expected);
    }
}
