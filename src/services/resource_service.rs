//! Resource Manager service layer: filesystem listing, standard POSIX
//! permissions (`chmod`/`chown`), POSIX ACLs (`getfacl`/`setfacl`), and
//! recursive path safety guards.

use crate::errors::AppError;
use crate::models::resource::{
    is_protected_recursive_path, validate_acl_remove_spec, validate_acl_spec, validate_mode,
    validate_owner,
};
use crate::models::{AclInfo, Group, ResourceEntry, User};
use crate::services::user_service::{present, OpResult};
use crate::system::{acl, filesystem, permission};
use std::path::{Path, PathBuf};

/// Initial starting directory for the Resource browser.
pub fn default_start_dir() -> PathBuf {
    filesystem::default_start_dir()
}

/// Lists directory contents with resolved user/group names.
pub fn list_dir(
    dir: &Path,
    users: &[User],
    groups: &[Group],
) -> Result<Vec<ResourceEntry>, AppError> {
    filesystem::list_dir(dir, users, groups)
}

/// Reads POSIX ACL metadata for `path`.
pub fn read_acl(path: &Path) -> AclInfo {
    acl::read_acl(path)
}

/// Counts affected entries across `paths` up to `limit` for recursive previews.
pub fn count_recursive_items_many(paths: &[PathBuf], limit: usize) -> (usize, bool) {
    let mut total = 0;
    for path in paths {
        let rem = limit.saturating_sub(total);
        if rem == 0 {
            return (total, true);
        }
        let (c, trunc) = filesystem::count_recursive_items(path, rem);
        total += c;
        if trunc || total >= limit {
            return (total, true);
        }
    }
    (total, false)
}

/// Refuses recursive operations on critical system directories.
pub fn guard_recursive_paths(paths: &[PathBuf]) -> Result<(), OpResult> {
    for path in paths {
        if is_protected_recursive_path(path) {
            return Err(OpResult::failure(format!(
                "refused: recursive operation on protected system path '{}'",
                path.display()
            )));
        }
    }
    Ok(())
}

fn format_targets(paths: &[PathBuf]) -> String {
    if paths.len() == 1 {
        paths[0].display().to_string()
    } else {
        format!("{} items", paths.len())
    }
}

