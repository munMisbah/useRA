//! User domain model and validation.

/// Account category of a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    /// UID 0–999: system/service account.
    System,
    /// UID 1000+: regular login account.
    Regular,
}

impl AccountType {
    /// Classifies by UID using the Debian convention (1000 = first user).
    pub fn from_uid(uid: u32) -> Self {
        if uid >= 1000 {
            AccountType::Regular
        } else {
            AccountType::System
        }
    }

    /// Label shown in tables.
    pub const fn label(self) -> &'static str {
        match self {
            AccountType::System => "system",
            AccountType::Regular => "regular",
        }
    }
}

/// A user account as read from the system account databases.
#[derive(Debug, Clone)]
pub struct User {
    /// Login name.
    pub name: String,
    /// POSIX user ID.
    pub uid: u32,
    /// Primary group ID.
    pub gid: u32,
    /// GECOS full name (may be empty).
    pub gecos: String,
    /// Home directory path.
    pub home: String,
    /// Login shell path.
    pub shell: String,
    /// Username → group memberships from the group database.
    pub supplementary_groups: Vec<String>,
}

impl User {
    /// Account category derived from the UID.
    pub fn account_type(&self) -> AccountType {
        AccountType::from_uid(self.uid)
    }
}

/// Validates a username per Debian policy: starts with a letter or
/// underscore, then lowercase letters/digits/underscores/hyphens
/// (optionally ending with `$`), and at most 32 characters.
///
/// Called by create/edit forms in Phase 3; contract is fixed and
/// unit-tested now.
#[allow(dead_code)]
pub fn validate_username(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("username must not be empty".to_string());
    }
    if name.len() > 32 {
        return Err("username must be at most 32 characters".to_string());
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first == b'_') {
        return Err("username must start with a lowercase letter or underscore".to_string());
    }
    let body = &name[..name.len()
        - if name.ends_with('$') && name.len() > 1 {
            1
        } else {
            0
        }];
    if !body
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err(
            "username may only contain lowercase letters, digits, underscores, and hyphens"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uid_classification_matches_debian_convention() {
        assert_eq!(AccountType::from_uid(0), AccountType::System);
        assert_eq!(AccountType::from_uid(999), AccountType::System);
        assert_eq!(AccountType::from_uid(1000), AccountType::Regular);
    }

    #[test]
    fn valid_usernames_pass() {
        assert!(validate_username("deploy").is_ok());
        assert!(validate_username("_system").is_ok());
        assert!(validate_username("user-1").is_ok());
        assert!(validate_username("svc$").is_ok());
    }

    #[test]
    fn invalid_usernames_rejected() {
        assert!(validate_username("").is_err());
        assert!(validate_username("1user").is_err());
        assert!(validate_username("User").is_err());
        assert!(validate_username("us er").is_err());
        assert!(validate_username(&"a".repeat(33)).is_err());
    }
}
