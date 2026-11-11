//! Resource Manager domain models: filesystem entries, POSIX ACL entries,
//! effective permission evaluation, and safety validators.

use std::path::{Component, Path, PathBuf};

/// Filesystem entry classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

/// One filesystem item shown in the Resource Manager browser.
#[derive(Debug, Clone)]
pub struct ResourceEntry {
    /// Display name (or `..` for parent link).
    pub name: String,
    /// Full path on disk.
    pub path: PathBuf,
    /// Entry classification.
    pub kind: EntryKind,
    /// POSIX permission mode bits (`st_mode & 0o7777`).
    pub mode: u32,
    /// Owning user ID.
    pub uid: u32,
    /// Owning group ID.
    pub gid: u32,
    /// Resolved owner name (falls back to numeric UID).
    pub owner_name: String,
    /// Resolved group name (falls back to numeric GID).
    pub group_name: String,
    /// File size in bytes.
    pub size: u64,
    /// Whether this row is the synthetic `..` parent entry.
    pub is_parent_link: bool,
    /// Whether this entry carries extended POSIX ACLs (`+` badge).
    pub has_acl: bool,
    /// Symlink target path string if this entry is a symlink.
    pub symlink_target: Option<String>,
}

impl ResourceEntry {
    /// Four-digit octal mode string, e.g. `0750` or `2755`.
    pub fn mode_octal(&self) -> String {
        format!("{:04o}", self.mode & 0o7777)
    }

    /// Standard `drwxr-x---` permission string including file-type prefix and special bits.
    pub fn mode_rwx(&self) -> String {
        format_mode_rwx(self.mode, self.kind)
    }
}

/// Formats POSIX mode bits into standard 10-char `drwxr-x---` notation.
pub fn format_mode_rwx(mode: u32, kind: EntryKind) -> String {
    let prefix = match kind {
        EntryKind::Directory => 'd',
        EntryKind::Symlink => 'l',
        EntryKind::File => '-',
        EntryKind::Other => '?',
    };
    let m = mode;
    let mut s = String::with_capacity(10);
    s.push(prefix);
    s.push(if m & 0o400 != 0 { 'r' } else { '-' });
    s.push(if m & 0o200 != 0 { 'w' } else { '-' });
    s.push(match (m & 0o100 != 0, m & 0o4000 != 0) {
        (true, true) => 's',
        (false, true) => 'S',
        (true, false) => 'x',
        (false, false) => '-',
    });
    s.push(if m & 0o040 != 0 { 'r' } else { '-' });
    s.push(if m & 0o020 != 0 { 'w' } else { '-' });
    s.push(match (m & 0o010 != 0, m & 0o2000 != 0) {
        (true, true) => 's',
        (false, true) => 'S',
        (true, false) => 'x',
        (false, false) => '-',
    });
    s.push(if m & 0o004 != 0 { 'r' } else { '-' });
    s.push(if m & 0o002 != 0 { 'w' } else { '-' });
    s.push(match (m & 0o001 != 0, m & 0o1000 != 0) {
        (true, true) => 't',
        (false, true) => 'T',
        (true, false) => 'x',
        (false, false) => '-',
    });
    s
}

impl ResourceEntry {
    /// Compact `owner:group` label.
    pub fn owner_label(&self) -> String {
        format!("{}:{}", self.owner_name, self.group_name)
    }

