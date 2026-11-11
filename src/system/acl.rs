//! POSIX ACL integration (`getfacl` / `setfacl`).
//!
//! Gracefully reports `available: false` when the `acl` package is not
//! installed so standard permission features remain fully usable.

use crate::errors::AppError;
use crate::models::{AclEntry, AclInfo, AclScope};
use crate::system::command;
use std::path::Path;

/// Reads POSIX ACLs for `path` via `getfacl -p -- <path>`.
pub fn read_acl(path: &Path) -> AclInfo {
    let path_str = path.to_string_lossy();
    match command::run("getfacl", &["-p", "--", &path_str]) {
        Err(err) => AclInfo {
            available: false,
            entries: Vec::new(),
            error: Some(format!(
                "getfacl unavailable (install `acl` package): {err}"
            )),
        },
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            AclInfo {
                available: true,
                entries: parse_getfacl_output(&stdout),
                error: None,
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            AclInfo {
                available: true,
                entries: Vec::new(),
                error: Some(if stderr.is_empty() {
                    "getfacl failed".to_string()
                } else {
                    stderr
                }),
            }
        }
    }
}

/// Parses `getfacl -p` output lines into structured `AclEntry` records.
pub fn parse_getfacl_output(text: &str) -> Vec<AclEntry> {
    let mut entries = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (main_part, effective) = match trimmed.split_once('#') {
            Some((left, right)) => {
                let eff = right
                    .trim()
                    .strip_prefix("effective:")
                    .map(|s| s.trim().to_string());
                (left.trim(), eff)
            }
            None => (trimmed, None),
        };

        let parts: Vec<&str> = main_part.split(':').collect();
        let (is_default, rest) = match parts.first().copied() {
            Some("default") => (true, &parts[1..]),
            _ => (false, &parts[..]),
        };
        if rest.len() != 3 {
            continue;
        }
        let scope = match rest[0] {
            "user" => AclScope::User,
            "group" => AclScope::Group,
            "mask" => AclScope::Mask,
            "other" => AclScope::Other,
            _ => continue,
        };
        entries.push(AclEntry {
            is_default,
            scope,
            qualifier: rest[1].to_string(),
            perms: rest[2].to_string(),
            effective,
        });
    }
    entries
}

/// Applies `setfacl [-R] -m <spec> -- <path>`.
pub fn set_acl(path: &Path, spec: &str, recursive: bool) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if recursive {
        vec!["-R", "-m", spec, "--", &path_str]
    } else {
        vec!["-m", spec, "--", &path_str]
    };
    run_setfacl(&args)
}

/// Removes an entry via `setfacl [-R] -x <spec> -- <path>`.
pub fn remove_acl(path: &Path, spec: &str, recursive: bool) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if recursive {
        vec!["-R", "-x", spec, "--", &path_str]
    } else {
        vec!["-x", spec, "--", &path_str]
    };
    run_setfacl(&args)
}

/// Strips all extended ACL entries via `setfacl [-R] -b -- <path>`.
pub fn clear_acl(path: &Path, recursive: bool) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let args: Vec<&str> = if recursive {
        vec!["-R", "-b", "--", &path_str]
    } else {
        vec!["-b", "--", &path_str]
    };
    run_setfacl(&args)
}

fn run_setfacl(args: &[&str]) -> Result<(), AppError> {
    let out = command::run("setfacl", args)?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(AppError::Io(if stderr.is_empty() {
            format!("setfacl exited with status {}", out.status)
        } else {
            stderr
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_getfacl_output_with_effective_and_default() {
        let sample = "\
# file: /var/www/app
# owner: root
# group: www-data
user::rwx
user:deploy:rwx\t#effective:r-x
group::r-x
mask::r-x
other::---
default:user::rwx
default:group:devs:r-x
";
        let entries = parse_getfacl_output(sample);
        assert_eq!(entries.len(), 7);
        assert_eq!(entries[1].scope, AclScope::User);
        assert_eq!(entries[1].qualifier, "deploy");
        assert_eq!(entries[1].perms, "rwx");
        assert_eq!(entries[1].effective.as_deref(), Some("r-x"));
        assert!(entries[5].is_default);
        assert_eq!(entries[6].qualifier, "devs");
    }
}
