//! Self-updater for the `usera` binary.
//!
//! Resolves the latest GitHub Release tag without hitting JSON API rate limits,
//! compares versions before downloading, fetches version-pinned release
//! artifacts, verifies the SHA-256 checksum, and installs atomically without
//! executing remote shell scripts (`bash -s`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_REPO: &str = "munMisbah/useRA";
const ASSET_TAR: &str = "usera-linux-x86_64.tar.gz";
const ASSET_SHA: &str = "usera-linux-x86_64.tar.gz.sha256";
const DEFAULT_INSTALL_BIN: &str = "/usr/local/bin/usera";

struct TempDir(PathBuf);

impl TempDir {
    fn create() -> Result<Self, String> {
        let out = Command::new("mktemp")
            .arg("-d")
            .output()
            .map_err(|e| format!("failed to run mktemp: {e}"))?;
        if !out.status.success() {
            return Err("mktemp -d failed".to_string());
        }
        let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        Ok(TempDir(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// CLI entrypoint for `usera update`.
pub fn run() -> io::Result<()> {
    if let Err(err) = run_inner() {
        eprintln!("\x1b[1;31museRA update failed:\x1b[0m {err}");
        std::process::exit(1);
    }
    Ok(())
}

fn run_inner() -> Result<(), String> {
    if std::env::consts::OS != "linux" {
        return Err(format!(
            "unsupported OS '{}' — useRA targets Linux",
            std::env::consts::OS
        ));
    }
    if std::env::consts::ARCH != "x86_64" {
        return Err(format!(
            "unsupported architecture '{}' — x86_64 only for now",
            std::env::consts::ARCH
        ));
    }

    for tool in ["tar", "sha256sum", "install"] {
        if !command_exists(tool) {
            return Err(format!("'{tool}' is required to update"));
        }
    }
    let has_curl = command_exists("curl");
    let has_wget = command_exists("wget");
    if !has_curl && !has_wget {
        return Err("curl or wget is required to update".to_string());
    }

    let repo = resolve_repo()?;
    let current = env!("CARGO_PKG_VERSION");
    println!("\x1b[1;35museRA\x1b[0m Checking for updates (current: v{current})...");

    let latest_tag = fetch_latest_tag(&repo, has_curl)?;
    let current_ver =
        parse_semver(current).ok_or_else(|| format!("invalid compiled version '{current}'"))?;
    let latest_ver = parse_semver(&latest_tag)
        .ok_or_else(|| format!("unexpected release tag '{latest_tag}'"))?;

    if latest_ver <= current_ver {
        println!("\x1b[1;35museRA\x1b[0m already up to date (v{current})");
        return Ok(());
    }

    println!("\x1b[1;35museRA\x1b[0m downloading {latest_tag} from github.com/{repo}");
    let tmp = TempDir::create()?;
    let tar_path = tmp.path().join(ASSET_TAR);
    let sha_path = tmp.path().join(ASSET_SHA);

    let base_url = format!("https://github.com/{repo}/releases/download/{latest_tag}");
    download_file(&format!("{base_url}/{ASSET_TAR}"), &tar_path, has_curl)?;
    download_file(&format!("{base_url}/{ASSET_SHA}"), &sha_path, has_curl)?;

    let sha_content =
        fs::read_to_string(&sha_path).map_err(|e| format!("cannot read checksum file: {e}"))?;
    let expected = parse_sha256_line(&sha_content)?;
    let actual = compute_sha256(&tar_path)?;
    if !expected.eq_ignore_ascii_case(&actual) {
        return Err("checksum mismatch — download corrupted or tampered".to_string());
    }
    println!("\x1b[1;35museRA\x1b[0m checksum verified ({actual})");

    let extract_status = Command::new("tar")
        .args(["-xzf"])
        .arg(&tar_path)
        .arg("-C")
        .arg(tmp.path())
        .arg("usera")
        .status()
        .map_err(|e| format!("failed to run tar: {e}"))?;
    if !extract_status.success() {
        return Err("failed to extract 'usera' binary from archive".to_string());
    }

    let extracted = tmp.path().join("usera");
    let meta = fs::symlink_metadata(&extracted)
        .map_err(|_| "archive did not contain the 'usera' binary".to_string())?;
    if !meta.file_type().is_file() {
        return Err("extracted 'usera' entry is not a regular file".to_string());
    }

    let target = install_target();
    install_binary(&extracted, &target)?;

    println!(
        "\x1b[1;35museRA\x1b[0m updated to {latest_tag} at {}",
        target.display()
    );
    Ok(())
}

fn resolve_repo() -> Result<String, String> {
    let raw = std::env::var("USERA_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());
    validate_repo_slug(&raw)?;
    Ok(raw)
}

fn validate_repo_slug(slug: &str) -> Result<(), String> {
    let parts: Vec<&str> = slug.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        })
    {
        return Err(format!(
            "invalid USERA_REPO slug '{slug}' (expected owner/repo)"
        ));
    }
    Ok(())
}

fn fetch_latest_tag(repo: &str, use_curl: bool) -> Result<String, String> {
    let latest_url = format!("https://github.com/{repo}/releases/latest");
    if use_curl {
        let out = Command::new("curl")
            .args([
                "-fsSL",
                "-o",
                "/dev/null",
                "-w",
                "%{url_effective}",
                &latest_url,
            ])
            .output()
            .map_err(|e| format!("failed to run curl: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("could not resolve latest release: {err}"));
        }
        let effective = String::from_utf8_lossy(&out.stdout);
        extract_tag_from_url(effective.trim())
    } else {
        let out = Command::new("wget")
            .args(["--spider", "-S", "--max-redirect=0", &latest_url])
            .output()
            .map_err(|e| format!("failed to run wget: {e}"))?;
        let stderr = String::from_utf8_lossy(&out.stderr);
        for line in stderr.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed
                .strip_prefix("Location:")
                .or_else(|| trimmed.strip_prefix("location:"))
            {
                if let Ok(tag) = extract_tag_from_url(rest.trim()) {
                    return Ok(tag);
                }
            }
        }
        Err("could not resolve latest release tag from GitHub".to_string())
    }
}

fn extract_tag_from_url(url: &str) -> Result<String, String> {
    let tag = url
        .rsplit_once("/releases/tag/")
        .map(|(_, t)| t.trim_end_matches('/'))
        .ok_or_else(|| format!("no release tag found in redirect URL '{url}'"))?;
    if tag.is_empty()
        || !tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
    {
        return Err(format!("invalid release tag '{tag}'"));
    }
    Ok(tag.to_string())
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let stripped = raw.strip_prefix('v').unwrap_or(raw);
    let core = stripped.split('-').next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn download_file(url: &str, dest: &Path, use_curl: bool) -> Result<(), String> {
    let status = if use_curl {
        Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(dest)
            .arg(url)
            .status()
            .map_err(|e| format!("failed to run curl: {e}"))?
    } else {
        Command::new("wget")
            .args(["-qO"])
            .arg(dest)
            .arg(url)
            .status()
            .map_err(|e| format!("failed to run wget: {e}"))?
    };
    if !status.success() {
        return Err(format!("download failed: {url}"));
    }
    Ok(())
}

fn parse_sha256_line(content: &str) -> Result<String, String> {
    let hex = content
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty checksum file".to_string())?;
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("malformed sha256 checksum '{hex}'"));
    }
    Ok(hex.to_string())
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    let out = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|e| format!("failed to run sha256sum: {e}"))?;
    if !out.status.success() {
        return Err("sha256sum failed".to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_sha256_line(&stdout)
}

fn install_target() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let s = exe.to_string_lossy();
        if !s.contains("/target/") {
            return exe;
        }
    }
    PathBuf::from(DEFAULT_INSTALL_BIN)
}

fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".usera-write-probe-{}", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn install_binary(src: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .unwrap_or_else(|| Path::new("/usr/local/bin"));
    if crate::system::privilege::is_root() || dir_is_writable(parent) {
        let status = Command::new("install")
            .args(["-m", "755"])
            .arg(src)
            .arg(target)
            .status()
            .map_err(|e| format!("failed to run install: {e}"))?;
        if !status.success() {
            return Err(format!("install to {} failed", target.display()));
        }
        Ok(())
    } else if command_exists("sudo") {
        println!(
            "\x1b[1;35museRA\x1b[0m requesting sudo to install to {}",
            target.display()
        );
        let status = Command::new("sudo")
            .args(["install", "-m", "755"])
            .arg(src)
            .arg(target)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| format!("failed to run sudo: {e}"))?;
        if !status.success() {
            return Err("sudo install failed".to_string());
        }
        Ok(())
    } else {
        Err(format!(
            "no write permission to {} and sudo is unavailable",
            parent.display()
        ))
    }
}

fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if Path::new(dir).join(cmd).is_file() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_tags_from_redirect_urls() {
        assert_eq!(
            extract_tag_from_url("https://github.com/munMisbah/useRA/releases/tag/v1.0.3").unwrap(),
            "v1.0.3"
        );
        assert_eq!(
            extract_tag_from_url("https://github.com/munMisbah/useRA/releases/tag/v1.2.0/")
                .unwrap(),
            "v1.2.0"
        );
        assert!(extract_tag_from_url("https://github.com/munMisbah/useRA/releases").is_err());
        assert!(extract_tag_from_url("https://example.com/releases/tag/v1/../../evil").is_err());
    }

    #[test]
    fn compares_semver_versions_accurately() {
        assert!(parse_semver("v1.0.4") > parse_semver("1.0.3"));
        assert!(parse_semver("v1.1.0") > parse_semver("1.0.99"));
        assert_eq!(parse_semver("v1.0.3"), parse_semver("1.0.3"));
        assert!(parse_semver("bad").is_none());
    }

    #[test]
    fn validates_repo_slugs_strictly() {
        assert!(validate_repo_slug("munMisbah/useRA").is_ok());
        assert!(validate_repo_slug("evil/repo/extra").is_err());
        assert!(validate_repo_slug("no-slash").is_err());
        assert!(validate_repo_slug("bad;cmd/repo").is_err());
    }

    #[test]
    fn parses_sha256_checksum_lines() {
        let hex = "eb0a4d9735d8c4c0675545969a082f95aa8325c09b134d729aabeac3c7353255";
        assert_eq!(
            parse_sha256_line(&format!("{hex}  usera-linux-x86_64.tar.gz\n")).unwrap(),
            hex
        );
        assert!(parse_sha256_line("not-a-sha").is_err());
    }
}