    /// Human-readable file size.
    pub fn size_label(&self) -> String {
        if self.kind == EntryKind::Directory {
            return "<dir>".to_string();
        }
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * 1024 * 1024;
        if self.size >= GB {
            format!("{:.1} GiB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1} MiB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1} KiB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

/// POSIX ACL entry scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclScope {
    User,
    Group,
    Mask,
    Other,
}

impl AclScope {
    pub const fn short(self) -> &'static str {
        match self {
            AclScope::User => "user",
            AclScope::Group => "group",
            AclScope::Mask => "mask",
            AclScope::Other => "other",
        }
    }
}

/// One parsed line from `getfacl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclEntry {
    pub is_default: bool,
    pub scope: AclScope,
    pub qualifier: String,
    pub perms: String,
    pub effective: Option<String>,
}

impl AclEntry {
    /// Renders an ACL entry string matching `getfacl` notation.
    pub fn display_line(&self) -> String {
        let prefix = if self.is_default { "default:" } else { "" };
        let base = format!(
            "{prefix}{}:{}:{}",
            self.scope.short(),
            self.qualifier,
            self.perms
        );
        match &self.effective {
            Some(eff) => format!("{base} (eff: {eff})"),
            None => base,
        }
    }

    /// Effective 3-char permission string for this entry.
    pub fn effective_perms(&self) -> &str {
        self.effective.as_deref().unwrap_or(&self.perms)
    }
}

/// Parsed ACL state for a selected path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AclInfo {
    /// Whether `getfacl` was available and succeeded.
    pub available: bool,
    /// Parsed ACL entries.
    pub entries: Vec<AclEntry>,
    /// Optional diagnostic/hint (e.g. `acl` package missing).
    pub error: Option<String>,
}

impl AclInfo {
    /// True if the path carries any extended ACL entries beyond base u/g/o.
    pub fn has_extended_acl(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.is_default || e.scope == AclScope::Mask || !e.qualifier.is_empty())
    }

    /// Computes effective `rwx` string for a given user against `entry` using POSIX.1e ACL rules.
    pub fn effective_for_user(
        &self,
        username: &str,
        user_uid: u32,
        user_gid: u32,
        user_groups: &[String],
        entry: &ResourceEntry,
    ) -> String {
        // Fallback to standard UNIX mode bits if getfacl isn't available.
        if !self.available || self.entries.is_empty() {
            return standard_effective_for_user(username, user_uid, user_gid, user_groups, entry);
        }

        if user_uid == 0 && entry.uid != 0 && username != entry.owner_name {
            return root_bypass_label(entry, Some(self));
        }
        let mask = self
            .entries
            .iter()
            .find(|e| !e.is_default && e.scope == AclScope::Mask)
            .map(|e| e.perms.as_str());

        // 1. Owning user (mask does not apply to owning user).
        if user_uid == entry.uid || username == entry.owner_name {
            if let Some(u_base) = self
                .entries
                .iter()
                .find(|e| !e.is_default && e.scope == AclScope::User && e.qualifier.is_empty())
            {
                return format!("{} (owner)", u_base.perms);
            }
        }

        // 2. Named user ACL entry.
        if let Some(named_u) = self
            .entries
            .iter()
            .find(|e| !e.is_default && e.scope == AclScope::User && e.qualifier == username)
        {
            let eff = match (&named_u.effective, mask) {
                (Some(e), _) => e.clone(),
                (None, Some(m)) => mask_rwx(&named_u.perms, m),
                (None, None) => named_u.perms.clone(),
            };
            return format!("{eff} (user ACL)");
        }

        // 3. Owning group + named group ACL entries (union, then masked by mask::).
        let mut group_bits = [false; 3];
        let mut matched_group = false;

        if user_gid == entry.gid || user_groups.iter().any(|g| g == &entry.group_name) {
            if let Some(g_base) = self
                .entries
                .iter()
                .find(|e| !e.is_default && e.scope == AclScope::Group && e.qualifier.is_empty())
            {
                or_rwx(&mut group_bits, &g_base.perms);
                matched_group = true;
            }
        }

        for g_entry in self
            .entries
            .iter()
            .filter(|e| !e.is_default && e.scope == AclScope::Group && !e.qualifier.is_empty())
        {
            if user_groups.iter().any(|g| g == &g_entry.qualifier) {
                or_rwx(&mut group_bits, &g_entry.perms);
                matched_group = true;
            }
        }

        if matched_group {
            let raw = bits_to_rwx(group_bits);
            let eff = match mask {
                Some(m) => mask_rwx(&raw, m),
                None => raw,
            };
            return format!("{eff} (group)");
        }

        // 4. Other.
        if let Some(other) = self
            .entries
            .iter()
            .find(|e| !e.is_default && e.scope == AclScope::Other)
        {
            return format!("{} (other)", other.perms);
        }

        standard_effective_for_user(username, user_uid, user_gid, user_groups, entry)
    }

    /// Computes effective `rwx` string for a given group against `entry`.
    pub fn effective_for_group(
        &self,
        group_name: &str,
        group_gid: u32,
        entry: &ResourceEntry,
    ) -> String {
        if self.available && !self.entries.is_empty() {
            let mask = self
                .entries
                .iter()
                .find(|e| !e.is_default && e.scope == AclScope::Mask)
                .map(|e| e.perms.as_str());

            if let Some(named_g) = self
                .entries
                .iter()
                .find(|e| !e.is_default && e.scope == AclScope::Group && e.qualifier == group_name)
            {
                let eff = match (&named_g.effective, mask) {
                    (Some(e), _) => e.clone(),
                    (None, Some(m)) => mask_rwx(&named_g.perms, m),
                    (None, None) => named_g.perms.clone(),
                };
                return format!("{eff} (group ACL)");
            }

            if group_gid == entry.gid || group_name == entry.group_name {
                if let Some(g_base) = self
                    .entries
                    .iter()
                    .find(|e| !e.is_default && e.scope == AclScope::Group && e.qualifier.is_empty())
                {
                    let eff = match mask {
                        Some(m) => mask_rwx(&g_base.perms, m),
                        None => g_base.perms.clone(),
                    };
                    return format!("{eff} (owning group)");
                }
            }
            return "--- (no entry)".to_string();
        }

        if group_gid == entry.gid || group_name == entry.group_name {
            let r = if entry.mode & 0o040 != 0 { 'r' } else { '-' };
            let w = if entry.mode & 0o020 != 0 { 'w' } else { '-' };
            let x = if entry.mode & 0o010 != 0 { 'x' } else { '-' };
            format!("{r}{w}{x} (owning group)")
        } else {
            "--- (not owner)".to_string()
        }
    }
}

