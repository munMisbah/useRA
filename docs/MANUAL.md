# useRA — User Manual

A keyboard-driven terminal interface for managing Linux users and groups
on Debian. This manual covers everyday use; installation is in
[INSTALL.md](INSTALL.md).

## Concepts

useRA follows a split-panel layout:

```
┌────────────┬──────────────────────────────┬──────────────────┐
│ useRA      │  <active page>               │ Details          │
│ Navigation │  list / overview             │ selected item    │
│ ...        │                              │                  │
├────────────┴──────────────────────────────┴──────────────────┤
│ STATUS  shortcuts                                            │
└──────────────────────────────────────────────────────────────┘
```

- **Navigation (left)** — the five pages. The active page is highlighted.
- **Content (center)** — the list or overview of the active page.
- **Details (right)** — everything known about the item highlighted in
  the content list. Hidden on narrow terminals (< 80 columns).
- **Status bar (bottom)** — panel focus, operation results (OK/ERR),
  context-sensitive shortcuts. Shortcut hints appear only when they work:
  management keys show up only when running as root.

## Keyboard reference

Global:

| Key | Action |
| --- | --- |
| `←` `→` or `Tab` | move panel focus (Navigation → Content → Details) |
| `↑` `↓` | navigate within the focused panel |
| `Enter` | select / continue from Navigation |
| `r` | reload data from the system |
| `/` | search / filter list (Users, Groups, Sessions) |
| `?` | help modal (any key closes it) |
| `Esc` | clear search · close modal · dismiss status message |
| `q` or `Ctrl+C` | quit |

Users page (Content focus):

| Key | Action |
| --- | --- |
| `A` | create user (optional masked password field included) |
| `P` | set / change password for the selected user |
| `E` | change login shell of the selected user |
| `L` | lock the selected account (type YES) |
| `U` | unlock the selected account (type YES) |
| `G` | add selected user to supplementary groups |
| `X` | set/clear account expiry (YYYY-MM-DD; empty clears) |
| `D` | delete the selected user (type YES; home is kept) |

Groups page (Content focus):

| Key | Action |
| --- | --- |
| `A` | create group (name, optional GID) |
| `E` | rename the selected group |
| `G` | add members (comma-separated usernames) |
| `R` | remove members (prefilled with current members) |
| `D` | delete the selected group (type YES) |

Users/Groups Hybrid panel (`F` toggles; auto-active when terminal >= 105x24) & `Resources` page:

| Key | Action |
| --- | --- |
| `F` | toggle contextual Resources split on Users/Groups |
| `Space` | mark/unmark multiple files or folders for batch operations |
| `Enter` / `⌫` | open directory / go to parent directory |
| `G` / `H` / `.` | jump to path / jump to user home (Hybrid) / toggle dotfiles |
| `M` | change mode (`chmod` checkboxes incl. SUID/SGID/Sticky + recursive YES) |
| `O` | change owner/group (`chown`, optional recursive preview + YES) |
| `A` | set/edit POSIX ACL (`setfacl -m` Scope, `Enter`/`Tab` autocomplete + auto-load existing ACL, `r/w/x/X`, `Access` & `Default d:`) |
| `X` | remove POSIX ACLs (`setfacl -x`/`-k` checklist from active entries) |
| `B` | clear all extended ACLs (`setfacl -b`, type YES) |
| `C` / `V` | copy ACL rules / paste ACL onto single or multi-selected items (`YES` confirm) |

Inside a form: `Tab`/`Shift+Tab` or `↓`/`↑` move between fields,
`←`/`→` move the text caret, `Backspace`/`Delete` edit, `Enter` submits,
`Esc` cancels. Confirmation dialogs require typing exactly `YES`.

## Pages

### Dashboard

Hostname, OS, kernel, totals (users with regular/system split, groups,
sessions) and your current privilege state. All read live at startup;
press `r` to refresh.

### Users

Every account from the passwd database sorted by UID, with account type
(system < 1000 / regular ≥ 1000). The Details panel shows the full name,
UID, GID, home, shell and supplementary groups. Management keys operate
on the highlighted row.

### Groups

Every group from the group database sorted by GID with member counts.
Details show the member list. Group operations modify membership with
`gpasswd`; one bad username does not abort the rest — failures are
reported individually.

### Sessions

