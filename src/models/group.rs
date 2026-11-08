//! Group domain model and validation.

/// A group as read from the group database.
#[derive(Debug, Clone)]
pub struct Group {
    /// Group name.
    pub name: String,
    /// POSIX group ID.
    pub gid: u32,
    /// Member usernames listed in the group database.
    pub members: Vec<String>,
}

/// Validates a group name per Debian policy (same charset as usernames).
///
/// Called by group create/rename forms in Phase 4; contract is fixed
/// and unit-tested now.
#[allow(dead_code)]
pub fn validate_group_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("group name must not be empty".to_string());
    }
    if name.len() > 32 {
        return Err("group name must be at most 32 characters".to_string());
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first == b'_') {
        return Err("group name must start with a lowercase letter or underscore".to_string());
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err(
            "group name may only contain lowercase letters, digits, underscores, and hyphens"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_group_names_pass() {
        assert!(validate_group_name("docker").is_ok());
        assert!(validate_group_name("web_dev").is_ok());
        assert!(validate_group_name("team-2").is_ok());
    }

    #[test]
    fn invalid_group_names_rejected() {
        assert!(validate_group_name("").is_err());
        assert!(validate_group_name("Docker").is_err());
        assert!(validate_group_name("a b").is_err());
        assert!(validate_group_name(&"g".repeat(33)).is_err());
    }
}
