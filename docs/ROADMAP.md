# [ROADMAP.md](http://ROADMAP.md)

# useRA — Development Roadmap

## Phase 0 — Project Foundation

Status: Planned

- Initialize Rust project.
- Configure Ratatui.
- Configure Crossterm.
- Establish module structure.
- Configure error handling.
- Configure logging.
- Add basic tests.

## Phase 1 — UI Foundation

Status: Planned

- Implement terminal initialization.
- Implement terminal cleanup.
- Create dark theme.
- Create header component.
- Create sidebar component.
- Create content panel.
- Create details panel.
- Create footer component.
- Implement keyboard navigation.
- Implement responsive layout.
- Implement modal system.

Acceptance criteria:

The application can launch and navigate all main pages without performing system modifications.

## Phase 2 — Read-only System Integration

Status: Planned

- Detect Debian version.
- Read hostname and kernel information.
- Read users through system account databases.
- Read groups through system account databases.
- Inspect user details.
- Inspect group details.
- Display login sessions.

Acceptance criteria:

Users and groups can be inspected without modifying system accounts.

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