//! Modal state machine: forms with text fields, confirmations, and
//! pending operations.

use ratatui::layout::Rect;
use ratatui::Frame;

/// What the modal is currently asking the user for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalKind {
    /// Create-user form.
    CreateUser,
    /// Edit form for the selected user (shell only in phase 3 core).
    EditShell(String),
    /// Lock confirmation.
    ConfirmLock(String),
    /// Unlock confirmation.
    ConfirmUnlock(String),
    /// Delete confirmation; bool = remove home as well.
    ConfirmDelete { user: String, remove_home: bool },
    /// Add-user-to-groups form.
    AddGroups(String),
    /// Set expiry form.
    SetExpiry(String),
    /// Set password form.
    SetPassword(String),
    /// Create-group form.
    CreateGroup,
    /// Rename-group form (value = current name).
    RenameGroup(String),
    /// Delete-group confirmation.
    ConfirmDeleteGroup(String),
    /// Add users to a group (value = group name).
    GroupAddMembers(String),
    /// Remove users from a group (value = group name).
    GroupRemoveMembers(String),
    /// Jump directory modal.
    GoToPath,
    /// Change mode (`chmod`) checkbox grid modal.
    Chmod(ChmodState),
    /// Change owner/group (`chown`) modal.
    Chown { path: String, is_dir: bool },
    /// Set POSIX ACL (`setfacl -m`) modal.
    SetAcl { path: String, is_dir: bool },
    /// Remove POSIX ACL (`setfacl -x`) modal.
    RemoveAcl { path: String, is_dir: bool },
    /// Clear all extended ACLs (`setfacl -b`) confirmation.
    ConfirmClearAcl(String),
    /// Explicit preview + YES confirmation for recursive (`-R`) resource mutations.
    ConfirmRecursive(RecursiveOp),
}

/// Pending recursive operation awaiting explicit YES confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecursiveOp {
    Chmod { path: String, mode: String },
    Chown { path: String, owner: String },
    SetAcl { path: String, spec: String },
    RemoveAcl { path: String, spec: String },
}

/// Interactive `r/w/x` permission checkbox state for `Owner / Group / Other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChmodState {
    pub path: String,
    pub is_dir: bool,
    /// Full 12-bit POSIX mode (`0o7777`); lower 9 bits toggled via checkboxes.
    pub mode: u32,
    /// Focused row: `0` = Owner, `1` = Group, `2` = Other, `3` = Recursive (when `is_dir`).
    pub row: usize,
    /// Focused permission column: `0` = Read, `1` = Write, `2` = Execute.
    pub col: usize,
    /// Whether `-R` is checked (only editable when `is_dir`).
    pub recursive: bool,
}

impl ChmodState {
    pub fn new(path: String, mode: u32, is_dir: bool) -> Self {
        Self {
            path,
            is_dir,
            mode: mode & 0o7777,
            row: 0,
            col: 0,
            recursive: false,
        }
    }

    fn row_count(&self) -> usize {
        if self.is_dir {
            4
        } else {
            3
        }
    }

    fn bit_mask(row: usize, col: usize) -> u32 {
        let shift = match row {
            0 => 6,
            1 => 3,
            2 => 0,
            _ => return 0,
        };
        let bit = match col {
            0 => 0o4,
            1 => 0o2,
            2 => 0o1,
            _ => return 0,
        };
        bit << shift
    }

    /// True if permission bit at `(row, col)` is currently enabled.
    pub fn has_bit(&self, row: usize, col: usize) -> bool {
        let mask = Self::bit_mask(row, col);
        mask != 0 && (self.mode & mask) != 0
    }

    /// Toggles the currently focused checkbox (permission bit or Recursive).
    pub fn toggle(&mut self) {
        if self.row == 3 && self.is_dir {
            self.recursive = !self.recursive;
        } else {
            self.mode ^= Self::bit_mask(self.row, self.col);
        }
    }

