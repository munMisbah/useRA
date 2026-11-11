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

/// Counts affected entries under `path` up to `limit` for recursive previews.
pub fn count_recursive_items(path: &Path, limit: usize) -> (usize, bool) {
    filesystem::count_recursive_items(path, limit)
}

/// Refuses recursive operations on critical system directories.
pub fn guard_recursive_path(path: &Path) -> Result<(), OpResult> {
    if is_protected_recursive_path(path) {
        return Err(OpResult::failure(format!(
            "refused: recursive operation on protected system path '{}'",
            path.display()
        )));
    }
    Ok(())
}

/// Changes POSIX file mode via `chmod`.
pub fn set_mode(path: &Path, mode: &str, recursive: bool) -> OpResult {
    let valid = match validate_mode(mode) {
        Ok(m) => m,
        Err(msg) => return OpResult::failure(format!("invalid mode: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_path(path) {
            return res;
        }
    }
    let target = path.display().to_string();
    match permission::chmod(path, &valid, recursive) {
        Ok(()) => OpResult::success(format!(
            "mode of {target} set to {valid}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => op_error("chmod", &target, err),
    }
}

/// Changes owner/group via `chown`.
pub fn set_owner(path: &Path, owner: &str, recursive: bool) -> OpResult {
    let valid = match validate_owner(owner) {
        Ok(o) => o,
        Err(msg) => return OpResult::failure(format!("invalid owner: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_path(path) {
            return res;
        }
    }
    let target = path.display().to_string();
    match permission::chown(path, &valid, recursive) {
        Ok(()) => OpResult::success(format!(
            "owner of {target} set to {valid}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => op_error("chown", &target, err),
    }
}

/// Adds or updates a POSIX ACL entry via `setfacl -m`.
pub fn set_acl(path: &Path, spec: &str, recursive: bool) -> OpResult {
    let valid = match validate_acl_spec(spec) {
        Ok(s) => s,
        Err(msg) => return OpResult::failure(format!("invalid ACL: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_path(path) {
            return res;
        }
    }
    let target = path.display().to_string();
    match acl::set_acl(path, &valid, recursive) {
        Ok(()) => OpResult::success(format!(
            "ACL '{valid}' applied to {target}{}",
            if recursive { " (recursive)" } else { "" }
        )),
        Err(err) => acl_error("setfacl", &target, err),
    }
}

/// Removes a specific POSIX ACL entry via `setfacl -x`.
pub fn remove_acl(path: &Path, spec: &str, recursive: bool) -> OpResult {
    let valid = match validate_acl_remove_spec(spec) {
        Ok(s) => s,
        Err(msg) => return OpResult::failure(format!("invalid ACL target: {msg}")),
    };
    if recursive {
        if let Err(res) = guard_recursive_path(path) {
            return res;
        }
    }
    let target = path.display().to_string();
    match acl::remove_acl(path, &valid, recursive) {
        Ok(()) => OpResult::success(format!("ACL '{valid}' removed from {target}")),
        Err(err) => acl_error("remove ACL", &target, err),
    }
}

/// Strips all extended ACL entries via `setfacl -b`.
pub fn clear_acl(path: &Path, recursive: bool) -> OpResult {
    if recursive {
        if let Err(res) = guard_recursive_path(path) {
            return res;
        }
    }
    let target = path.display().to_string();
    match acl::clear_acl(path, recursive) {
        Ok(()) => OpResult::success(format!("extended ACLs cleared on {target}")),
        Err(err) => acl_error("clear ACLs", &target, err),
    }
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
        let res = set_mode(Path::new("/etc"), "0755", true);
        assert!(!res.ok);
        assert!(res.message.contains("protected system path"));

        let res2 = set_owner(Path::new("/"), "root:root", true);
        assert!(!res2.ok);
        assert!(res2.message.contains("protected system path"));

        let res3 = set_acl(Path::new("/usr"), "u:deploy:rx", true);
        assert!(!res3.ok);
        assert!(res3.message.contains("protected system path"));
    }

    #[test]
    fn validates_inputs_before_calling_tools() {
        assert!(!set_mode(Path::new("/tmp/x"), "-R", false).ok);
        assert!(!set_owner(Path::new("/tmp/x"), "bad owner", false).ok);
        assert!(!set_acl(Path::new("/tmp/x"), "bad-acl", false).ok);
        assert!(!remove_acl(Path::new("/tmp/x"), "u:alice:rwx", false).ok);
    }
}
