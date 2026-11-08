# Changelog

All notable changes to useRA are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com); versioning
follows [SemVer](https://semver.org).

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