    /// Jumps to `col` (`0=r, 1=w, 2=x`) on the active permission row and toggles it.
    pub fn toggle_col(&mut self, col: usize) {
        if self.row < 3 && col < 3 {
            self.col = col;
            self.mode ^= Self::bit_mask(self.row, col);
        }
    }

    /// Moves row focus vertically, wrapping around.
    pub fn move_row(&mut self, delta: isize) {
        let len = self.row_count() as isize;
        self.row = (self.row as isize + delta).rem_euclid(len) as usize;
    }

    /// Moves column focus horizontally (`Read <-> Write <-> Exec`), wrapping around.
    pub fn move_col(&mut self, delta: isize) {
        if self.row < 3 {
            self.col = (self.col as isize + delta).rem_euclid(3) as usize;
        }
    }

    /// Formatted 4-digit octal string (preserving any existing SUID/SGID/sticky bits).
    pub fn octal_string(&self) -> String {
        format!("{:04o}", self.mode & 0o7777)
    }

    /// Formatted 10-character `drwxr-x---` permission preview.
    pub fn rwx_string(&self) -> String {
        let kind = if self.is_dir {
            crate::models::EntryKind::Directory
        } else {
            crate::models::EntryKind::File
        };
        crate::models::resource::format_mode_rwx(self.mode, kind)
    }
}

/// One labeled text input with its current buffer.
#[derive(Debug, Clone)]
pub struct Field {
    /// Prompt label.
    pub label: &'static str,
    /// Current input buffer.
    pub value: String,
    /// Whether this field masks input characters.
    pub masked: bool,
}

impl Field {
    pub fn new(label: &'static str) -> Self {
        Field {
            label,
            value: String::new(),
            masked: false,
        }
    }

    pub fn with_value(label: &'static str, value: impl Into<String>) -> Self {
        Field {
            label,
            value: value.into(),
            masked: false,
        }
    }

    pub fn masked(label: &'static str) -> Self {
        Field {
            label,
            value: String::new(),
            masked: true,
        }
    }
}

/// Active modal: kind, its fields, cursor position, and any inline
/// validation error to display.
#[derive(Debug, Default)]
pub struct Modal {
    /// What is being asked.
    pub kind: Option<ModalKind>,
    /// Editable fields, in order.
    pub fields: Vec<Field>,
    /// Index of the focused field.
    pub cursor_field: usize,
    /// Byte offset of the caret inside the focused field.
    pub cursor_col: usize,
    /// Inline validation error shown above the input line.
    pub error: Option<String>,
}

impl Modal {
    /// Opens a create-user form.
    pub fn create_user() -> Self {
        Modal {
            kind: Some(ModalKind::CreateUser),
            fields: vec![
                Field::new("Username"),
                Field::new("Full name (optional)"),
                Field::new("Home dir (default /home/<name>)"),
                Field::new("Shell [/bin/bash]"),
                Field::new("Supplementary groups (comma-sep, optional)"),
                Field::masked("Password (optional)"),
            ],
            ..Modal::default()
        }
    }

    /// Opens the edit-shell form prefilled with the current shell.
    pub fn edit_shell(user: String, current: &str) -> Self {
        Modal {
            kind: Some(ModalKind::EditShell(user)),
            fields: vec![Field::with_value("New shell", current)],
            ..Modal::default()
        }
    }

    /// Opens a yes/no confirmation with a message line.
    pub fn confirm(kind: ModalKind, message: &str) -> Self {
        let mut modal = Modal {
            kind: Some(kind),
            fields: vec![Field::new("Type YES to confirm")],
            ..Modal::default()
        };
        modal.error = Some(message.to_string());
        modal
    }

