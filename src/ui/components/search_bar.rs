//! Search input placeholder bar matching Superfile design.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the search bar: placeholder `🔍 (/) Type something` or live input.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let line = if app.search_active {
        Line::from(vec![
            Span::styled(
                format!(" {} ", crate::ui::icons::SEARCH),
                crate::ui::theme::accent(),
            ),
            Span::styled(app.search_query.clone(), crate::ui::theme::title()),
            Span::styled("█", crate::ui::theme::accent()),
            Span::styled("  (Enter to apply, Esc to clear)", crate::ui::theme::dim()),
        ])
    } else if !app.search_query.is_empty() {
        Line::from(vec![
            Span::styled(
                format!(" {} ", crate::ui::icons::SEARCH),
                crate::ui::theme::success(),
            ),
            Span::styled(app.search_query.clone(), crate::ui::theme::title()),
            Span::styled("  (/ to edit, Esc to clear)", crate::ui::theme::dim()),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} ", crate::ui::icons::SEARCH),
                crate::ui::theme::success(),
            ),
            Span::styled("(/) ", crate::ui::theme::dim()),
            Span::styled("Type something", crate::ui::theme::dim()),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_bar_renders_without_panic() {
        let app = App::default();
        let text = format!(" {} (/) Type something", crate::ui::icons::SEARCH);
        assert!(text.contains("Type something"));
        assert!(text.contains("(/)"));
        assert!(!app.search_active);
    }
}
