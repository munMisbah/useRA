//! Dashboard page: live host facts and account counts with icons.

use crate::app::{App, Focus};
use crate::ui::components::details;
use crate::ui::icons;
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders dashboard summary from the live system snapshot.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let data = &app.data;

    fn stat(icon: &str, label: &str, value: String) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {icon} "), crate::ui::theme::accent()),
            Span::raw(format!("{label:<13}")),
            Span::raw(value),
        ])
    }

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} System Overview", icons::DASHBOARD),
            crate::ui::theme::title(),
        )),
        Line::from(""),
        stat(icons::HOME, "Hostname", data.host.hostname.clone()),
        Line::from(""),
        stat(icons::TAG, "OS", data.host.os.clone()),
        Line::from(""),
        stat(icons::SHELL, "Kernel", data.host.kernel.clone()),
        Line::from(""),
        stat(
            icons::USERS,
            "Users",
            format!(
                "{} ({} regular)",
                data.users.len(),
                data.regular_user_count()
            ),
        ),
        Line::from(""),
        stat(icons::GROUPS, "Groups", data.groups.len().to_string()),
        Line::from(""),
        stat(icons::SESSIONS, "Sessions", data.sessions.len().to_string()),
        Line::from(""),
        stat(
            icons::STATUS,
            "Privileges",
            if data.is_root {
                "root (administrative)"
            } else {
                "unprivileged (read-only)"
            }
            .to_string(),
        ),
        Line::from(""),
        Line::from("  Reload anytime with [r].").set_style(crate::ui::theme::dim()),
    ];
    frame.render_widget(Paragraph::new(lines), content);

    if details_area.width > 0 {
        let fields = vec![
            details::Field::new("Page", "Dashboard".to_string()),
            details::Field::new("Focus", format!("{:?}", app.focus)),
        ];
        details::render(frame, details_area, &fields, app.focus == Focus::Details);
    }
}
