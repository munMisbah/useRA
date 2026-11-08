//! Right details panel: icon-labeled key/value display.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// One key/value pair shown in the details panel.
pub struct Field {
    /// Nerd Font glyph for this field kind.
    pub icon: &'static str,
    /// Label, e.g. "Username".
    pub key: &'static str,
    /// Current value, e.g. "deploy".
    pub value: String,
}

impl Field {
    /// Convenience constructor using the default icon set.
    pub fn new(key: &'static str, value: String) -> Self {
        Field {
            icon: icon_for(key),
            key,
            value,
        }
    }
}

/// Maps field labels to icons.
fn icon_for(key: &str) -> &'static str {
    match key {
        "Username" | "User" | "Full name" | "Owner" => crate::ui::icons::USER,
        "UID" | "GID" | "Session" | "ID" => crate::ui::icons::KEY,
        "Home" => crate::ui::icons::HOME,
        "Shell" => crate::ui::icons::SHELL,
        "Type" | "State" | "Status" => crate::ui::icons::TAG,
        "Groups" | "Members" => crate::ui::icons::GROUPS,
        "Group" => crate::ui::icons::GROUPS,
        _ => crate::ui::icons::TAG,
    }
}

/// Renders key/value fields; values render dimmed when `focused == false`
/// so the panel visibly responds to focus changes. Each entry gets a
/// blank spacer row above it so fields breathe.
pub fn render(frame: &mut Frame, area: Rect, fields: &[Field], focused: bool) {
    let value_style = if focused {
        crate::ui::theme::base()
    } else {
        crate::ui::theme::dim()
    };
    let lines: Vec<Line> = fields
        .iter()
        .flat_map(|f| {
            vec![
                Line::from(""),
                Line::from(""),
                Line::from(vec![
                    Span::styled(format!(" {} ", f.icon), crate::ui::theme::accent()),
                    Span::raw(format!("{}: ", f.key)),
                    Span::styled(f.value.clone(), value_style),
                ]),
            ]
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_constructor_assigns_known_icons() {
        let f = Field::new("Home", "/root".to_string());
        assert_eq!(f.icon, crate::ui::icons::HOME);
        let g = Field::new("Unknown", "x".to_string());
        assert_eq!(g.icon, crate::ui::icons::TAG);
    }
}
