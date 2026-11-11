//! Phase 4 group management operations.
//!
//! Same contract as `user_service`: Debian tools with separate
//! arguments, client-side validation, readable results.
//!
//! Reuses the shared `OpResult` type.

use crate::errors::AppError;
use crate::models::group::validate_group_name;
use crate::services::user_service::OpResult;
use crate::system::command;

fn success(message: impl Into<String>) -> OpResult {
    OpResult {
        ok: true,
        message: message.into(),
    }
}

fn failure(message: impl Into<String>) -> OpResult {
    OpResult {
        ok: false,
        message: message.into(),
    }
}

fn status_error(op: &str, target: &str, output: &std::process::Output) -> OpResult {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stderr = if stderr.is_empty() {
        "no stderr".to_string()
    } else {
        stderr
    };
    let raw = format!("{op} {target} failed: {stderr}");
    failure(crate::services::user_service::present(
        op, target, false, &raw,
    ))
}

fn command_error(op: &str, target: &str, err: AppError) -> OpResult {
    failure(format!("{op} {target}: {err}"))
}

/// Creates a group (`groupadd`).
pub fn create_group(name: &str, gid: Option<u32>) -> OpResult {
    if let Err(msg) = validate_group_name(name) {
        return failure(format!("invalid group name: {msg}"));
    }
    let mut args: Vec<String> = Vec::new();
    if let Some(id) = gid {
        args.push("--gid".to_string());
        args.push(id.to_string());
    }
    args.push("--".to_string());
    args.push(name.to_string());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match command::run("groupadd", &arg_refs) {
        Err(err) => command_error("create group", name, err),
        Ok(output) if output.status.success() => success(format!("group {name} created")),
        Ok(output) => status_error("create group", name, &output),
    }
}

/// Renames a group (`groupmod -n new old`).
pub fn rename_group(old: &str, new: &str) -> OpResult {
    if let Err(msg) = validate_group_name(new) {
        return failure(format!("invalid group name: {msg}"));
    }
    match command::run("groupmod", &["--new-name", new, "--", old]) {
        Err(err) => command_error("rename group", old, err),
        Ok(output) if output.status.success() => success(format!("group {old} renamed to {new}")),
        Ok(output) => status_error("rename group", old, &output),
    }
}

/// Deletes a group (`groupdel`). Membership in this group is removed
/// with it; user accounts are untouched.
pub fn delete_group(name: &str) -> OpResult {
    match command::run("groupdel", &["--", name]) {
        Err(err) => command_error("delete group", name, err),
        Ok(output) if output.status.success() => success(format!("group {name} deleted")),
        Ok(output) => status_error("delete group", name, &output),
    }
}

/// Adds users to a group (`gpasswd -a user group`, one call per user so
/// one bad username doesn't abort the rest).
pub fn add_members(group: &str, users: &[String]) -> OpResult {
    mutate_members("add", group, users)
}

/// Removes users from a group (`gpasswd -d user group`).
pub fn remove_members(group: &str, users: &[String]) -> OpResult {
    mutate_members("remove", group, users)
}

fn mutate_members(action: &str, group: &str, users: &[String]) -> OpResult {
    if users.is_empty() {
        return failure("no users given".to_string());
    }
    let flag = if action == "add" { "-a" } else { "-d" };
    let mut failures: Vec<String> = Vec::new();
    let mut done: Vec<String> = Vec::new();
    for user in users {
        match command::run("gpasswd", &[flag, user.as_str(), "--", group]) {
            Err(err) => failures.push(format!("{user}: {err}")),
            Ok(output) if output.status.success() => done.push(user.clone()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                failures.push(format!("{user}: {stderr}"));
            }
        }
    }
    if failures.is_empty() {
        success(format!("{action} {} → {group}", done.join(", ")))
    } else if done.is_empty() {
        failure(format!(
            "{action} to {group} failed for all: {}",
            failures.join("; ")
        ))
    } else {
        failure(format!(
            "{action} to {group} partially done — ok: {}; failed: {}",
            done.join(", "),
            failures.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_group_rejects_invalid_names_without_running() {
        let res = create_group("Bad Group", None);
        assert!(!res.ok);
        assert!(res.message.contains("invalid group name"));
    }

    #[test]
    fn rename_group_validates_new_name() {
        let res = rename_group("users", "Not Valid");
        assert!(!res.ok);
        assert!(res.message.contains("invalid group name"));
    }

    #[test]
    fn member_ops_reject_empty_user_list() {
        assert!(!add_members("users", &[]).ok);
        assert!(!remove_members("users", &[]).ok);
    }

    #[test]
    fn ops_report_failure_for_missing_targets() {
        // Unprivileged runs fail with status errors, exercising paths.
        let res = delete_group("no-such-group-xyz");
        assert!(!res.ok);
        assert!(res.message.starts_with("delete group"));

        let res2 = add_members("no-such-group-xyz", &["no-such-user-xyz".to_string()]);
        assert!(!res2.ok);
    }

    #[test]
    fn partial_member_failures_are_reported() {
        // root exists and would succeed as root; missing user fails.
        // Unprivileged: both fail → all-failed message.
        let res = add_members(
            "no-such-group-xyz",
            &["root".to_string(), "no-such-user-xyz".to_string()],
        );
        assert!(!res.ok);
        assert!(res.message.contains("root") && res.message.contains("no-such-user-xyz"));
    }
}
