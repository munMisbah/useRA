//! Navigation sidebar: page list with icons, active highlight, and a
//! host/privilege footer row (Superfile-style).

use crate::app::{App, Focus, PAGES};
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders sidebar entries with Nerd Font icons. The active page keeps
/// its accent style; inactive entries dim when unfocused.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let sidebar_focused = app.focus == Focus::Sidebar;
    let mut lines: Vec<Line> = Vec::new();
    // Blank row under the border for breathing room.
    lines.push(Line::from(""));

    for page in PAGES {
        let icon = crate::ui::icons::for_page(page);
        let active = page == app.page;
        let marker = if active { "> " } else { "  " };
        let text = format!("{marker}{icon} {}", page.title());
        let style = if active {
            crate::ui::theme::accent()
        } else if sidebar_focused {
            crate::ui::theme::base()
        } else {
            crate::ui::theme::dim()
        };
        lines.push(Line::from(text).set_style(style));
    }

    // Host info pinned at the bottom, Superfile "Disks" style.
    // Reserves room for the two host rows plus the trailing spacer.
    let min_bottom = 3;
    let used = lines.len() as u16;
    if area.height > used + min_bottom + 1 {
        while (lines.len() as u16) < area.height - min_bottom - 1 {
            lines.push(Line::from(""));
        }
        lines.push(section_label("Host"));
        lines.push(host_line(app));
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Dim section label like Superfile's "Pinned"/"Disks".
fn section_label(title: &str) -> Line<'static> {
    Line::from(format!("─ {title}").set_style(crate::ui::theme::dim()))
}

/// Hostname and privilege as a dim two-line block.
fn host_line(app: &App) -> Line<'static> {
    let priv_icon = if app.data.is_root {
        crate::ui::icons::STATUS
    } else {
        crate::ui::icons::USER
    };
    let label = if app.data.is_root { "root" } else { "user" };
    Line::from(vec![
        Span::styled(format!("  {priv_icon} "), crate::ui::theme::dim()),
        Span::raw(app.data.host.hostname.clone()),
        Span::styled(format!(" ({label})"), crate::ui::theme::dim()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_list_matches_layout_order() {
        assert_eq!(PAGES.len(), 5);
        assert_eq!(PAGES[0], crate::app::Page::Dashboard);
        assert_eq!(PAGES[4], crate::app::Page::Settings);
    }

    #[test]
    fn section_label_has_divider_prefix() {
        assert!(section_label("Host").to_string().starts_with('─'));
    }
}
