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
    OpResult::failure(format!("{op} {target}: {err}"))
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
    OpResult::failure(format!("{op} {target} failed: {}", stderr_text(output)))
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
        // chage with a status error (unprivileged here).
        let res2 = set_expiry("root", Some("2026-13-99"));
        assert!(!res2.ok);
        assert!(res2.message.contains("failed"));
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
