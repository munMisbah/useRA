# useRA — Feature Specification

## 1. Dashboard

- Display system information.
- Display total users.
- Display total groups.
- Display active sessions.
- Display recent application activity.

## 2. User Management

### User List

- Display regular and system accounts.
- Search by username.
- Filter by account category.
- Sort by username or UID.
- Select a user to inspect details.

### Create User

Fields:

- Username.
- Full name (optional).
- Home directory (optional, validated absolute path).
- Default shell (validated absolute path).
- Supplementary groups (optional).
- Password (optional, masked, piped safely to `chpasswd`).

### Edit User

Allow modification of supported account properties.

### Account Security

- Set / change password (`chpasswd` via stdin).
- Lock account.
- Unlock account.
- Set account expiration.
- Change default shell.
### Delete User

Provide explicit confirmation.

Home directory removal must be an independent, clearly explained option and must never happen implicitly.

## 3. Group Management

### Group List

- Display group name.
- Display GID.
- Display group members.

### Group Operations

- Create group.
- Modify group.
- Delete group.
- Add member.
- Remove member.

## 4. Sessions

Display currently available login sessions using system information.

## 4a. Resource & POSIX ACL Management

- Browse directories (`Resources` page and Hybrid contextual right panel on wide `Users`/`Groups` terminals).
- Inspect Unix modes, ownership, symlinks, batch `+` ACL badges, and live POSIX.1e effective permissions.
- Interactive `chmod` checkbox matrix (`r/w/x` + `SUID/SGID/Sticky`), `chown`, and guided `setfacl` (`-m`, `-x`, `-k`, `-b`) modals.
- Recursive (`-R`) guard blocking protected OS root paths (`/`, `/etc`, `/usr`, `/var`, etc.) plus item-count preview and `YES` confirmation.
- Toggle dotfiles (`.`), jump to path (`G`), and jump to selected user home (`H`).

## 5. Search

Support live `/` filtering on:

- Users (username, UID, full name, shell, account type).
- Groups (group name, GID, members).
- Sessions (user, session ID, seat).
- Resources (entry name, owner, group, octal mode).
## 6. Notifications

Provide clear feedback for:

- Successful operations.
- Failed operations.
- Invalid input.
- Permission errors.
- Confirmation requirements.

## 7. Audit Log

Record application-initiated administrative operations.

Include:

- Timestamp.
- Operation.
- Target account or group.
- Result.
- Error information when applicable.

Never record plaintext passwords or sensitive authentication data.

## 8. Future Features

Potential future enhancements:

- Sudoers inspection.
- Advanced account filtering.
- Account export.
- Account import.
- Multi-distribution support.
- Configurable themes.
- Advanced audit reporting.