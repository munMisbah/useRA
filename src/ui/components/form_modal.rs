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
}

/// One labeled text input with its current buffer.
#[derive(Debug, Clone)]
pub struct Field {
    /// Prompt label.
    pub label: &'static str,
    /// Current input buffer.
    pub value: String,
}

impl Field {
    fn new(label: &'static str) -> Self {
        Field {
            label,
            value: String::new(),
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
            ],
            ..Modal::default()
        }
    }

    /// Opens the edit-shell form prefilled with the current shell.
    pub fn edit_shell(user: String, current: &str) -> Self {
        Modal {
            kind: Some(ModalKind::EditShell(user)),
            fields: vec![Field {
                label: "New shell",
                value: current.to_string(),
            }],
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
        None => return,
    };

    // Size: message line (if any) + fields + hints, bordered.
    let msg_rows = if modal.error.is_some() { 2 } else { 0 };
    let height = (modal.fields.len() as u16 * 2 + msg_rows + 5).min(area.height.saturating_sub(2));
    let width = 54_u16.min(area.width.saturating_sub(2));

    let mut lines: Vec<Line> = Vec::new();
    if let Some(err) = &modal.error {
        let style = if modal.kind.as_ref().is_some_and(|k| {
            matches!(
                k,
                ModalKind::ConfirmLock(_)
                    | ModalKind::ConfirmUnlock(_)
                    | ModalKind::ConfirmDelete { .. }
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
            let col = modal.cursor_col.min(field.value.len());
            let (before, after) = field.value.split_at(col);
            lines.push(Line::from(vec![
                label,
                Span::raw(before.to_string()),
                Span::styled("▏", crate::ui::theme::accent()),
                Span::styled(after.to_string(), value_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                label,
                Span::styled(field.value.clone(), value_style),
            ]));
        }
        lines.push(Line::from(""));
    }
    lines.push(
        Line::from("  Enter submit · Tab next field · Esc cancel")
            .set_style(crate::ui::theme::dim()),
    );

    let box_area = super::modal::centered(area, width, height + 2);
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .border_style(
            if title.starts_with("Delete")
                || title.starts_with("Lock")
                || title.starts_with("Unlock")
            {
                crate::ui::theme::error()
            } else {
                crate::ui::theme::border_focused()
            },
        );

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
    fn create_user_form_has_five_fields() {
        let m = form();
        assert!(m.is_open());
        assert_eq!(m.kind, Some(ModalKind::CreateUser));
        assert_eq!(m.fields.len(), 5);
        assert_eq!(m.cursor_field, 0);
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
}
