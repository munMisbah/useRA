# useRA

**Linux User &amp; Group Manager**

> User Management, Simplified.

useRA is a modern Terminal User Interface (TUI) application for managing Linux users and groups. It is designed to provide an intuitive, keyboard-driven system administration experience inspired by modern terminal applications such as Superfile.

## Project Overview


| Property         | Value               |
| ---------------- | ------------------- |
| Project name     | useRA               |
| Language         | Rust                |
| TUI Framework    | Ratatui             |
| Terminal Backend | Crossterm           |
| Initial OS       | Debian              |
| Interface        | Keyboard-driven TUI |
| Theme            | Dark                |
| Architecture     | Modular             |
| Distribution     | Standalone binary   |


## Core Principles

- Modern and intuitive terminal interface.
- Keyboard-first navigation.
- Modular and maintainable codebase.
- Safe system administration operations.
- Clear error handling and user feedback.
- Minimal external dependencies where practical.
- No unnecessary complexity.

## Initial Scope

The first release targets Debian-based Linux systems.

The application must support standard Linux account management through native system utilities and account databases.

---

# Documentation

Detailed project documentation is maintained in the `docs/` directory.

- `PROJECT_SPEC.md`: Product requirements and scope.
- `ARCHITECTURE.md`: Application architecture.
- `UI_DESIGN.md`: Interface and interaction design.
- `FEATURES.md`: Feature specifications.
- `SECURITY.md`: Security requirements.
- `ROADMAP.md`: Development milestones.
- `INSTALL.md`: Installation guide (Debian 12/13).
- `MANUAL.md`: User manual.

---

# Getting Started

Development prerequisites:

- Rust toolchain.
- Cargo.
- Debian Linux.
- Terminal with UTF-8 support.

The application will require appropriate root privileges for administrative operations.

---

## Development Status

**Current release: 1.0.0** — all roadmap phases complete.

1. ~~Establish the Rust project.~~ (Phase 0)
2. ~~Implement the Ratatui terminal interface.~~ (Phase 1)
3. ~~Build the navigation system.~~ (Phase 1)
4. ~~Implement user listing and inspection.~~ (Phase 2)
5. ~~Implement user management operations.~~ (Phase 3 — run as root for write ops)
6. ~~Implement group management.~~ (Phase 4)
7. ~~Add security and audit features.~~ (Phase 5)
8. ~~Release preparation.~~ (Phase 6 — see [CHANGELOG.md](CHANGELOG.md))

### User management shortcuts (Users page, as root)

| Key | Action |
| --- | --- |
| A | Create user (form: username, full name, home, shell, groups) |
| E | Change login shell of selected user |
| L | Lock account (confirm with YES) |
| U | Unlock account (confirm with YES) |
| G | Add user to supplementary groups |
| X | Set/clear account expiry (YYYY-MM-DD) |
| D | Delete user (confirm with YES; home is kept) |

### Group management shortcuts (Groups page, as root)

| Key | Action |
| --- | --- |
| A | Create group (name + optional GID) |
| E | Rename selected group |
| G | Add members (comma-separated usernames) |
| R | Remove members (prefilled with current members) |
| D | Delete group (confirm with YES; user accounts untouched) |

### Security model

- **No auto-elevation**: run `sudo usera` for write operations. Without
  root, all operations are read-only and write attempts are refused with
  a clear message.
- **Audit log**: every executed operation is appended to
  `/var/log/usera/audit.log` (root) or
  `$XDG_STATE_HOME/usera/audit.log`. Format: timestamp, OK/FAIL,
  operation, target, error text. Passwords and hashes are never logged.
- **System-account guard**: deleting or locking `root` or any system
  account (UID < 1000) is refused inside the app.
- **Explicit confirmation**: destructive operations require typing YES;
  home removal is never implicit.

### Building and running

```sh
cargo build --release
./target/release/usera
```

The application reads account data live via `getent`, `uname`, and
`loginctl`; administrative (write) operations arrive in later phases and
will require root privileges.

### Nerd Font requirement

The interface uses Nerd Font glyphs for icons. Without one, icons render
as boxes (tofu) — the app still works, but looks degraded. Check your
setup with:

```sh
./scripts/check-font.sh
```

On Debian 12/13 no Nerd Font package exists in the repositories; the
script prints manual install steps (font file into `~/.local/share/fonts`
plus `fc-cache`). Debian repos only carry `fonts-powerline` /
`fonts-jetbrains-mono` (no glyphs), so an upstream install is required.

---

# Project Philosophy

useRA should feel like a modern terminal application rather than a traditional collection of Linux commands.

The interface should remain clean, predictable, responsive, and easy to navigate.