//! Audit logging for application-initiated administrative operations
//! (SECURITY.md §7, FEATURES.md §7).
//!
//! - Log file: `/var/log/usera/audit.log` when running as root,
//!   else `~/.local/state/usera/audit.log` (XDG state dir) so unprivileged
//!   sessions still record what they attempted.
//! - Never logged: passwords, hashes, shadow content. Only operation,
//!   target, result, and error text.
//! - Best-effort: a logging failure never blocks the operation result.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// One audit record.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Operation name, e.g. "create_user".
    pub operation: &'static str,
    /// Target account or group.
    pub target: String,
    /// Success flag.
    pub ok: bool,
    /// Error text when the operation failed (may be empty).
    pub error: String,
}

impl AuditEntry {
    /// Builds an entry from an operation result.
    pub fn from_result(operation: &'static str, target: &str, ok: bool, message: &str) -> Self {
        AuditEntry {
            operation,
            target: target.to_string(),
            ok,
            error: if ok {
                String::new()
            } else {
                message.to_string()
            },
        }
    }
}

/// Log file path for the current privilege level.
pub fn log_path() -> PathBuf {
    if crate::system::privilege::is_root() {
        PathBuf::from("/var/log/usera/audit.log")
    } else {
        let state = std::env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".local/state")
            });
        state.join("usera/audit.log")
    }
}

/// RFC-3339-ish UTC timestamp without external crates: seconds since
/// epoch plus a coarse day-based fallback is NOT human friendly, so we
/// render a compact UTC timestamp via days->date math.
fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

/// Converts Unix seconds to `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days
/// algorithm, Howard Hinnant).
fn format_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{y:04}-{mth:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Appends an entry to the audit log. Returns Err text on I/O failure
/// (caller decides whether to surface it; it never replaces the
/// operation result).
pub fn record(entry: &AuditEntry) -> Result<(), String> {
    record_to_path(&log_path(), entry)
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64")
))]
const O_NOFOLLOW: i32 = 0o400000;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64")
))]
const O_DIRECTORY: i32 = 0o200000;

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const O_NOFOLLOW: i32 = 0o100000;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const O_DIRECTORY: i32 = 0o40000;

#[cfg(not(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "riscv64",
        target_arch = "aarch64"
    )
)))]
const O_NOFOLLOW: i32 = 0;
#[cfg(not(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "riscv64",
        target_arch = "aarch64"
    )
)))]
const O_DIRECTORY: i32 = 0;

