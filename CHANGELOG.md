# Changelog

All notable changes to useRA are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com); versioning
follows [SemVer](https://semver.org).

## Release artifacts

For each release, attach to the GitHub release:

- `usera-linux-x86_64.tar.gz` — contains the `usera` binary
- `usera-linux-x86_64.tar.gz.sha256` — `sha256sum` of the tarball

`install.sh` (repo root) consumes exactly these names. Build both with:

```sh
cargo build --release
tar -czf usera-linux-x86_64.tar.gz -C target/release usera
sha256sum usera-linux-x86_64.tar.gz > usera-linux-x86_64.tar.gz.sha256
```

## [1.1.1] — 2026-11-11

### Security & Fixed

- **Atomic Audit Log Opening (`src/system/audit.rs`, `privilege.rs`)**:
  Eliminated TOCTOU symlink race conditions by opening the audit directory
  with `O_NOFOLLOW | O_DIRECTORY` and `audit.log` with `O_NOFOLLOW`, verifying
  regular-file/dir types and EUID ownership via `fstat`, and applying
  permissions (`0750` / `0600`) via `fchmod` on the open descriptors.
- **POSIX/ACL Readout Clarity & Root UID 0 Handling (`src/models/resource.rs`, `ui/pages/resources.rs`)**:
  Clarified the Hybrid Details label as `POSIX/ACL for`, handled `UID 0`
  (`root`) DAC bypass accurately (`rwx (root bypass)` / `rw- (root bypass; no +x)`),
  and documented filesystem/kernel permission boundaries (`vfat`/`exFAT`,
  `ro`/`noexec` mounts, `chattr +i`, AppArmor, systemd sandboxing) in
  `docs/MANUAL.md` and `docs/SECURITY.md`.

## [1.1.0] — 2026-11-11

### Added

- **Hybrid Resource & Permission Manager**:
  - Dedicated `Resources` page (`Name | Mode | Owner` browser, `/` live filter,
    right-hand `Details & ACL` panel).
  - Contextual Hybrid layout on `Users` & `Groups` (auto-enabled on
    `>= 105x24` terminals, toggleable with `F`): `Resources` browser on the
    right with compact `Details` + live POSIX.1e `Effective for '<name>'`
    evaluation underneath.
  - Interactive `chmod` checkbox grid (`M`) covering Owner/Group/Other `r/w/x`
    plus Special bits (`SUID`, `SGID`, `Sticky`) and recursive (`-R`).
  - Interactive `Set ACL` modal (`A`) with Scope selector (`User / Group / Mask /
    Other`), 1+ char type-to-filter autocomplete (`Tab` complete), `r/w/x`
    checkboxes, `Default ACL (d:)`, and recursive (`-R`).
  - `Remove ACL` checklist (`X`) populated directly from active `getfacl` entries
    (supporting multi-entry `-x` and default ACL clear `-k`).
  - `Clear ACLs` (`B`, `setfacl -b`), `chown` (`O`), direct path jump (`G`),
    selected user home jump (`H` in Hybrid mode), and dotfiles toggle (`.`).
  - Batch `+` ACL indicators across directory rows via `getfacl -s -p -P`.
  - Safety gates for recursive (`-R`) operations: refuses protected OS root
    paths (`/`, `/etc`, `/usr`, `/var`, etc.) and requires item-count preview +
    explicit `YES` confirmation.
- **Live Search & List Scrolling**:
  - Interactive `/` search filtering on `Users`, `Groups`, `Sessions`, and
    `Resources`.
  - Stateful table scrolling (`TableState` + vertical `Scrollbar`) on all
    list views.
- **CLI Self-Updater & Flags**:
  - Direct version-aware `usera update` (`src/system/updater.rs`): checks
    latest release tag before downloading, verifies SHA-256 on version-pinned
    artifacts, and replaces the binary atomically without piping remote
    scripts into `bash -s`.
  - `--version` / `-v` and `--help` / `-h` CLI flags.

### Changed

- Redesigned borderless left sidebar, removed top status strip, added outer
  padding, and updated dark theme palette (`#141414` / `#332c27` / `#F56A00`).
- Replaced `USER` / `USERS` icons with Nerd Font v3 `fa-user` (`\u{f007}`) and
  `fa-users` (`\u{f0c0}`).
- Fixed `Left` arrow focus movement (`previous_focus`) and `install.sh` hang
  when updating over older binaries without `--version`.

### Security

- Enforced explicit permissions on audit directory (`0750`) and `audit.log`
  (`0600`) with symlink guards (`symlink_metadata`).
- Fixed partial-operation audit logging in `submit_create_user` so `create_user`,
  `add_to_groups`, and `set_password` outcomes are all recorded.
- Added absolute path checks for `home` & `shell` in `create_user`, `--`
  separators across `user_service` and `group_service`, and strict input
  validators for modes, owners, and ACL specs.

## [1.0.3] — 2026-11-10

### Added

- Masked password field (optional) in Create User (`A`) modal.
- Set/change password shortcut (`P`) on the Users page with confirmation
  field, piped safely to `chpasswd` via stdin (never logged or exposed).

## [1.0.0] — 2026-11-09

Initial release. Full user and group management for Debian with an
audit-logged, safety-gated TUI.

### Added

- Split-panel TUI (header / navigation / content / details / status bar)
  inspired by Superfile: dark theme, Nerd Font icons, rounded borders,
  responsive (< 80 cols hides details, < 60 collapses navigation).
- Five pages: Dashboard, Users, Groups, Sessions, Settings — all data
  read live from the system (`getent`, `uname`, `loginctl`,
  `/etc/os-release`), never from `/etc/shadow`.
- User management (as root): create (`useradd`), lock/unlock
  (`passwd -l/-u`), change shell (`usermod -s`), set/clear expiry
  (`chage -E`), modify supplementary groups (`usermod -aG`), delete
  (`userdel`, home kept by default).
- Group management (as root): create (`groupadd`, optional GID), rename
  (`groupmod -n`), delete (`groupdel`), add/remove members
  (`gpasswd -a/-d`, per-user error reporting).
- Form modal system: multi-field forms with caret editing, inline
  validation, YES-confirmation for destructive operations.
- Security model: no auto-elevation (write ops require `sudo usera`);
  system-account guard refuses delete/lock of root and UID < 1000;
  client-side validation per Debian name policy; separate-argument
  command execution only (no shell interpolation).
- Audit logging: every executed operation appended to
  `/var/log/usera/audit.log` (root) or `$XDG_STATE_HOME/usera/audit.log`;
  UTC timestamps; no secrets logged.
- Friendly error reporting: raw tool errors mapped to actionable hints.
- Responsive layout, keyboard-first navigation, context-sensitive
  shortcut bar (hints appear only for working features).
- 64 unit/integration tests; clippy- and fmt-clean.

### Platform

- Debian 12 (bookworm) and Debian 13 (trixie), x86-64.
- Rust ≥ 1.88 to build; runtime glibc ≥ 2.34 (2.39 symbols weakly
  referenced, used only when present).
- Optimized release binary (~620 KiB, LTO, stripped).
