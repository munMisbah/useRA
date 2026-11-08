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
- Full name.
- Home directory.
- Default shell.
- Primary group.
- Supplementary groups.
- Home directory creation preference.

### Edit User

Allow modification of supported account properties.

### Account Security

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

## 5. Search

Support searching:

- Usernames.
- UIDs.
- Group names.
- GIDs.

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