//! Phase 3 user management operations.
//!
//! Every function shells out to the standard Debian tools with separate
//! arguments (AGENTS.md §5) and returns a human-readable result. No
//! shell interpolation of user input ever happens.

use crate::errors::AppError;
use crate::models::user::validate_username;
use crate::system::command;

/// Outcome of an admin operation, ready to display to the user.
#[derive(Debug)]
pub struct OpResult {
    /// Whether the operation succeeded.
    pub ok: bool,
    /// Message for the notification bar.
    pub message: String,
}

impl OpResult {
    fn success(message: impl Into<String>) -> Self {
        OpResult {
            ok: true,
            message: message.into(),
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        OpResult {
            ok: false,
            message: message.into(),
        }
    }
}

/// Maps a failed `Command` run to an error message with stderr context.
fn command_error(op: &str, target: &str, err: AppError) -> OpResult {
    let raw = format!("{op} {target}: {err}");
    OpResult::failure(present(op, target, false, &raw))
}

fn stderr_text(output: &std::process::Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if text.is_empty() {
        "no stderr".to_string()
    } else {
        text
    }
}

fn status_error(op: &str, target: &str, output: &std::process::Output) -> OpResult {
    let raw = format!("{op} {target} failed: {}", stderr_text(output));
    OpResult::failure(present(op, target, false, &raw))
}

/// Translates raw tool errors into user-facing hints where the cause is
/// unambiguous. Unknown errors pass through unchanged.
fn friendly_error(message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();
    if lower.contains("permission denied") || lower.contains("cannot lock") {
        Some("insufficient privileges — run `sudo usera`")
    } else if lower.contains("already exists") {
        Some("already exists on this system")
    } else if lower.contains("not found") || lower.contains("does not exist") {
        Some("not found on this system (stale list? press [r] to reload)")
    } else if lower.contains("currently used by process") || lower.contains("is currently") {
        Some("account/group is in use by running processes")
    } else if lower.contains("home directory") && lower.contains("not") {
        Some("home directory missing or not owned by the user")
    } else {
        None
    }
}

/// Builds the final message: friendly hint when known, raw detail otherwise.
pub fn present(op: &str, target: &str, ok: bool, raw: &str) -> String {
    if ok {
        return raw.to_string();
    }
    match friendly_error(raw) {
        Some(hint) => format!("{op} {target}: {hint}"),
        None => format!("{op} {target} failed: {raw}"),
    }
}

/// Guards destructive operations against system accounts (UID < 1000)
/// and the root account. Deleting/locking `root` or service accounts
/// breaks the system; refuse with a clear reason (SECURITY.md §6).
pub fn guard_system_account(name: &str, users: &[crate::models::User]) -> Result<(), OpResult> {
    let user = match users.iter().find(|u| u.name == name) {
        Some(u) => u,
        None => return Ok(()), // unknown target: let the tool decide
    };
    if user.uid == 0 {
        return Err(OpResult::failure(
            "refused: cannot delete or lock the root account".to_string(),
        ));
    }
    if user.uid < 1000 {
        return Err(OpResult::failure(format!(
            "refused: '{}' is a system account (UID {}); removing it can break the system",
            user.name, user.uid
        )));
    }
    Ok(())
}

/// Creates a user account.
///
/// `home` is the requested home dir (`None` = default), `shell` the
/// login shell path, `create_home` mirrors useradd's `-m` behavior.
pub fn create_user(
    name: &str,
    gecos: Option<&str>,
    home: Option<&str>,
    shell: &str,
    create_home: bool,
) -> OpResult {
    if let Err(msg) = validate_username(name) {
        return OpResult::failure(format!("invalid username: {msg}"));
    }
    let mut args: Vec<&str> = vec!["--shell", shell];
    if let Some(dir) = home {
        args.push("--home");
        args.push(dir);
    }
    if create_home {
        args.push("--create-home");
    }
    if let Some(comment) = gecos {
        if !comment.is_empty() {
            args.push("--comment");
            args.push(comment);
        }
    }
    args.push(name);

    match command::run("useradd", &args) {
        Err(err) => command_error("create user", name, err),
        Ok(output) if output.status.success() => OpResult::success(format!("user {name} created")),
        Ok(output) => status_error("create user", name, &output),
    }
}

/// Locks an account (`passwd -l`).
pub fn lock_user(name: &str) -> OpResult {
    lock_impl(name, true)
}

/// Unlocks an account (`passwd -u`).
pub fn unlock_user(name: &str) -> OpResult {
    lock_impl(name, false)
}

fn lock_impl(name: &str, lock: bool) -> OpResult {
    let op = if lock { "lock" } else { "unlock" };
    let flag = if lock { "-l" } else { "-u" };
    match command::run("passwd", &[flag, name]) {
        Err(err) => command_error(op, name, err),
        Ok(output) if output.status.success() => OpResult::success(format!("user {name} {op}ed")),
        Ok(output) => status_error(op, name, &output),
    }
}

/// Changes a user's login shell (`usermod -s`).
pub fn set_shell(name: &str, shell: &str) -> OpResult {
    if !shell.starts_with('/') {
        return OpResult::failure("shell must be an absolute path".to_string());
    }
    match command::run("usermod", &["--shell", shell, name]) {
        Err(err) => command_error("change shell", name, err),
        Ok(output) if output.status.success() => {
            OpResult::success(format!("shell of {name} set to {shell}"))
        }
        Ok(output) => status_error("change shell", name, &output),
    }
}

/// Sets or clears account expiration (`chage -E`); `YYYY-MM-DD` or
/// `None` to clear.
pub fn set_expiry(name: &str, date: Option<&str>) -> OpResult {
    let flag = "-E";
    let value = date.unwrap_or("-1");
    if let Some(d) = date {
        // chage accepts YYYY-MM-DD only.
        let ok = d.len() == 10
            && d.as_bytes()[4] == b'-'
            && d.as_bytes()[7] == b'-'
            && d.chars().all(|c| c.is_ascii_digit() || c == '-');
        if !ok {
            return OpResult::failure("expiry must be YYYY-MM-DD".to_string());
        }
    }
    match command::run("chage", &[flag, value, name]) {
        Err(err) => command_error("set expiry", name, err),
        Ok(output) if output.status.success() => OpResult::success(format!(
            "expiry of {name} {}",
            date.map(|d| format!("set to {d}"))
                .unwrap_or_else(|| "cleared".into())
        )),
        Ok(output) => status_error("set expiry", name, &output),
    }
}

/// Adds a user to supplementary groups (`usermod -aG`).
pub fn add_to_groups(name: &str, groups: &[&str]) -> OpResult {
    if groups.is_empty() {
        return OpResult::failure("no groups given".to_string());
    }
    let list = groups.join(",");
    match command::run("usermod", &["--append", "--groups", &list, name]) {
        Err(err) => command_error("add groups", name, err),
        Ok(output) if output.status.success() => {
            OpResult::success(format!("{name} added to {list}"))
        }
        Ok(output) => status_error("add groups", name, &output),
    }
}

/// Deletes a user account (`userdel`).
///
/// `remove_home` is passed by the caller only after an explicit,
/// separate confirmation (SECURITY.md §6): with `-r` the home dir and
/// mail spool are removed; without it the account is removed but files
/// remain.
pub fn delete_user(name: &str, remove_home: bool) -> OpResult {
    let args: Vec<&str> = if remove_home {
        vec!["--remove", name]
    } else {
        vec![name]
    };
    match command::run("userdel", &args) {
        Err(err) => command_error("delete user", name, err),
        Ok(output) if output.status.success() => OpResult::success(format!(
            "user {name} deleted{}",
            if remove_home {
                " (home removed)"
            } else {
                " (home kept)"
            }
        )),
        Ok(output) => status_error("delete user", name, &output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_maps_permission_denied_to_hint() {
        let msg = present("lock", "root", false, "passwd: Permission denied.");
        assert!(msg.contains("insufficient privileges"), "got {msg}");
        assert!(msg.contains("sudo usera"));
    }

    #[test]
    fn present_maps_stale_target() {
        let msg = present(
            "delete",
            "ghost",
            false,
            "userdel: user ghost does not exist",
        );
        assert!(msg.contains("not found"), "got {msg}");
    }

    #[test]
    fn present_passes_unknown_errors_through() {
        let msg = present("op", "x", false, "weird tool failure 42");
        assert!(msg.contains("weird tool failure 42"));
    }

    #[test]
    fn present_keeps_success_messages() {
        assert_eq!(present("op", "x", true, "all good"), "all good");
    }

    #[test]
    fn system_accounts_guarded_from_destructive_ops() {
        let users = vec![
            crate::models::User {
                name: "root".into(),
                uid: 0,
                gid: 0,
                gecos: String::new(),
                home: "/root".into(),
                shell: String::new(),
                supplementary_groups: vec![],
            },
            crate::models::User {
                name: "www-data".into(),
                uid: 33,
                gid: 33,
                gecos: String::new(),
                home: String::new(),
                shell: String::new(),
                supplementary_groups: vec![],
            },
            crate::models::User {
                name: "deploy".into(),
                uid: 1001,
                gid: 1001,
                gecos: String::new(),
                home: String::new(),
                shell: String::new(),
                supplementary_groups: vec![],
            },
        ];
        assert!(guard_system_account("root", &users).is_err());
        assert!(guard_system_account("www-data", &users).is_err());
        assert!(guard_system_account("deploy", &users).is_ok());
        // Unknown targets fall through to the tool.
        assert!(guard_system_account("ghost", &users).is_ok());
    }

    #[test]
    fn create_user_rejects_invalid_names_without_running() {
        let res = create_user("Bad Name", None, None, "/bin/bash", true);
        assert!(!res.ok);
        assert!(res.message.contains("invalid username"));
    }

    #[test]
    fn set_shell_rejects_relative_paths() {
        let res = set_shell("root", "bash");
        assert!(!res.ok);
        assert!(res.message.contains("absolute path"));
    }

    #[test]
    fn set_expiry_validates_date_shape() {
        // Wrong shape rejected before running chage.
        let res = set_expiry("root", Some("tomorrow"));
        assert!(!res.ok);
        assert!(res.message.contains("YYYY-MM-DD"));

        // Right shape but invalid date passes shape-check and fails at
        // chage; unprivileged run gets the friendly privilege hint.
        let res2 = set_expiry("root", Some("2026-13-99"));
        assert!(!res2.ok);
        assert!(
            res2.message.contains("insufficient privileges") || res2.message.contains("failed"),
            "got: {}",
            res2.message
        );
    }

    #[test]
    fn add_to_groups_rejects_empty_list() {
        let res = add_to_groups("root", &[]);
        assert!(!res.ok);
    }

    #[test]
    fn ops_report_clear_failure_for_missing_targets() {
        // Unprivileged run: passwd -l on a nonexistent user fails with
        // a status error, exercising the stderr path.
        let res = lock_user("no-such-user-xyz");
        assert!(!res.ok);
        assert!(res.message.starts_with("lock no-such-user-xyz"));

        let res2 = delete_user("no-such-user-xyz", false);
        assert!(!res2.ok);
    }
}
