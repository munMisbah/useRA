//! Account database readers (passwd, group) via `getent` and
//! session/host readers. Purely read-only; never mutates the system.

use crate::errors::AppError;
use crate::models::{Group, HostInfo, Session, User};
use crate::system::command;

/// Reads all users from the passwd database.
///
/// Supplementary group membership is merged from the group database.
/// Never reads `/etc/shadow` (SECURITY.md §4).
pub fn read_users() -> Result<Vec<User>, AppError> {
    let output = command::run("getent", &["passwd"])?;
    if !output.status.success() {
        return Err(AppError::Io("getent passwd failed".to_string()));
    }
    let text = String::from_utf8_lossy(&output.stdout);

    let mut users: Vec<User> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_passwd_line)
        .collect();

    // Merge group memberships by member username.
    for group in read_groups()? {
        for member in &group.members {
            for user in &mut users {
                if &user.name == member {
                    user.supplementary_groups.push(group.name.clone());
                }
            }
        }
    }
    Ok(users)
}

/// Parses one `name:pw:uid:gid:gecos:home:shell` line.
fn parse_passwd_line(line: &str) -> Option<User> {
    let mut parts = line.split(':');
    let name = parts.next()?.to_string();
    let _password = parts.next()?; // always 'x'; shadow is never read
    let uid = parts.next()?.parse().ok()?;
    let gid = parts.next()?.parse().ok()?;
    let gecos = parts.next()?.to_string();
    let home = parts.next()?.to_string();
    let shell = parts.next()?.to_string();
    Some(User {
        name,
        uid,
        gid,
        gecos,
        home,
        shell,
        supplementary_groups: Vec::new(),
    })
}

/// Reads all groups from the group database.
pub fn read_groups() -> Result<Vec<Group>, AppError> {
    let output = command::run("getent", &["group"])?;
    if !output.status.success() {
        return Err(AppError::Io("getent group failed".to_string()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_group_line)
        .collect())
}

/// Parses one `name:pw:gid:members` line.
fn parse_group_line(line: &str) -> Option<Group> {
    let mut parts = line.split(':');
    let name = parts.next()?.to_string();
    let _password = parts.next()?;
    let gid = parts.next()?.parse().ok()?;
    let members = parts
        .next()
        .unwrap_or("")
        .split(',')
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string())
        .collect();
    Some(Group { name, gid, members })
}

/// Reads login sessions via `loginctl`; empty when systemd/logind is
/// unavailable (e.g. minimal servers).
pub fn read_sessions() -> Vec<Session> {
    let Some(output) = command::run_trimmed("loginctl", &["list-sessions", "--no-legend"]) else {
        return Vec::new();
    };
    output.lines().filter_map(parse_session_line).collect()
}

/// Parses one loginctl legend-less row.
///
/// Column layout varies across systemd versions; both observed shapes
/// are handled:
/// - `ID SEAT USER ...` (older format)
/// - `ID UID USER SEAT ...` (current Debian 12/13: uid before user)
fn parse_session_line(line: &str) -> Option<Session> {
    let mut fields = line.split_whitespace();
    let id = fields.next()?.to_string();
    let second = fields.next()?;
    let (user, seat) = if second.chars().all(|c| c.is_ascii_digit()) {
        // `ID UID USER SEAT...`: user third, seat fourth.
        let user = fields.next()?;
        let seat = fields.next().unwrap_or("?").to_string();
        (user.to_string(), seat)
    } else {
        // `ID SEAT USER...`: seat second, user third.
        (fields.next()?.to_string(), second.to_string())
    };
    // Modern loginctl prints yes/no state column; legacy prints "active".
    let active = line.split_whitespace().any(|f| f == "yes" || f == "active");
    Some(Session {
        id,
        user,
        seat,
        active,
    })
}

/// Reads hostname, OS, and kernel facts.
pub fn read_host_info() -> HostInfo {
    HostInfo {
        hostname: command::run_trimmed("uname", &["-n"]).unwrap_or_else(|| "unknown".to_string()),
        os: read_pretty_name(),
        kernel: command::run_trimmed("uname", &["-r"]).unwrap_or_else(|| "unknown".to_string()),
    }
}

/// Reads PRETTY_NAME from /etc/os-release.
fn read_pretty_name() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_passwd_line() {
        let user = parse_passwd_line("deploy:x:1001:1001:Deploy User:/home/deploy:/bin/bash")
            .expect("valid line");
        assert_eq!(user.name, "deploy");
        assert_eq!(user.uid, 1001);
        assert_eq!(user.gid, 1001);
        assert_eq!(user.gecos, "Deploy User");
        assert_eq!(user.home, "/home/deploy");
        assert_eq!(user.shell, "/bin/bash");
    }

    #[test]
    fn rejects_malformed_passwd_lines() {
        assert!(parse_passwd_line("broken").is_none());
        assert!(parse_passwd_line("a:b:notanumber:c:d:e:f").is_none());
        assert!(parse_passwd_line("").is_none());
    }

    #[test]
    fn parses_group_line_with_and_without_members() {
        let g = parse_group_line("sudo:x:27:ubuntu,deploy").expect("valid");
        assert_eq!(g.name, "sudo");
        assert_eq!(g.gid, 27);
        assert_eq!(g.members, vec!["ubuntu", "deploy"]);

        let empty = parse_group_line("users:x:100:").expect("valid");
        assert!(empty.members.is_empty());
    }

    #[test]
    fn parses_loginctl_row_uid_format() {
        // Debian 12/13: `ID UID USER SEAT ... STATE`
        let s = parse_session_line("292 1001 wagydev - 528249 user - yes -").expect("valid");
        assert_eq!(s.id, "292");
        assert_eq!(s.user, "wagydev");
        assert_eq!(s.seat, "-");
        assert!(s.active);

        let inactive = parse_session_line("292 1001 wagydev - 528249 user - no -").expect("valid");
        assert!(!inactive.active);
    }

    #[test]
    fn parses_loginctl_row_legacy_format() {
        // Older: `ID SEAT USER ...`
        let s = parse_session_line("c1 seat0 alice - active").expect("valid");
        assert_eq!(s.id, "c1");
        assert_eq!(s.user, "alice");
        assert_eq!(s.seat, "seat0");
        assert!(s.active);
    }

    #[test]
    fn reads_real_system_data() {
        let users = read_users().expect("getent available on CI host");
        assert!(users.iter().any(|u| u.name == "root"));
        let groups = read_groups().expect("getent available");
        assert!(groups.iter().any(|g| g.name == "root" && g.gid == 0));
    }

    #[test]
    fn host_info_has_os_string() {
        let host = read_host_info();
        assert!(!host.os.is_empty());
    }
}
