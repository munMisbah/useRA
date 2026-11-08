# [ROADMAP.md](http://ROADMAP.md)

# useRA — Development Roadmap

## Phase 0 — Project Foundation

Status: Completed (2026-11-09)

- [x] Initialize Rust project.
- [x] Configure Ratatui (0.30, crossterm_0_29 backend).
- [x] Configure Crossterm (0.29).
- [x] Establish module structure (`app`, `errors`, `events`, `ui`; `services`/`system` deferred to Phase 2).
- [x] Configure error handling (`errors::AppError`).
- [x] Logging: no crate added; in-app feedback first, audit logging lands with Phase 5 as planned.
- [x] Add basic tests (15 unit tests: state, theme palette, selection math, error display).

## Phase 1 — UI Foundation

Status: Completed (2026-11-09)

- [x] Implement terminal initialization (`ratatui::run`, panic-safe restore).
- [x] Implement terminal cleanup (owned by `ratatui::run` teardown).
- [x] Create dark theme (`ui/theme.rs`, palette per UI_DESIGN §4).
- [x] Create header component.
- [x] Create sidebar component.
- [x] Create content panel.
- [x] Create details panel.
- [x] Create footer component (context-sensitive hints).
- [x] Implement keyboard navigation (↑↓ pages/selection, ←→/Tab focus, ?, Esc, Q).
- [x] Implement responsive layout (<80 cols: details hidden; <60: sidebar collapsed).
- [x] Implement modal system (help modal `[?]` as working proof).

Acceptance criteria: met — the application launches, navigates all main
pages, and performs no system modifications. List content uses mock data
(`ui/pages/mock_data.rs`) until Phase 2 replaces the data source.

## Phase 2 — Read-only System Integration

Status: Completed (2026-11-09)

- [x] Detect Debian version (PRETTY_NAME from /etc/os-release).
- [x] Read hostname and kernel information (`uname`).
- [x] Read users through system account databases (`getent passwd`; shadow never read).
- [x] Read groups through system account databases (`getent group`).
- [x] Inspect user details (incl. supplementary group merge from group db).
- [x] Inspect group details.
- [x] Display login sessions (`loginctl`, graceful without logind).

Acceptance criteria: met — users and groups are inspected without modifying
system accounts. Reload happens on page change and `[r]`; privilege state
(root/user) is shown in the header and dashboard.

## Phase 3 — User Management

Status: Planned

- Create user.
- Edit user.
- Lock user.
- Unlock user.
- Change default shell.
- Modify supplementary groups.
- Delete user.
- Add explicit home directory handling.
- Implement operation confirmation.

Acceptance criteria:

User operations work safely on Debian and report their results accurately.

## Phase 4 — Group Management

Status: Planned

- Create group.
- Modify group.
- Delete group.
- Add group member.
- Remove group member.

Acceptance criteria:

Group operations work and membership information refreshes correctly.

## Phase 5 — Security and Audit

Status: Planned

- Implement privilege checks.
- Improve validation.
- Add audit logging.
- Add error reporting.
- Add integration tests.
- Review destructive operation safeguards.

## Phase 6 — Release Preparation

Status: Planned

- Optimize startup.
- Verify terminal compatibility.
- Test Debian 12.
- Test Debian 13.
- Prepare release binary.
- Write installation documentation.
- Write user manual.
- Prepare initial release.

---

## Development Policy

Complete and validate each phase before moving to the next.

Do not mark a feature as completed until its implementation and relevant tests have been verified.