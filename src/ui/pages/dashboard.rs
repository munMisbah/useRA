//! Dashboard page: live host facts and account counts.

use crate::app::{App, Focus};
use crate::ui::components::details;
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders dashboard summary from the live system snapshot.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let data = &app.data;
    let lines = vec![
        Line::from(" System Overview").style(crate::ui::theme::title()),
        Line::from(""),
        Line::from(format!("  Hostname     {}", data.host.hostname)),
        Line::from(format!("  OS           {}", data.host.os)),
        Line::from(format!(
            "  Kernel       {}",
            data.host.kernel.clone().set_style(crate::ui::theme::dim())
        )),
        Line::from(""),
        Line::from(format!(
            "  Users        {} ({} regular)",
            data.users.len(),
            data.regular_user_count()
        )),
        Line::from(format!("  Groups       {}", data.groups.len())),
        Line::from(format!("  Sessions     {}", data.sessions.len())),
        Line::from(""),
        Line::from(
            "  Privileges   ".to_string()
                + if data.is_root {
                    "root (administrative)"
                } else {
                    "unprivileged (read-only)"
                },
        ),
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
