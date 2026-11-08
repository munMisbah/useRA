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

**Current phase:** UI foundation and read-only system integration complete.

The initial development priorities are:

1. ~~Establish the Rust project.~~ (Phase 0)
2. ~~Implement the Ratatui terminal interface.~~ (Phase 1)
3. ~~Build the navigation system.~~ (Phase 1)
4. ~~Implement user listing and inspection.~~ (Phase 2)
5. Implement user management operations. (Phase 3)
6. Implement group management. (Phase 4)
7. Add security and audit features. (Phase 5)

### Building and running

```sh
cargo build --release
./target/release/usera
```

The application reads account data live via `getent`, `uname`, and
`loginctl`; administrative (write) operations arrive in later phases and
will require root privileges.

---

# Project Philosophy

useRA should feel like a modern terminal application rather than a traditional collection of Linux commands.

The interface should remain clean, predictable, responsive, and easy to navigate.