//! Navigation sidebar: page list with active highlight.

use crate::app::{App, Focus, PAGES};
use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the sidebar entries. Active page is highlighted; entries are
/// dimmed unless the sidebar itself holds focus.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let sidebar_focused = app.focus == Focus::Sidebar;
    let lines: Vec<Line> = PAGES
        .iter()
        .map(|page| {
            let marker = if *page == app.page { "> " } else { "  " };
            let text = format!("{marker}{}", page.title());
            if *page == app.page {
                Line::from(text).set_style(crate::ui::theme::accent())
            } else if sidebar_focused {
                Line::from(text)
            } else {
                Line::from(text).set_style(crate::ui::theme::dim())
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
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
}