fn record_to_path(path: &std::path::Path, entry: &AuditEntry) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

    let expected_uid = crate::system::privilege::effective_uid();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o750)
                .create(parent)
            {
                if !parent.exists() {
                    return Err(format!("cannot create {}: {err}", parent.display()));
                }
            }
            let dir_fd = OpenOptions::new()
                .read(true)
                .custom_flags(O_NOFOLLOW | O_DIRECTORY)
                .open(parent)
                .map_err(|e| format!("refusing audit dir {}: {e}", parent.display()))?;
            let dir_meta = dir_fd
                .metadata()
                .map_err(|e| format!("cannot fstat {}: {e}", parent.display()))?;
            if !dir_meta.is_dir() {
                return Err(format!(
                    "refusing audit dir {}: not a directory",
                    parent.display()
                ));
            }
            if dir_meta.uid() != expected_uid {
                return Err(format!(
                    "refusing audit dir {}: unexpected owner UID {}",
                    parent.display(),
                    dir_meta.uid()
                ));
            }
            dir_fd
                .set_permissions(std::fs::Permissions::from_mode(0o750))
                .map_err(|e| format!("cannot fchmod {}: {e}", parent.display()))?;
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let file_meta = file
        .metadata()
        .map_err(|e| format!("cannot fstat {}: {e}", path.display()))?;
    if !file_meta.is_file() {
        return Err(format!(
            "refusing audit file {}: not a regular file",
            path.display()
        ));
    }
    if file_meta.uid() != expected_uid {
        return Err(format!(
            "refusing audit file {}: unexpected owner UID {}",
            path.display(),
            file_meta.uid()
        ));
    }
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("cannot fchmod {}: {e}", path.display()))?;

    let status = if entry.ok { "OK" } else { "FAIL" };
    let mut line = format!(
        "{} {} op={} target={}",
        timestamp(),
        status,
        entry.operation,
        entry.target
    );
    if !entry.error.is_empty() {
        // Keep single-line; strip control chars from tool stderr.
        let clean: String = entry.error.chars().filter(|c| !c.is_control()).collect();
        line.push_str(&format!(" err=\"{clean}\""));
    }
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_timestamp() {
        // 2026-01-01T00:00:00Z = 1767225600
        assert_eq!(format_utc(1_767_225_600), "2026-01-01T00:00:00Z");
        // 2026-11-09T00:00:00Z = 1794182400
        assert_eq!(format_utc(1_794_182_400), "2026-11-09T00:00:00Z");
    }

    #[test]
    fn entries_from_results_carry_error_only_on_failure() {
        let ok = AuditEntry::from_result("create_user", "deploy", true, "user deploy created");
        assert!(ok.error.is_empty());

        let bad = AuditEntry::from_result("delete_user", "root", false, "Permission denied");
        assert_eq!(bad.error, "Permission denied");
        assert_eq!(bad.operation, "delete_user");
    }

    #[test]
    fn records_append_to_state_log() {
        // Unprivileged run writes to the state dir; proves end-to-end I/O.
        let entry = AuditEntry::from_result("unit_test", "sample", true, "");
        record(&entry).expect("audit write works");
        let path = log_path();
        let content = std::fs::read_to_string(&path).expect("log readable");
        assert!(
            content.contains("op=unit_test target=sample"),
            "in {content}"
        );
    }

    #[test]
    fn error_text_is_single_line() {
        let entry = AuditEntry::from_result("unit_test", "x", false, "line1\nline2\tpwned\u{7}");
        record(&entry).expect("write works");
        let content = std::fs::read_to_string(log_path()).expect("readable");
        let last = content.lines().last().expect("has lines");
        assert!(!last.contains('\n'));
        assert!(last.contains("line1line2pwned"));
    }

    #[test]
    fn audit_dir_and_file_have_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let entry = AuditEntry::from_result("perm_test", "u", true, "");
        record(&entry).expect("audit write works");
        let path = log_path();
        let file_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600);
        let dir_mode = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o750);
    }

    #[test]
    fn rejects_symlink_log_file_and_symlink_parent_atomically() {
        let base = std::env::temp_dir().join(format!("usera_audit_toctou_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let victim = base.join("victim.txt");
        std::fs::write(&victim, b"untouched\n").unwrap();

        // Symlink as audit file must fail atomically via O_NOFOLLOW without modifying target.
        let symlink_log = base.join("audit.log");
        std::os::unix::fs::symlink(&victim, &symlink_log).unwrap();
        let entry = AuditEntry::from_result("toctou_test", "x", true, "");
        let err = record_to_path(&symlink_log, &entry).unwrap_err();
        assert!(err.contains("cannot open"), "unexpected error: {err}");
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched\n");

        // Symlink as audit parent directory must fail via O_NOFOLLOW | O_DIRECTORY.
        let real_dir = base.join("real_dir");
        std::fs::create_dir_all(&real_dir).unwrap();
        let symlink_dir = base.join("sym_dir");
        std::os::unix::fs::symlink(&real_dir, &symlink_dir).unwrap();
        let err_dir = record_to_path(&symlink_dir.join("audit.log"), &entry).unwrap_err();
        assert!(err_dir.contains("refusing audit dir"), "got: {err_dir}");

        let _ = std::fs::remove_dir_all(&base);
    }
}
