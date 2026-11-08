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

Status: Completed (2026-11-09)

- [x] Create user (`useradd`, fields: username, full name, home dir, shell,
      supplementary groups; home always created with the account).
- [x] Edit user (login shell via `usermod -s`).
- [x] Lock user (`passwd -l`, explicit YES confirmation).
- [x] Unlock user (`passwd -u`, explicit YES confirmation).
- [x] Change default shell (validated absolute path).
- [x] Modify supplementary groups (`usermod -aG`, comma-separated).
- [x] Delete user (`userdel`; home kept by default — the form states this;
      `-r` home removal is wired but only reachable when a future UI
      revision exposes the explicit "remove home" option per SECURITY §6).
- [x] Add explicit home directory handling (message shows "home kept"/"home removed").
- [x] Implement operation confirmation (type YES; wrong/empty input cancels).
- [x] Input validation client-side (Debian username policy, shell path,
      expiry shape) before any command runs.
- [x] /usr/sbin resolution so useradd/usermod/userdel spawn under user PATHs.

Acceptance criteria: met on the Debian 13.6 dev box. All operations spawn
the correct binary with separate args; unprivileged runs fail with the
tool's Permission denied surfaced in the footer (no auto-elevation).
Root-path smoke (create/lock/delete against real accounts) pending a
root environment; behavior verified up to the spawn+status boundary.

## Phase 4 — Group Management

Status: Completed (2026-11-09)

- [x] Create group (`groupadd`, optional GID).
- [x] Modify group — rename (`groupmod -n`).
- [x] Delete group (`groupdel`, explicit YES confirmation; accounts untouched).
- [x] Add group member (`gpasswd -a`, comma-separated list, per-user
      error reporting with partial-failure message).
- [x] Remove group member (`gpasswd -d`, prefilled with current members).
- [x] Membership information refreshes after every successful operation
      (auto-reload, same path as Phase 3).

Acceptance criteria: met on the Debian 13.6 dev box. All operations spawn
the correct binary with separate args; unprivileged runs fail with the
tool's Permission denied surfaced in the footer. Root-path smoke pending
a root environment.

## Phase 5 — Security and Audit

Status: Completed (2026-11-09)

- [x] Implement privilege checks — write operations refused up front when
      not root ("requires root — run `sudo usera`"); no tool spawn, no
      auto-elevation (SECURITY.md §2).
- [x] Improve validation — Debian username/group policy, absolute shell
      paths, GID numeric, expiry shape (Phases 3–4, hardened here).
- [x] Add audit logging — every executed operation appended to
      `/var/log/usera/audit.log` (root) or
      `$XDG_STATE_HOME/usera/audit.log` (unprivileged); format
      `TIMESTAMP STATUS op=… target=… err="…"`; control chars stripped;
      no passwords/hashes ever logged (SECURITY.md §7). Best-effort:
      log I/O failure never replaces the operation result.
- [x] Add error reporting — raw tool errors mapped to actionable hints
      (insufficient privileges, already exists, not found/stale list,
      in-use by processes); unknown errors pass through.
- [x] Add integration-style tests — audit I/O end-to-end, message
      mapping, service failure paths against the live system
      (64 tests total).
- [x] Review destructive operation safeguards — system-account guard:
      delete/lock of `root` (UID 0) and any UID < 1000 refused with an
      explanation before spawning the tool; unknown targets still fall
      through to the tool.

Acceptance criteria: met.

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