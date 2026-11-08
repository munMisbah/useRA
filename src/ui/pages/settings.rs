//! Settings page: runtime facts, shortcut reference pointer, scope notes.

use crate::app::{App, Focus};
use crate::ui::components::details;
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the settings page content.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let priv_label = if app.data.is_root {
        "root (administrative operations enabled)".to_string()
    } else {
        "unprivileged (read-only view)"
            .set_style(crate::ui::theme::warning())
            .to_string()
    };
    let lines = vec![
        Line::from(" Settings").style(crate::ui::theme::title()),
        Line::from(""),
        Line::from("  Theme        dark (fixed in phase 1)"),
        Line::from(format!("  Privileges   {priv_label}")),
        Line::from("  Data source  live system (getent, uname, loginctl)"),
        Line::from(""),
        Line::from("  Press [r] to reload system data.".set_style(crate::ui::theme::dim())),
        Line::from("  Shortcut reference is available via [?].".set_style(crate::ui::theme::dim())),
    ];
    frame.render_widget(Paragraph::new(lines), content);

    if details_area.width > 0 {
        let fields = vec![details::Field {
            key: "Version",
            value: crate::ui::components::header::APP_VERSION.to_string(),
        }];
        details::render(frame, details_area, &fields, app.focus == Focus::Details);
    }
}