fn standard_effective_for_user(
    username: &str,
    user_uid: u32,
    user_gid: u32,
    user_groups: &[String],
    entry: &ResourceEntry,
) -> String {
    if user_uid == 0 && entry.uid != 0 && username != entry.owner_name {
        return root_bypass_label(entry, None);
    }
    let shift = if user_uid == entry.uid || username == entry.owner_name {
        6
    } else if user_gid == entry.gid || user_groups.iter().any(|g| g == &entry.group_name) {
        3
    } else {
        0
    };
    let bits = (entry.mode >> shift) & 0o7;
    let r = if bits & 0o4 != 0 { 'r' } else { '-' };
    let w = if bits & 0o2 != 0 { 'w' } else { '-' };
    let x = if bits & 0o1 != 0 { 'x' } else { '-' };
    let src = match shift {
        6 => "owner",
        3 => "group",
        _ => "other",
    };
    format!("{r}{w}{x} ({src})")
}

fn root_bypass_label(entry: &ResourceEntry, acl: Option<&AclInfo>) -> String {
    let has_any_exec = (entry.mode & 0o111 != 0)
        || acl.is_some_and(|a| {
            a.entries
                .iter()
                .any(|e| !e.is_default && e.effective_perms().contains('x'))
        });
    if entry.kind == EntryKind::Directory || has_any_exec {
        "rwx (root bypass)".to_string()
    } else {
        "rw- (root bypass; no +x)".to_string()
    }
}

fn or_rwx(bits: &mut [bool; 3], perms: &str) {
    let bytes = perms.as_bytes();
    if bytes.first() == Some(&b'r') {
        bits[0] = true;
    }
    if bytes.get(1) == Some(&b'w') {
        bits[1] = true;
    }
    if bytes.get(2) == Some(&b'x') {
        bits[2] = true;
    }
}

