# useRA — Product Specification

## 1. Purpose

useRA is a terminal-based Linux account management application designed to simplify user and group administration.

It provides an interactive interface for common administrative tasks without requiring users to remember multiple Linux commands.

## 2. Target Platform

Initial target:

- Debian 12.
- Debian 13.

Other distributions may be supported in future releases.

## 3. Target Users

- Linux system administrators.
- VPS administrators.
- Linux server operators.
- Advanced Linux desktop users.

## 4. Product Goals

- Simplify Linux user management.
- Simplify Linux group management.
- Provide clear visibility into account information.
- Reduce operational mistakes.
- Provide a modern terminal experience.

## 5. Main Modules

### Dashboard

Display:

- Hostname.
- Operating system.
- Kernel version.
- Total users.
- Total groups.
- Active login sessions.

### User Management

Support:

- List users.
- Search users.
- Inspect user information.
- Create users.
- Modify users.
- Delete users.
- Lock accounts.
- Unlock accounts.
- Change default shell.
- Modify supplementary groups.
- Manage account expiration.

### Group Management

Support:

- List groups.
- Inspect group information.
- Create groups.
- Modify groups.
- Delete groups.
- Add users to groups.
- Remove users from groups.

### Session Management

Display available login sessions and relevant account information.

### Settings

Support:

- Theme configuration.
- Keyboard shortcut reference.
- Application preferences.
- Audit log viewing.

## 6. User Experience

The application must provide:

- Keyboard-driven navigation.
- Consistent layout.
- Clear selection indicators.
- Context-sensitive shortcuts.
- Confirmation dialogs.
- Informative error messages.
- Responsive terminal layout.

## 7. Non-Goals for Initial Release

The first release will not implement:

- LDAP administration.
- Active Directory integration.
- Multi-server management.
- Web-based administration.
- PAM configuration management.
- Container user management.

## 8. Success Criteria

The initial release is considered functional when users can:

1. View system account information.
2. Create regular Linux users.
3. Inspect user details.
4. Modify user properties.
5. Lock and unlock accounts.
6. Delete users safely.
7. Create and manage groups.
8. Manage group membership.
9. Navigate the interface entirely through the keyboard.