/// Changes POSIX file mode via `chmod`.
pub fn set_mode(paths: &[PathBuf], mode: &str, recursive: bool) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    let valid = match validate_mode(mode) {
        Ok(m) => m,
        Err(msg) => return OpResult::failure(format!("invalid mode: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }
    let target = format_targets(paths);
    match permission::chmod(paths, &valid, recursive) {
        Ok(()) => OpResult::success(format!(
            "mode of {target} set to {valid}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => op_error("chmod", &target, err),
    }
}

/// Changes owner/group via `chown`.
pub fn set_owner(paths: &[PathBuf], owner: &str, recursive: bool) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    let valid = match validate_owner(owner) {
        Ok(o) => o,
        Err(msg) => return OpResult::failure(format!("invalid owner: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }
    let target = format_targets(paths);
    match permission::chown(paths, &valid, recursive) {
        Ok(()) => OpResult::success(format!(
            "owner of {target} set to {valid}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => op_error("chown", &target, err),
    }
}

/// Adds or updates one or more POSIX ACL entries (e.g. access + default) via `setfacl -m`.
pub fn set_acl_many(paths: &[PathBuf], specs: &[String], recursive: bool) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    if specs.is_empty() {
        return OpResult::failure("no ACL entries to apply".to_string());
    }
    let mut valid_specs = Vec::with_capacity(specs.len());
    for s in specs {
        match validate_acl_spec(s) {
            Ok(v) => valid_specs.push(v),
            Err(msg) => return OpResult::failure(format!("invalid ACL: {msg}")),
        }
    }
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }
    let joined = valid_specs.join(",");
    let target = format_targets(paths);
    match acl::set_acl(paths, &joined, recursive) {
        Ok(()) => OpResult::success(format!(
            "ACL '{joined}' applied to {target}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => acl_error("setfacl", &target, err),
    }
}

/// Removes one or more specific ACL entries and/or clears all default ACLs (`-k`).
pub fn remove_acl_many(
    paths: &[PathBuf],
    specs: &[String],
    clear_default: bool,
    recursive: bool,
) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    if specs.is_empty() && !clear_default {
        return OpResult::failure("select at least one ACL entry".to_string());
    }
    let mut valid_specs = Vec::with_capacity(specs.len());
    for s in specs {
        match validate_acl_remove_spec(s) {
            Ok(v) => valid_specs.push(v),
            Err(msg) => return OpResult::failure(format!("invalid ACL target: {msg}")),
        }
    }
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }

    let target = format_targets(paths);
    if !valid_specs.is_empty() {
        let joined = valid_specs.join(",");
        if let Err(err) = acl::remove_acl(paths, &joined, recursive) {
            return acl_error("remove ACL", &target, err);
        }
    }
    if clear_default {
        if let Err(err) = acl::clear_default_acl(paths, recursive) {
            return acl_error("clear default ACLs", &target, err);
        }
    }

    let summary = match (!valid_specs.is_empty(), clear_default) {
        (true, true) => format!(
            "removed {} and cleared default ACLs",
            valid_specs.join(", ")
        ),
        (true, false) => format!("removed ACL {}", valid_specs.join(", ")),
        (false, true) => "cleared default ACLs".to_string(),
        (false, false) => unreachable!(),
    };
    OpResult::success(format!(
        "{summary} on {target}{}",
        if recursive { " (recursive)" } else { "" }
    ))
}

/// Strips all extended ACL entries via `setfacl -b`.
pub fn clear_acl(paths: &[PathBuf], recursive: bool) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }
    let target = format_targets(paths);
    match acl::clear_acl(paths, recursive) {
        Ok(()) => OpResult::success(format!("extended ACLs cleared on {target}")),
        Err(err) => acl_error("clear ACLs", &target, err),
    }
}

/// Exports headerless ACL rules for Copy ACL (`C`).
pub fn export_acl(path: &Path) -> Result<String, String> {
    acl::export_acl(path).map_err(|err| {
        let raw = err.to_string();
        if raw.contains("No such file or directory") && raw.contains("getfacl") {
            "getfacl not found — install the `acl` package (`sudo apt install acl`)".to_string()
        } else {
            raw
        }
    })
}

/// Clones `acl_text` onto `paths` via `setfacl --set-file=-`.
/// Strips `default:` rules automatically when pasting onto non-directory files.
pub fn paste_acl(paths: &[PathBuf], acl_text: &str, recursive: bool) -> OpResult {
    if paths.is_empty() {
        return OpResult::failure("no target paths selected".to_string());
    }
    if acl_text.trim().is_empty() {
        return OpResult::failure("ACL clipboard is empty".to_string());
    }
    if recursive {
        if let Err(res) = guard_recursive_paths(paths) {
            return res;
        }
    }

    let dirs: Vec<PathBuf> = paths.iter().filter(|p| p.is_dir()).cloned().collect();
    let files: Vec<PathBuf> = paths.iter().filter(|p| !p.is_dir()).cloned().collect();
    let target = format_targets(paths);

    if !dirs.is_empty() {
        if let Err(err) = acl::apply_acl_text(&dirs, acl_text, recursive) {
            return acl_error("paste ACL", &target, err);
        }
    }
    if !files.is_empty() {
        let access_only: String = acl_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("default:"))
            .collect::<Vec<&str>>()
            .join("\n");
        if !access_only.is_empty() {
            if let Err(err) = acl::apply_acl_text(&files, &access_only, false) {
                return acl_error("paste ACL", &target, err);
            }
        }
    }

    OpResult::success(format!(
        "cloned ACL onto {target}{}",
        if recursive { " (recursive)" } else { "" }
    ))
}

fn op_error(op: &str, target: &str, err: AppError) -> OpResult {
    let raw = err.to_string();
    if raw.to_lowercase().contains("operation not permitted") {
        return OpResult::failure(format!(
            "{op} {target}: insufficient privileges — run `sudo usera`"
        ));
    }
    OpResult::failure(present(op, target, false, &raw))
}

fn acl_error(op: &str, target: &str, err: AppError) -> OpResult {
    let raw = err.to_string();
    if raw.contains("No such file or directory") && raw.contains("setfacl") {
        return OpResult::failure(
            "setfacl not found — install the `acl` package (`sudo apt install acl`)".to_string(),
        );
    }
    op_error(op, target, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_recursive_ops_on_protected_system_paths() {
        let res = set_mode(&[PathBuf::from("/etc")], "0755", true);
        assert!(!res.ok);
        assert!(res.message.contains("protected system path"));

        let res2 = set_owner(
            &[PathBuf::from("/tmp/ok"), PathBuf::from("/")],
            "root:root",
            true,
        );
        assert!(!res2.ok);
        assert!(res2.message.contains("protected system path"));

        let res3 = set_acl_many(&[PathBuf::from("/usr")], &["u:deploy:rwX".into()], true);
        assert!(!res3.ok);
        assert!(res3.message.contains("protected system path"));

        let res4 = paste_acl(
            &[PathBuf::from("/var")],
            "user::rwx\ngroup::r-x\nother::---",
            true,
        );
        assert!(!res4.ok);
        assert!(res4.message.contains("protected system path"));
    }

    #[test]
    fn validates_inputs_before_calling_tools() {
        let p = [PathBuf::from("/tmp/x")];
        assert!(!set_mode(&p, "-R", false).ok);
        assert!(!set_owner(&p, "bad owner", false).ok);
        assert!(!set_acl_many(&p, &["bad-acl".into()], false).ok);
        assert!(
            set_acl_many(&p, &["u:deploy:rwX".into()], false).ok
                || !crate::system::privilege::is_root()
        );
        assert!(!remove_acl_many(&p, &["u:alice:rwx".into()], false, false).ok);
        assert!(!remove_acl_many(&p, &[], false, false).ok);
    }
}