fn bits_to_rwx(bits: [bool; 3]) -> String {
    format!(
        "{}{}{}",
        if bits[0] { 'r' } else { '-' },
        if bits[1] { 'w' } else { '-' },
        if bits[2] { 'x' } else { '-' }
    )
}

fn mask_rwx(perms: &str, mask: &str) -> String {
    let p = perms.as_bytes();
    let m = mask.as_bytes();
    let r = if p.first() == Some(&b'r') && m.first() == Some(&b'r') {
        'r'
    } else {
        '-'
    };
    let w = if p.get(1) == Some(&b'w') && m.get(1) == Some(&b'w') {
        'w'
    } else {
        '-'
    };
    let x = if p.get(2) == Some(&b'x') && m.get(2) == Some(&b'x') {
        'x'
    } else {
        '-'
    };
    format!("{r}{w}{x}")
}

/// Validates a `chmod` mode string (3–4 octal digits or symbolic `u+rwx,g-w`).
pub fn validate_mode(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("mode must not be empty".to_string());
    }
    if s.starts_with('-') {
        return Err("mode cannot start with '-' (use u-w / g-x / octal)".to_string());
    }
    if (s.len() == 3 || s.len() == 4) && s.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return Ok(s.to_string());
    }
    for clause in s.split(',') {
        if clause.is_empty() {
            return Err("empty clause in symbolic mode".to_string());
        }
        let op_pos = clause
            .find(['+', '-', '='])
            .ok_or_else(|| "expected octal (e.g. 0755) or symbolic (e.g. u+rwx)".to_string())?;
        let who = &clause[..op_pos];
        let perms = &clause[op_pos + 1..];
        if !who.bytes().all(|b| matches!(b, b'u' | b'g' | b'o' | b'a')) {
            return Err(format!("invalid mode target '{who}' (use u/g/o/a)"));
        }
        if perms.is_empty()
            || !perms
                .bytes()
                .all(|b| matches!(b, b'r' | b'w' | b'x' | b'X' | b's' | b't'))
        {
            return Err(format!("invalid mode permission chars '{perms}'"));
        }
    }
    Ok(s.to_string())
}

/// Validates a `chown` owner specification: `user`, `user:group`, or `:group`.
pub fn validate_owner(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("owner must not be empty".to_string());
    }
    if s.starts_with('-') || s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("owner cannot contain spaces, control chars, or start with '-'".to_string());
    }
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() > 2 {
        return Err("expected user, user:group, or :group".to_string());
    }
    if parts.len() == 2 && parts[0].is_empty() && parts[1].is_empty() {
        return Err("must specify at least user or group".to_string());
    }
    for p in parts.iter().filter(|p| !p.is_empty()) {
        let valid_chars = p
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'$'));
        if !valid_chars {
            return Err(format!("invalid owner/group token '{p}'"));
        }
    }
    Ok(s.to_string())
}

/// Validates a `setfacl -m` spec: `[d:]u:<name>:<rwx|->`, `[d:]g:<name>:<rwx|->`, `m::<rwx>`, etc.
pub fn validate_acl_spec(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("ACL entry must not be empty".to_string());
    }
    if s.starts_with('-') || s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("invalid whitespace or leading '-' in ACL entry".to_string());
    }
    let parts: Vec<&str> = s.split(':').collect();
    let rest = match parts.first().copied() {
        Some("d" | "default") => &parts[1..],
        _ => &parts[..],
    };
    if rest.len() != 3 {
        return Err("expected format like u:alice:rwx, g:devs:rx, or d:u:alice:rwx".to_string());
    }
    let (scope, qualifier, perms) = (rest[0], rest[1], rest[2]);
    match scope {
        "u" | "user" | "g" | "group" => {
            if !qualifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'$'))
            {
                return Err(format!("invalid ACL target '{qualifier}'"));
            }
        }
        "m" | "mask" | "o" | "other" => {
            if !qualifier.is_empty() {
                return Err(format!("'{scope}' ACL cannot have a name qualifier"));
            }
        }
        _ => return Err(format!("unknown ACL scope '{scope}' (use u/g/m/o)")),
    }
    if perms.is_empty()
        || perms.len() > 3
        || !perms
            .bytes()
            .all(|b| matches!(b, b'r' | b'w' | b'x' | b'X' | b'-'))
    {
        return Err(format!("invalid ACL permissions '{perms}' (use r/w/x/X/-)"));
    }
    Ok(s.to_string())
}

