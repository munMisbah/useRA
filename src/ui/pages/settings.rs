//! Settings page: runtime facts, shortcut reference pointer, scope notes.

use crate::app::{App, Focus};
use crate::ui::components::details;
use crate::ui::icons;
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the settings page content.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let (priv_icon, priv_label) = if app.data.is_root {
        (
            icons::STATUS,
            "root (administrative operations enabled)".to_string(),
        )
    } else {
        (icons::USER, "unprivileged (read-only view)".to_string())
    };
    let stat = |icon: &str, label: &str, value: Line<'static>| {
        let mut line = Line::from(vec![
            Span::styled(format!("  {icon} "), crate::ui::theme::accent()),
            Span::raw(format!("{label:<13}")),
        ]);
        for span in value.spans {
            line.spans.push(span);
        }
        line
    };
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} Settings", icons::SETTINGS),
            crate::ui::theme::title(),
        )),
        Line::from(""),
        stat(icons::TAG, "Theme", Line::from("dark (fixed in phase 1)")),
        stat(icons::STATUS, "Privileges", Line::from(priv_label)),
        stat(
            icons::SHELL,
            "Data source",
            Line::from("live system (getent, uname, loginctl)"),
        ),
        Line::from(""),
        Line::from("  Press [r] to reload system data.").set_style(crate::ui::theme::dim()),
        Line::from("  Shortcut reference is available via [?].").set_style(crate::ui::theme::dim()),
    ];
    let _ = priv_icon;
    frame.render_widget(Paragraph::new(lines), content);

    if details_area.width > 0 {
        let fields = vec![details::Field::new(
            "Version",
            crate::ui::components::header::APP_VERSION.to_string(),
        )];
        details::render(frame, details_area, &fields, app.focus == Focus::Details);
    }
}
