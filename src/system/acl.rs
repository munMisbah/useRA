//! POSIX ACL integration (`getfacl` / `setfacl`).
//!
//! Gracefully reports `available: false` when the `acl` package is not
//! installed so standard permission features remain fully usable.

use crate::errors::AppError;
use crate::models::{AclEntry, AclInfo, AclScope};
use crate::system::command;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Reads POSIX ACLs for `path` via `getfacl -p -P -- <path>`.
pub fn read_acl(path: &Path) -> AclInfo {
    let path_str = path.to_string_lossy();
    match command::run("getfacl", &["-p", "-P", "--", &path_str]) {
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

/// Batch-checks which paths carry extended POSIX ACLs via `getfacl -s -p -P`.
/// Skips symlinks (`-P`) and returns quickly even if individual entries error.
pub fn paths_with_acl(paths: &[PathBuf]) -> HashSet<PathBuf> {
    let mut result = HashSet::new();
    if paths.is_empty() {
        return result;
    }
    for chunk in paths.chunks(200) {
        let mut args: Vec<&str> = Vec::with_capacity(4 + chunk.len());
        args.extend(["-s", "-p", "-P", "--"]);
        let strs: Vec<String> = chunk
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        for s in &strs {
            args.push(s.as_str());
        }
        if let Ok(out) = command::run("getfacl", &args) {
            let stdout = String::from_utf8_lossy(&out.stdout);
            parse_acl_file_headers(&stdout, &mut result);
        }
    }
    result
}

/// Extracts `# file: <path>` headers produced by `getfacl -s -p`.
pub fn parse_acl_file_headers(text: &str, out: &mut HashSet<PathBuf>) {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# file: ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                out.insert(PathBuf::from(trimmed));
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
pub fn set_acl(paths: &[PathBuf], spec: &str, recursive: bool) -> Result<(), AppError> {
    run_setfacl_paths(paths, &["-m", spec], recursive)
}

/// Removes an entry via `setfacl [-R] -x <spec> -- <paths...>`.
pub fn remove_acl(paths: &[PathBuf], spec: &str, recursive: bool) -> Result<(), AppError> {
    run_setfacl_paths(paths, &["-x", spec], recursive)
}

/// Strips all default ACL entries (`setfacl [-R] -k -- <paths...>`).
pub fn clear_default_acl(paths: &[PathBuf], recursive: bool) -> Result<(), AppError> {
    run_setfacl_paths(paths, &["-k"], recursive)
}

/// Strips all extended ACL entries via `setfacl [-R] -b -- <paths...>`.
pub fn clear_acl(paths: &[PathBuf], recursive: bool) -> Result<(), AppError> {
    run_setfacl_paths(paths, &["-b"], recursive)
}

/// Exports headerless ACL rules (`getfacl -c -p -P -- <path>`) suitable for `--set-file=-`.
pub fn export_acl(path: &Path) -> Result<String, AppError> {
    let path_str = path.to_string_lossy();
    let out = command::run("getfacl", &["-c", "-p", "-P", "--", &path_str])?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(AppError::Io(if stderr.is_empty() {
            format!("getfacl exited with status {}", out.status)
        } else {
            stderr
        }));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if lines.is_empty() {
        return Err(AppError::Io("no ACL rules returned by getfacl".to_string()));
    }
    Ok(lines.join("\n"))
}

/// Replaces ACLs on `paths` by piping `acl_text` to `setfacl [-R] --set-file=-`.
pub fn apply_acl_text(paths: &[PathBuf], acl_text: &str, recursive: bool) -> Result<(), AppError> {
    let payload = format!("{}\n", acl_text.trim());
    for chunk in paths.chunks(200) {
        let mut args: Vec<&str> = Vec::with_capacity(3 + chunk.len());
        if recursive {
            args.push("-R");
        }
        args.push("--set-file=-");
        args.push("--");
        let strs: Vec<String> = chunk
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        for s in &strs {
            args.push(s.as_str());
        }
        let out = command::run_with_stdin("setfacl", &args, &payload)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(AppError::Io(if stderr.is_empty() {
                format!("setfacl exited with status {}", out.status)
            } else {
                stderr
            }));
        }
    }
    Ok(())
}

fn run_setfacl_paths(paths: &[PathBuf], op_args: &[&str], recursive: bool) -> Result<(), AppError> {
    for chunk in paths.chunks(200) {
        let mut args: Vec<&str> = Vec::with_capacity(2 + op_args.len() + chunk.len());
        if recursive {
            args.push("-R");
        }
        args.extend_from_slice(op_args);
        args.push("--");
        let strs: Vec<String> = chunk
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        for s in &strs {
            args.push(s.as_str());
        }
        run_setfacl(&args)?;
    }
    Ok(())
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

    #[test]
    fn parses_batch_acl_headers_from_skip_base_output() {
        let out = "\
# file: /var/www/app
# owner: root
# group: www-data
user::rwx
user:deploy:rwx

# file: /var/www/with space
user::rw-
group::r--
";
        let mut set = HashSet::new();
        parse_acl_file_headers(out, &mut set);
        assert!(set.contains(&PathBuf::from("/var/www/app")));
        assert!(set.contains(&PathBuf::from("/var/www/with space")));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn exports_and_clones_acl_text_across_temp_dirs() {
        let base = std::env::temp_dir().join(format!("usera_acl_clone_sys_{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        set_acl(&[src.clone()], "u:root:rwx,d:u:root:rwx", false).expect("setfacl works");
        let exported = export_acl(&src).expect("export_acl works");
        assert!(exported.contains("user:root:rwx"));
        assert!(exported.contains("default:user:root:rwx"));

        apply_acl_text(&[dst.clone()], &exported, false).expect("apply_acl_text works");
        let dst_acl = read_acl(&dst);
        assert!(dst_acl
            .entries
            .iter()
            .any(|e| !e.is_default && e.scope == AclScope::User && e.qualifier == "root"));
        assert!(dst_acl
            .entries
            .iter()
            .any(|e| e.is_default && e.scope == AclScope::User && e.qualifier == "root"));

        let _ = std::fs::remove_dir_all(&base);
    }
}