Login sessions reported by `loginctl` (id, user, state). Gracefully
empty on systems without systemd-logind.

### Resources (Permission & POSIX ACL Manager)

Browse directories, inspect Unix modes, ownership, and parsed POSIX ACLs
(`getfacl -p -P`). On wide terminals (`>= 105x24`), Users and Groups also open
a contextual Resources panel on the right with calculated POSIX/ACL
permissions shown in the Details strip below. Recursive (`-R`) mutations
refuse protected OS root paths (`/`, `/etc`, `/usr`, `/var`, etc.) and
require preview + `YES`.

#### Limitations & Kernel Boundaries

Not every directory or file can have its mode, ownership, or ACL modified, and
calculated `POSIX/ACL` permissions are not a guarantee of actual process access:

- **Unsupported filesystems:** Native Debian filesystems (`ext4`, `xfs`,
  `btrfs`) support POSIX modes and ACLs. Non-POSIX or virtual filesystems
  (`FAT32`/`vfat`, `exFAT`, certain `FUSE`/`sshfs` mounts, `/proc`, `/sys`) do
  not support POSIX ACLs (`Operation not supported`) or per-file `chmod`/`chown`.
- **Symlinks:** Symbolic links do not have independent POSIX ACLs; `useRA`
  inspects them physically (`getfacl -P`) and runs `chown -h` so symlink
  targets are not silently altered.
- **Immutable files (`chattr +i`):** Entries with the immutable attribute
  cannot be altered — even by `root` — until `chattr -i` is run.
- **Mount flags (`ro`, `noexec`, `nosuid`):** Read-only mounts reject all
  permission/ACL changes; `noexec` blocks execution even when `+x` is set.
- **AppArmor, systemd sandboxing & capabilities:** Debian 12/13 enables
  AppArmor per application profile (`/etc/apparmor.d/`), and many system
  services restrict paths via systemd (`ProtectSystem`, `ReadOnlyPaths`). A
  service may still fail with `Permission denied` even when `POSIX/ACL` shows
  `rwx`. Conversely, `root` (`UID 0`) normally bypasses DAC via
  `CAP_DAC_OVERRIDE` unless constrained by MAC, mounts, or immutable flags.
- **`acl` package:** `getfacl`/`setfacl` require the Debian `acl` package
  (`sudo apt install acl`); standard `chmod`/`chown` continue working without it.

### Settings

Theme state, privilege state, data source, shortcut reference pointer.

## Safety model

- **Read-only unless root.** Without root you can inspect everything but
  cannot change anything: management keys are refused with a clear
  message and the underlying commands are never started.
- **Two-step confirmation.** Lock, unlock and delete require typing
  `YES` in the dialog. A wrong or empty input cancels cleanly.
- **Home directory safety.** Deleting a user keeps the home directory by
  default; the dialog states this. Home removal is never implicit.
- **System-account guard.** `root` (UID 0) and every system account
  (UID < 1000) are protected: delete/lock is refused by useRA itself,
  before any command runs.
- **Audit log.** Every executed operation is appended to
  `/var/log/usera/audit.log` (root) or `$XDG_STATE_HOME/usera/audit.log`
  with timestamp, result, target and error text. Passwords and password
  hashes are never read or logged.
- **No shell injection.** User input is passed to tools as separate
  arguments and validated (username/group name policy, absolute shell
  paths, numeric GID, expiry date shape) before any command runs.

## Troubleshooting

| Symptom | Cause & fix |
| --- | --- |
| Icons are boxes/tofu | Terminal font is not a Nerd Font — see INSTALL.md |
| "requires root" on every operation | Run `sudo usera` for management |
| "stale list? press [r] to reload" | Someone changed accounts outside useRA — press `r` |
| Sessions page empty | No systemd-logind sessions; not an error |
| Details panel missing | Terminal narrower than 80 columns |
| `"Operation not supported"` on ACL/chmod | Target resides on a non-POSIX/virtual filesystem (`vfat`, `exFAT`, `/proc`, `/sys`, FUSE) |
| `"Operation not permitted"` even as `sudo` | Target may be on a `ro` mount, marked immutable (`lsattr`/`chattr +i`), or restricted by container/capabilities |
| Operation fails with tool stderr | The raw error is shown in the status bar; common causes are mapped to hints |