    /// Opens the add-groups form.
    pub fn add_groups(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::AddGroups(user)),
            fields: vec![Field::new("Groups (comma-sep)")],
            ..Modal::default()
        }
    }

    /// Opens the expiry form (empty clears the expiry).
    pub fn set_expiry(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::SetExpiry(user)),
            fields: vec![Field::new("Expiry YYYY-MM-DD (empty=clear)")],
            ..Modal::default()
        }
    }

    /// Opens the set-password form for a user.
    pub fn set_password(user: String) -> Self {
        Modal {
            kind: Some(ModalKind::SetPassword(user)),
            fields: vec![
                Field::masked("New password"),
                Field::masked("Confirm password"),
            ],
            ..Modal::default()
        }
    }

    /// Opens a create-group form (name + optional GID).
    pub fn create_group() -> Self {
        Modal {
            kind: Some(ModalKind::CreateGroup),
            fields: vec![Field::new("Group name"), Field::new("GID (optional)")],
            ..Modal::default()
        }
    }

    /// Opens a rename-group form prefilled with the current name.
    pub fn rename_group(current: String) -> Self {
        Modal {
            kind: Some(ModalKind::RenameGroup(current.clone())),
            fields: vec![Field::with_value("New name", current)],
            ..Modal::default()
        }
    }

    /// Opens the add-members form (comma-separated usernames).
    pub fn group_add_members(group: String) -> Self {
        Modal {
            kind: Some(ModalKind::GroupAddMembers(group)),
            fields: vec![Field::new("Users to add (comma-sep)")],
            ..Modal::default()
        }
    }

    /// Opens the remove-members form prefilled with current members.
    pub fn group_remove_members(group: String, members: &[String]) -> Self {
        Modal {
            kind: Some(ModalKind::GroupRemoveMembers(group)),
            fields: vec![Field::with_value(
                "Users to remove (comma-sep)",
                members.join(", "),
            )],
            ..Modal::default()
        }
    }

    /// Opens the directory jump modal prefilled with `current`.
    pub fn goto_path(current: &str) -> Self {
        Modal {
            kind: Some(ModalKind::GoToPath),
            fields: vec![Field::with_value("Directory path", current)],
            cursor_col: current.len(),
            ..Modal::default()
        }
    }

    /// Opens the interactive `chmod` checkbox modal initialized from `current_mode`.
    pub fn chmod(path: String, current_mode: u32, is_dir: bool) -> Self {
        Modal {
            kind: Some(ModalKind::Chmod(ChmodState::new(
                path,
                current_mode,
                is_dir,
            ))),
            ..Modal::default()
        }
    }

    /// Opens the `chown` modal; directories include an optional recursive toggle.
    pub fn chown(path: String, current_owner: &str, is_dir: bool) -> Self {
        let mut fields = vec![Field::with_value("Owner[:Group]", current_owner)];
        if is_dir {
            fields.push(Field::new("Recursive? (yes/no)"));
        }
        Modal {
            kind: Some(ModalKind::Chown { path, is_dir }),
            cursor_col: current_owner.len(),
            fields,
            ..Modal::default()
        }
    }

    /// Opens the `setfacl -m` modal (prefilled with contextual user/group in Hybrid mode).
    pub fn set_acl(path: String, prefill: &str, is_dir: bool) -> Self {
        let mut fields = vec![Field::with_value(
            "ACL entry (u:user:rwx / g:grp:rx)",
            prefill,
        )];
        if is_dir {
            fields.push(Field::new("Recursive? (yes/no)"));
        }
        Modal {
            kind: Some(ModalKind::SetAcl { path, is_dir }),
            cursor_col: prefill.len(),
            fields,
            ..Modal::default()
        }
    }

    /// Opens the `setfacl -x` modal.
    pub fn remove_acl(path: String, prefill: &str, is_dir: bool) -> Self {
        let mut fields = vec![Field::with_value(
            "Remove ACL target (u:user / g:grp)",
            prefill,
        )];
        if is_dir {
            fields.push(Field::new("Recursive? (yes/no)"));
        }
        Modal {
            kind: Some(ModalKind::RemoveAcl { path, is_dir }),
            cursor_col: prefill.len(),
            fields,
            ..Modal::default()
        }
    }

    /// Opens the recursive preview + YES confirmation dialog.
    pub fn confirm_recursive(op: RecursiveOp, preview_msg: &str) -> Self {
        Self::confirm(ModalKind::ConfirmRecursive(op), preview_msg)
    }

    /// Whether a modal is open.
    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    /// Closes the modal.
    pub fn close(&mut self) {
        *self = Modal::default();
    }

    /// Current focused field, if any.
    fn active_field_mut(&mut self) -> Option<&mut Field> {
        self.fields.get_mut(self.cursor_field)
    }

    /// Types a character into the focused field at the caret.
    pub fn push_char(&mut self, c: char) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            field.value.insert(col, c);
        }
        self.cursor_col = col + c.len_utf8();
    }

    /// Backspace at the caret.
    pub fn backspace(&mut self) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            if col > 0 {
                let prev = field.value[..col]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                field.value.remove(col - prev);
                self.cursor_col = col - prev;
            }
        } else {
            self.cursor_col = col;
        }
    }

    /// Deletes the char under the caret.
    pub fn delete(&mut self) {
        let col = self.cursor_col.min(self.field_value_len());
        if let Some(field) = self.fields.get_mut(self.cursor_field) {
            if col < field.value.len() {
                field.value.remove(col);
            }
        }
        self.cursor_col = col;
    }

    /// Moves the caret left/right within the field.
    pub fn move_cursor(&mut self, delta: isize) {
        if let Some(field) = self.active_field_mut() {
            let cols: Vec<usize> = (0..=field.value.len())
                .filter(|i| field.value.is_char_boundary(*i))
                .collect();
            let pos = cols.iter().position(|&c| c == self.cursor_col).unwrap_or(0);
            let next = (pos as isize + delta).clamp(0, cols.len() as isize - 1) as usize;
            self.cursor_col = cols[next];
        }
    }

    /// Moves to the next (or first) field; wraps around.
    pub fn next_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.cursor_field = (self.cursor_field + 1) % self.fields.len();
        self.cursor_col = self.field_value_len();
    }

    /// Moves to the previous field.
    pub fn previous_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.cursor_field = (self.cursor_field + self.fields.len() - 1) % self.fields.len();
        self.cursor_col = self.field_value_len();
    }

    fn field_value_len(&self) -> usize {
        self.fields
            .get(self.cursor_field)
            .map(|f| f.value.len())
            .unwrap_or(0)
    }
}