/// Validates a `setfacl -x` removal spec: `[d:]u:<name>` or `[d:]g:<name>` or `[d:]m`.
pub fn validate_acl_remove_spec(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("ACL removal target must not be empty".to_string());
    }
    if s.starts_with('-') || s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("invalid whitespace or leading '-' in ACL target".to_string());
    }
    let parts: Vec<&str> = s.split(':').collect();
    let rest = match parts.first().copied() {
        Some("d" | "default") => &parts[1..],
        _ => &parts[..],
    };
    if rest.is_empty() || rest.len() > 2 {
        return Err("expected format like u:alice, g:devs, or d:u:alice".to_string());
    }
    match rest[0] {
        "u" | "user" | "g" | "group" => {
            if rest.len() != 2 || rest[1].is_empty() {
                return Err("user/group ACL removal requires a name (e.g. u:alice)".to_string());
            }
            if !rest[1]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'$'))
            {
                return Err(format!("invalid ACL target '{}'", rest[1]));
            }
        }
        "m" | "mask" => {}
        _ => {
            return Err(format!(
                "cannot remove ACL scope '{}' (use u:<name>, g:<name>, or m)",
                rest[0]
            ))
        }
    }
    Ok(s.to_string())
}

/// Critical top-level OS paths where recursive (`-R`) chmod/chown/setfacl MUST be refused.
const PROTECTED_RECURSIVE_PATHS: [&str; 15] = [
    "/", "/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/proc", "/root", "/run",
    "/sbin", "/sys", "/usr", "/var",
];

