//! Dashboard page: static placeholders until Phase 2 system reads.

use crate::app::{App, Focus};
use crate::ui::components::details;
use crate::ui::pages::mock_data;
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders dashboard summary cards as text lines.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let lines = vec![
        Line::from(" System Overview").style(crate::ui::theme::title()),
        Line::from(""),
        Line::from(format!(
            "  Hostname     {}",
            "(detected in phase 2)".set_style(crate::ui::theme::dim())
        )),
        Line::from(format!(
            "  OS           {}",
            "Debian GNU/Linux".set_style(crate::ui::theme::dim())
        )),
        Line::from(""),
        Line::from(format!("  Users        {}", mock_data::USERS.len())),
        Line::from(format!("  Groups       {}", mock_data::GROUPS.len())),
        Line::from(format!("  Sessions     {}", mock_data::SESSIONS.len())),
        Line::from(""),
        Line::from("  Real system data arrives in phase 2.".set_style(crate::ui::theme::dim())),
    ];
    frame.render_widget(Paragraph::new(lines), content);

    if details_area.width > 0 {
        let fields = vec![
            details::Field {
                key: "Page",
                value: "Dashboard".to_string(),
            },
            details::Field {
                key: "Focus",
                value: format!("{:?}", app.focus),
            },
        ];
        details::render(frame, details_area, &fields, app.focus == Focus::Details);
    }
}