/// Character input handling for the focused field.
pub fn handle_input(modal: &mut Modal, code: ratatui::crossterm::event::KeyCode) {
    match code {
        ratatui::crossterm::event::KeyCode::Left => modal.move_cursor(-1),
        ratatui::crossterm::event::KeyCode::Right => modal.move_cursor(1),
        ratatui::crossterm::event::KeyCode::Backspace => modal.backspace(),
        ratatui::crossterm::event::KeyCode::Delete => modal.delete(),
        _ => {}
    }
}

/// Renders the active form/confirm modal.
pub fn render(frame: &mut Frame, modal: &Modal, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    // Dim overlay.
    frame.render_widget(
        Block::new().style(ratatui::style::Style::new().bg(crate::ui::theme::BG)),
        area,
    );

    let (title, _warn) = match &modal.kind {
        Some(ModalKind::CreateUser) => ("Create User", false),
        Some(ModalKind::EditShell(user)) => (&format!("Edit Shell — {user}")[..], false),
        Some(ModalKind::ConfirmLock(user)) => (&format!("Lock — {user}")[..], true),
        Some(ModalKind::ConfirmUnlock(user)) => (&format!("Unlock — {user}")[..], true),
        Some(ModalKind::ConfirmDelete { user, remove_home }) => (
            &format!(
                "Delete — {user} (home {})",
                if *remove_home { "removed" } else { "kept" }
            )[..],
            true,
        ),
        Some(ModalKind::AddGroups(user)) => (&format!("Add Groups — {user}")[..], false),
        Some(ModalKind::SetExpiry(user)) => (&format!("Set Expiry — {user}")[..], false),
        Some(ModalKind::SetPassword(user)) => (&format!("Set Password — {user}")[..], false),
        Some(ModalKind::CreateGroup) => ("Create Group", false),
        Some(ModalKind::RenameGroup(group)) => (&format!("Rename Group — {group}")[..], false),
        Some(ModalKind::ConfirmDeleteGroup(group)) => {
            (&format!("Delete Group — {group}")[..], true)
        }
        Some(ModalKind::GroupAddMembers(group)) => (&format!("Add Members — {group}")[..], false),
        Some(ModalKind::GroupRemoveMembers(group)) => {
            (&format!("Remove Members — {group}")[..], false)
        }
        Some(ModalKind::GoToPath) => ("Go To Directory", false),
        Some(ModalKind::Chmod(state)) => {
            render_chmod(frame, modal, state, area);
            return;
        }
        Some(ModalKind::Chown { path, .. }) => (&format!("Owner (chown) — {path}")[..], false),
        Some(ModalKind::SetAcl { path, .. }) => (&format!("Set ACL — {path}")[..], false),
        Some(ModalKind::RemoveAcl { path, .. }) => (&format!("Remove ACL — {path}")[..], false),
        Some(ModalKind::ConfirmClearAcl(path)) => (&format!("Clear ACLs — {path}")[..], true),
        Some(ModalKind::ConfirmRecursive(_)) => ("Confirm Recursive Operation", true),
        None => return,
    };

    // Size: message line (if any) + fields + hints, bordered.
    let msg_rows = if modal.error.is_some() { 2 } else { 0 };
    let height = (modal.fields.len() as u16 + msg_rows + 3).min(area.height.saturating_sub(2));
    let width = 64_u16.min(area.width.saturating_sub(2));

    let mut lines: Vec<Line> = Vec::new();
    if let Some(err) = &modal.error {
        let style = if modal.kind.as_ref().is_some_and(|k| {
            matches!(
                k,
                ModalKind::ConfirmLock(_)
                    | ModalKind::ConfirmUnlock(_)
                    | ModalKind::ConfirmDelete { .. }
                    | ModalKind::ConfirmClearAcl(_)
                    | ModalKind::ConfirmRecursive(_)
            )
        }) {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::error()
        };
        lines.push(Line::from(format!(" {err}").set_style(style)));
        lines.push(Line::from(""));
    }
    for (i, field) in modal.fields.iter().enumerate() {
        let focused = i == modal.cursor_field;
        let caret = if focused { "▸ " } else { "  " };
        let label = Span::styled(format!("{caret}{}: ", field.label), crate::ui::theme::dim());
        let value_style = if focused {
            crate::ui::theme::base()
        } else {
            crate::ui::theme::dim()
        };
        // Render caret inside the value on the focused field.
        if focused {
            let char_count = field.value.chars().count();
            let display_chars: Vec<char> = if field.masked {
                vec!['•'; char_count]
            } else {
                field.value.chars().collect()
            };
            let col = modal.cursor_col.min(field.value.len());
            let chars_before = field.value[..col].chars().count();
            let before: String = display_chars[..chars_before].iter().collect();
            let after: String = display_chars[chars_before..].iter().collect();
            lines.push(Line::from(vec![
                label,
                Span::raw(before),
                Span::styled("▏", crate::ui::theme::accent()),
                Span::styled(after, value_style),
            ]));
        } else {
            let display_str: String = if field.masked {
                "•".repeat(field.value.chars().count())
            } else {
                field.value.clone()
            };
            lines.push(Line::from(vec![
                label,
                Span::styled(display_str, value_style),
            ]));
        }
    }
    lines.push(
        Line::from("  Enter submit · Tab next field · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(
            if title.starts_with("Delete")
                || title.starts_with("Lock")
                || title.starts_with("Unlock")
                || title.starts_with("Clear ACLs")
                || title.starts_with("Confirm Recursive")
            {
                crate::ui::theme::error()
            } else {
                crate::ui::theme::border_focused()
            },
        );

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

/// Renders the interactive 3x3 Read/Write/Execute checkbox matrix for `chmod`.
fn render_chmod(frame: &mut Frame, modal: &Modal, state: &ChmodState, area: Rect) {
    use ratatui::style::Styled;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let title = format!("Mode (chmod) — {}", state.path);
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &modal.error {
        lines.push(Line::from(
            format!(" {err}").set_style(crate::ui::theme::error()),
        ));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(
        "             Read (4)     Write (2)     Exec (1)".set_style(crate::ui::theme::dim()),
    ));

    for (row_idx, row_label) in ["Owner", "Group", "Other"].iter().enumerate() {
        let row_focused = state.row == row_idx;
        let prefix = if row_focused { "▸ " } else { "  " };
        let label_style = if row_focused {
            crate::ui::theme::title()
        } else {
            crate::ui::theme::dim()
        };
        let mut spans = vec![Span::styled(
            format!("{prefix}{row_label:<8}   "),
            label_style,
        )];
        for (col_idx, perm_char) in ['r', 'w', 'x'].iter().enumerate() {
            let checked = state.has_bit(row_idx, col_idx);
            let cell_focused = row_focused && state.col == col_idx;
            let box_str = if checked {
                format!("[x] {perm_char}")
            } else {
                format!("[ ] {perm_char}")
            };
            let style = if cell_focused {
                crate::ui::theme::accent()
            } else if checked {
                crate::ui::theme::success()
            } else {
                crate::ui::theme::dim()
            };
            spans.push(Span::styled(box_str, style));
            if col_idx < 2 {
                spans.push(Span::raw("        "));
            }
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    let special_note = if state.mode & 0o7000 != 0 {
        format!("   (special bits {:o} preserved)", (state.mode >> 9) & 0o7)
    } else {
        String::new()
    };
    lines.push(Line::from(vec![
        Span::styled("  Octal: ", crate::ui::theme::dim()),
        Span::styled(state.octal_string(), crate::ui::theme::title()),
        Span::styled(
            format!("  ({})", state.rwx_string()),
            crate::ui::theme::base(),
        ),
        Span::styled(special_note, crate::ui::theme::warning()),
    ]));

    if state.is_dir {
        let rec_focused = state.row == 3;
        let prefix = if rec_focused { "▸ " } else { "  " };
        let mark = if state.recursive { "[x]" } else { "[ ]" };
        let style = if rec_focused {
            crate::ui::theme::accent()
        } else if state.recursive {
            crate::ui::theme::warning()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{mark} Recursive (-R)"),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(
        Line::from("  ↑↓/←→ move · Space or r/w/x toggle · Enter apply · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let width = 68_u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let box_area = super::modal::centered(area, width, height);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::border_focused());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Modal {
        Modal::create_user()
    }

    #[test]
    fn create_user_form_has_six_fields() {
        let m = form();
        assert!(m.is_open());
        assert_eq!(m.kind, Some(ModalKind::CreateUser));
        assert_eq!(m.fields.len(), 6);
        assert!(m.fields[5].masked);
        assert_eq!(m.cursor_field, 0);
    }

    #[test]
    fn set_password_form_has_two_masked_fields() {
        let m = Modal::set_password("alice".into());
        assert!(m.is_open());
        assert_eq!(m.kind, Some(ModalKind::SetPassword("alice".into())));
        assert_eq!(m.fields.len(), 2);
        assert!(m.fields[0].masked);
        assert!(m.fields[1].masked);
    }

    #[test]
    fn typing_and_backspace_edit_at_caret() {
        let mut m = form();
        for c in "deploy".chars() {
            m.push_char(c);
        }
        assert_eq!(m.fields[0].value, "deploy");
        assert_eq!(m.cursor_col, 6);
        m.backspace();
        assert_eq!(m.fields[0].value, "deplo");
        assert_eq!(m.cursor_col, 5);
        m.delete();
        assert_eq!(m.fields[0].value, "deplo");
    }

    #[test]
    fn cursor_moves_within_field() {
        let mut m = form();
        for c in "abc".chars() {
            m.push_char(c);
        }
        m.move_cursor(-2);
        m.push_char('X');
        assert_eq!(m.fields[0].value, "aXbc");
    }

    #[test]
    fn field_navigation_wraps() {
        let mut m = form();
        m.push_char('x');
        m.next_field();
        assert_eq!(m.cursor_field, 1);
        m.previous_field();
        assert_eq!(m.cursor_field, 0);
        // value length clamps caret to end of previous field.
        assert_eq!(m.cursor_col, 1);
    }

    #[test]
    fn close_resets_state() {
        let mut m = form();
        m.close();
        assert!(!m.is_open());
        assert!(m.fields.is_empty());
    }

    #[test]
    fn confirm_modal_requires_yes() {
        let m = Modal::confirm(
            ModalKind::ConfirmDelete {
                user: "x".into(),
                remove_home: false,
            },
            "Delete user x?",
        );
        assert_eq!(m.fields.len(), 1);
        assert_eq!(m.error.as_deref(), Some("Delete user x?"));
    }

    #[test]
    fn edit_shell_prefills_current() {
        let m = Modal::edit_shell("root".into(), "/bin/bash");
        assert_eq!(m.fields[0].value, "/bin/bash");
        assert_eq!(m.kind, Some(ModalKind::EditShell("root".into())));
    }

    #[test]
    fn delete_user_reports_missing_target() {
        // Unprivileged: userdel fails, exercising the failure path.
        let res = crate::services::user_service::delete_user("no-such-user-xyz", false);
        assert!(!res.ok);
    }

    #[test]
    fn chmod_checkbox_grid_toggles_bits_and_preserves_special_bits() {
        // SGID 0o2750 = drwxr-s---
        let mut state = ChmodState::new("/srv/app".into(), 0o2750, true);
        assert!(state.has_bit(0, 0)); // Owner r
        assert!(state.has_bit(0, 1)); // Owner w
        assert!(state.has_bit(0, 2)); // Owner x
        assert!(state.has_bit(1, 0)); // Group r
        assert!(!state.has_bit(1, 1)); // Group w off
        assert!(state.has_bit(1, 2)); // Group x
        assert!(!state.has_bit(2, 0)); // Other r off

        // Move to Group Write and toggle on -> 0o2770
        state.move_row(1);
        state.move_col(1);
        state.toggle();
        assert_eq!(state.octal_string(), "2770");
        assert_eq!(state.rwx_string(), "drwxrws---");

        // Quick toggle Group Exec off via 'x' col (2) -> 0o2760
        state.toggle_col(2);
        assert_eq!(state.octal_string(), "2760");

        // Move down to row 3 (Recursive) and toggle
        state.move_row(2);
        assert_eq!(state.row, 3);
        state.toggle();
        assert!(state.recursive);

        // Files do not expose Recursive row (3 rows only)
        let mut file_state = ChmodState::new("/srv/app/.env".into(), 0o0600, false);
        file_state.move_row(-1);
        assert_eq!(file_state.row, 2);
    }

    #[test]
    fn chmod_checkbox_modal_renders_without_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let modal = Modal::chmod("/var/www/app".into(), 0o0750, true);
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|frame| render(frame, &modal, frame.area()))
            .unwrap();
    }
}
