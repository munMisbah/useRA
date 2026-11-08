# AI Coding Agent Instructions — useRA

## 1. Project Identity

You are working on **useRA**, a Linux User &amp; Group Management TUI application.

The application is written in Rust and uses Ratatui with Crossterm.

Initial target platform: Debian Linux.

## 2. Mandatory Technology Stack

Use the following technologies:

- Rust as the primary programming language.
- Ratatui for terminal UI rendering.
- Crossterm for terminal backend and input handling.
- Cargo for project and dependency management.

Do not replace the selected framework or programming language without explicit approval.

## 3. Development Rules

- Follow idiomatic Rust practices.
- Prefer clear and maintainable code.
- Use modular architecture.
- Avoid unnecessary abstractions.
- Keep UI rendering separate from business logic.
- Keep Linux system operations separate from UI components.
- Handle errors explicitly.
- Avoid unnecessary dependencies.

## 4. UI Guidelines

The interface must follow the approved Split Panel design inspired by Superfile.

Main layout:

1. Header.
2. Left navigation sidebar.
3. Center content panel.
4. Right detail panel.
5. Bottom status and shortcut bar.

Use a consistent dark theme.

The interface must support keyboard navigation.

Do not redesign the main layout without approval.

## 5. Linux Integration

Use native Debian/Linux account management mechanisms.

Expected utilities include:

- `getent`
- `useradd`
- `usermod`
- `userdel`
- `groupadd`
- `groupmod`
- `groupdel`
- `passwd`
- `loginctl` (where available)

Never construct shell commands by concatenating untrusted user input.

Use `std::process::Command` with separate arguments.

## 6. Security Requirements

- Never expose password hashes or sensitive shadow database contents in the UI.
- Validate usernames, group names, UID and GID inputs.
- Require root privileges for administrative operations.
- Never silently execute destructive operations.
- Require confirmation before deleting users or groups.
- Clearly communicate the consequences of account deletion.
- Do not automatically delete home directories unless explicitly requested.
- Handle command failures and permission errors gracefully.

## 7. Coding Workflow

Before implementing a feature:

1. Read the relevant project documentation.
2. Inspect the existing source structure.
3. Understand the current architecture.
4. Implement only the requested scope.
5. Run `cargo fmt`.
6. Run `cargo check`.
7. Run relevant tests.
8. Update documentation when behavior or architecture changes.

## 8. Project Integrity

- Do not remove existing features without approval.
- Do not introduce unrelated features.
- Do not modify project requirements silently.
- Do not duplicate existing functionality.
- Do not leave placeholder implementations presented as completed features.

## 9. Development Priority

Follow the roadmap in `docs/[ROADMAP.md](http://ROADMAP.md)`.

Complete the UI foundation before implementing destructive system operations.

## 10. Communication

When implementing features:

- Explain the files being changed.
- Explain important architectural decisions.
- Report completed functionality.
- Report known limitations.
- Identify tests that were executed.

Always prioritize correctness, safety, and maintainability.