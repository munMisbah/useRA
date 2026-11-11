//! Filesystem browser and metadata inspection for Resource Manager.
//!
//! Uses `symlink_metadata` (`lstat`) so symlinks are never dereferenced
//! silently during directory listing or recursive item counting.

use crate::errors::AppError;
use crate::models::{EntryKind, Group, ResourceEntry, User};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Returns the initial directory for the Resource Manager browser.
pub fn default_start_dir() -> PathBuf {
    std::env::current_dir()
        .ok()
        .filter(|p| fs::read_dir(p).is_ok())
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Lists entries in `dir` (directories first, then files, sorted by name).
/// Inserts a synthetic `..` entry at the top when `dir` has a parent.
pub fn list_dir(
    dir: &Path,
    users: &[User],
    groups: &[Group],
) -> Result<Vec<ResourceEntry>, AppError> {
    let read = fs::read_dir(dir)
        .map_err(|e| AppError::Io(format!("cannot read {}: {e}", dir.display())))?;

    let mut dirs: Vec<ResourceEntry> = Vec::new();
    let mut others: Vec<ResourceEntry> = Vec::new();

    for item in read {
        let entry = match item {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let ft = meta.file_type();
        let (kind, symlink_target) = if ft.is_symlink() {
            let target = fs::read_link(&path)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            (EntryKind::Symlink, target)
        } else if ft.is_dir() {
            (EntryKind::Directory, None)
        } else if ft.is_file() {
            (EntryKind::File, None)
        } else {
            (EntryKind::Other, None)
        };

        let uid = meta.uid();
        let gid = meta.gid();
        let item_entry = ResourceEntry {
            name,
            path,
            kind,
            mode: meta.mode() & 0o7777,
            uid,
            gid,
            owner_name: resolve_owner(uid, users),
            group_name: resolve_group(gid, groups),
            size: meta.size(),
            is_parent_link: false,
            symlink_target,
        };

        if kind == EntryKind::Directory {
            dirs.push(item_entry);
        } else {
            others.push(item_entry);
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    others.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut result = Vec::with_capacity(1 + dirs.len() + others.len());
    if let Some(parent) = dir.parent() {
        let meta = fs::symlink_metadata(parent).ok();
        let uid = meta.as_ref().map(|m| m.uid()).unwrap_or(0);
        let gid = meta.as_ref().map(|m| m.gid()).unwrap_or(0);
        result.push(ResourceEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            kind: EntryKind::Directory,
            mode: meta.as_ref().map(|m| m.mode() & 0o7777).unwrap_or(0o755),
            uid,
            gid,
            owner_name: resolve_owner(uid, users),
            group_name: resolve_group(gid, groups),
            size: 0,
            is_parent_link: true,
            symlink_target: None,
        });
    }
    result.extend(dirs);
    result.extend(others);
    Ok(result)
}

/// Counts items under `path` (including `path` itself) without following symlinks,
/// stopping once `limit` is reached for snappy confirmation previews.
pub fn count_recursive_items(path: &Path, limit: usize) -> (usize, bool) {
    let mut stack = vec![path.to_path_buf()];
    let mut count = 0;

    while let Some(curr) = stack.pop() {
        count += 1;
        if count >= limit {
            return (count, true);
        }
        if let Ok(meta) = fs::symlink_metadata(&curr) {
            if meta.file_type().is_dir() {
                if let Ok(entries) = fs::read_dir(&curr) {
                    for entry in entries.flatten() {
                        stack.push(entry.path());
                    }
                }
            }
        }
    }
    (count, false)
}

fn resolve_owner(uid: u32, users: &[User]) -> String {
    users
        .iter()
        .find(|u| u.uid == uid)
        .map(|u| u.name.clone())
        .unwrap_or_else(|| uid.to_string())
}

fn resolve_group(gid: u32, groups: &[Group]) -> String {
    groups
        .iter()
        .find(|g| g.gid == gid)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| gid.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_temp_dir_with_parent_and_sorted_dirs_first() {
        let base = std::env::temp_dir().join(format!("usera_fs_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("z_dir")).unwrap();
        fs::write(base.join("a_file.txt"), b"hi").unwrap();

        let entries = list_dir(&base, &[], &[]).unwrap();
        assert_eq!(entries[0].name, "..");
        assert!(entries[0].is_parent_link);
        assert_eq!(entries[1].name, "z_dir");
        assert_eq!(entries[1].kind, EntryKind::Directory);
        assert_eq!(entries[2].name, "a_file.txt");
        assert_eq!(entries[2].kind, EntryKind::File);

        let (count, truncated) = count_recursive_items(&base, 100);
        assert_eq!(count, 3);
        assert!(!truncated);

        let _ = fs::remove_dir_all(&base);
    }
}