/// Returns `true` if recursive operations on `path` would endanger core system directories.
pub fn is_protected_recursive_path(path: &Path) -> bool {
    let normalized = normalize_lexical(path);
    let check_norm = |p: &Path| {
        let s = p.to_string_lossy();
        PROTECTED_RECURSIVE_PATHS.contains(&s.as_ref())
    };
    if check_norm(&normalized) {
        return true;
    }
    if let Ok(canon) = std::fs::canonicalize(path) {
        if check_norm(&canon) {
            return true;
        }
    }
    false
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(c) => out.push(c),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_rwx_and_special_bits() {
        let dir = ResourceEntry {
            name: "shared".into(),
            path: PathBuf::from("/srv/shared"),
            kind: EntryKind::Directory,
            mode: 0o2750,
            uid: 0,
            gid: 1000,
            owner_name: "root".into(),
            group_name: "devs".into(),
            size: 4096,
            is_parent_link: false,
            symlink_target: None,
            has_acl: false,
        };
        assert_eq!(dir.mode_octal(), "2750");
        assert_eq!(dir.mode_rwx(), "drwxr-s---");
    }

    #[test]
    fn validates_modes_and_rejects_flags() {
        assert!(validate_mode("0755").is_ok());
        assert!(validate_mode("750").is_ok());
        assert!(validate_mode("u+rwx,g-w").is_ok());
        assert!(validate_mode("-R").is_err());
        assert!(validate_mode("999").is_err());
        assert!(validate_mode("").is_err());
    }

    #[test]
    fn validates_owners_and_acl_specs() {
        assert!(validate_owner("deploy:www-data").is_ok());
        assert!(validate_owner(":www-data").is_ok());
        assert!(validate_owner("-bad").is_err());
        assert!(validate_owner(":").is_err());

        assert!(validate_acl_spec("u:deploy:rwx").is_ok());
        assert!(validate_acl_spec("u:deploy:rwX").is_ok());
        assert!(validate_acl_spec("d:g:devs:r-X").is_ok());
        assert!(validate_acl_spec("d:g:devs:r-x").is_ok());
        assert!(validate_acl_spec("m::rx").is_ok());
        assert!(validate_acl_spec("u:deploy:bad").is_err());
        assert!(validate_acl_remove_spec("u:deploy").is_ok());
        assert!(validate_acl_remove_spec("u:deploy:rwx").is_err());
    }

    #[test]
    fn protects_critical_root_paths_from_recursive_ops() {
        assert!(is_protected_recursive_path(Path::new("/")));
        assert!(is_protected_recursive_path(Path::new("/etc")));
        assert!(is_protected_recursive_path(Path::new("/var/../etc")));
        assert!(!is_protected_recursive_path(Path::new("/var/www")));
        assert!(!is_protected_recursive_path(Path::new("/home/deploy")));
    }

    #[test]
    fn evaluates_effective_acl_with_mask() {
        let entry = ResourceEntry {
            name: "app".into(),
            path: PathBuf::from("/var/www/app"),
            kind: EntryKind::Directory,
            mode: 0o750,
            uid: 0,
            gid: 33,
            owner_name: "root".into(),
            group_name: "www-data".into(),
            size: 4096,
            is_parent_link: false,
            symlink_target: None,
            has_acl: true,
        };
        let acl = AclInfo {
            available: true,
            entries: vec![
                AclEntry {
                    is_default: false,
                    scope: AclScope::User,
                    qualifier: String::new(),
                    perms: "rwx".into(),
                    effective: None,
                },
                AclEntry {
                    is_default: false,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "rwx".into(),
                    effective: Some("r-x".into()),
                },
                AclEntry {
                    is_default: false,
                    scope: AclScope::Mask,
                    qualifier: String::new(),
                    perms: "r-x".into(),
                    effective: None,
                },
                AclEntry {
                    is_default: false,
                    scope: AclScope::Other,
                    qualifier: String::new(),
                    perms: "---".into(),
                    effective: None,
                },
            ],
            error: None,
        };

        assert_eq!(
            acl.effective_for_user("deploy", 1001, 1001, &[], &entry),
            "r-x (user ACL)"
        );
        assert_eq!(
            acl.effective_for_user("root", 0, 0, &[], &entry),
            "rwx (owner)"
        );
        assert_eq!(
            acl.effective_for_user("guest", 1002, 1002, &[], &entry),
            "--- (other)"
        );
    }

    #[test]
    fn evaluates_root_bypass_and_standard_fallback_without_acl() {
        let regular_no_exec = ResourceEntry {
            name: ".env".into(),
            path: PathBuf::from("/var/www/.env"),
            kind: EntryKind::File,
            mode: 0o600,
            uid: 1001,
            gid: 1001,
            owner_name: "deploy".into(),
            group_name: "deploy".into(),
            size: 64,
            is_parent_link: false,
            has_acl: false,
            symlink_target: None,
        };
        let unavailable_acl = AclInfo {
            available: false,
            entries: Vec::new(),
            error: Some("no getfacl".into()),
        };

        assert_eq!(
            unavailable_acl.effective_for_user("root", 0, 0, &[], &regular_no_exec),
            "rw- (root bypass; no +x)"
        );
        assert_eq!(
            unavailable_acl.effective_for_user("deploy", 1001, 1001, &[], &regular_no_exec),
            "rw- (owner)"
        );
        assert_eq!(
            unavailable_acl.effective_for_user("guest", 1002, 1002, &[], &regular_no_exec),
            "--- (other)"
        );
    }
}
