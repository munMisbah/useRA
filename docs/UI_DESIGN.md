# useRA — User Interface Specification

## 1. Design Direction

The UI is inspired by modern terminal applications, especially Superfile.

Visual direction:

- Dark theme.
- Minimalist.
- Clean layout.
- Consistent spacing.
- Keyboard-first interaction.
- Information-dense but readable.

## 2. Main Layout

Use a Split Panel layout.

```
┌──────────────────────────────────────────────────────────────┐
│ useRA                                  Debian System Manager  │
├──────────────┬───────────────────────┬───────────────────────┤
│              │                       │                       │
│ NAVIGATION   │ CONTENT               │ DETAILS               │
│              │                       │                       │
│ Dashboard    │ User List             │ Selected User         │
│ Users        │                       │                       │
│ Groups       │ root                  │ Username: deploy      │
│ Sessions     │ ubuntu                │ UID: 1001             │
│ Settings     │ > deploy              │ GID: 1001             │
│              │ www-data              │ Shell: /bin/bash      │
│              │ postgres              │ Home: /home/deploy    │
│              │                       │ Groups: sudo, docker   │
│              │                       │                       │
├──────────────┴───────────────────────┴───────────────────────┤
│ [A] Add  [E] Edit  [D] Delete  [/] Search  [?] Help  [Q] Quit│
└──────────────────────────────────────────────────────────────┘
```

## 3. Layout Components

### Header

Displays:

- Application name.
- Application version.
- Hostname.
- Operating system.

### Sidebar

Navigation items:

- Dashboard.
- Users.
- Groups.
- Sessions.
- Settings.

### Content Panel

Displays the active page content.

For the Users page:

- Username.
- UID.
- Primary GID.
- Account type.
- Account status.

### Details Panel

Displays information about the selected item.

For a selected user:

- Username.
- UID.
- Primary GID.
- Home directory.
- Default shell.
- Supplementary groups.
- Account status.

### Footer

Displays context-sensitive keyboard shortcuts.

## 4. Theme

Use a consistent dark palette.

Suggested colors:


| Element        | Color   |
| -------------- | ------- |
| Background     | #05090F |
| Panel          | #5F2A78 |
| Primary accent | #F56A00 |
| Secondary text | #9CA3AF |
| Main text      | #F9FAFB |
| Warning        | #FBBF24 |
| Error          | #820000 |
| Success        | #34D399 |


Use Ratatui styling APIs to implement the theme.

## 5. Keyboard Navigation


| Key          | Action              |
| ------------ | ------------------- |
| Up / Down    | Navigate list       |
| Left / Right | Change panel focus  |
| Enter        | Select or open      |
| Tab          | Next focus          |
| Esc          | Back or close modal |
| A            | Add user            |
| E            | Edit user           |
| D            | Delete user         |
| /            | Search              |
| ?            | Help                |
| Q            | Quit                |


Shortcuts must not interfere with text input fields.

## 6. Modal Design

Use modal dialogs for:

- Create user.
- Edit user.
- Delete user.
- Create group.
- Delete group.
- Lock account.
- Unlock account.

Destructive operations must display explicit confirmation.

## 7. Responsive Behavior

The UI must adapt to terminal dimensions.

When terminal width is limited:

- Reduce sidebar width.
- Hide or collapse the details panel.
- Keep the active content readable.

When terminal height is limited:

- Reduce unnecessary spacing.
- Preserve access to navigation and status information.

Never allow panels to overlap.

## 8. UX Principles

- Keep interactions predictable.
- Highlight the currently selected item.
- Display operation results clearly.
- Provide loading or progress feedback for long operations.
- Preserve navigation state when switching pages where practical.