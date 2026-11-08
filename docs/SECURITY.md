# useRA — Security Requirements

## 1. Security Philosophy

useRA manages sensitive Linux account information and administrative operations.

Security and predictable behavior take priority over convenience.

## 2. Privilege Management

- Read-only information should be available without root where permissions allow.
- Administrative operations require appropriate privileges.
- Detect insufficient privileges and provide clear feedback.
- Do not automatically elevate privileges without explicit user consent.

## 3. Command Execution

Use Rust's `std::process::Command`.

Requirements:

- Never execute untrusted input through a shell.
- Pass arguments separately.
- Validate all user-provided values.
- Check command exit status.
- Capture and handle errors.
- Avoid logging sensitive command arguments.

## 4. Account Safety

- Never expose password hashes.
- Never display contents of `/etc/shadow`.
- Never store plaintext passwords.
- Do not silently remove home directories.
- Do not silently remove user-owned files.
- Warn before operations that may affect running processes or services.

## 5. Input Validation

Validate:

- Username.
- Group name.
- UID.
- GID.
- Home directory.
- Shell path.

Use system-aware validation and avoid assuming all valid account names follow one simplistic pattern.

## 6. Destructive Operations

Require explicit confirmation for:

- User deletion.
- Group deletion.
- Home directory removal.
- Account modifications with significant consequences.

Display the target account or group before confirmation.

## 7. Audit Logging

Record administrative actions and their results.

Protect log files from unauthorized modification where practical.

Never log passwords, password hashes, or authentication secrets.

## 8. Testing

All destructive integration tests must use isolated test accounts.

Never run destructive tests against production accounts or system users.

## 9. Failure Handling

If an operation fails:

- Report the failure.
- Preserve the original system state as far as possible.
- Do not claim success without verifying the command result.
- Refresh account information when appropriate.