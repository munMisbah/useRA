//! Modal dialogs. Phase 1 ships the help modal as the working proof of
//! the modal system; operation dialogs (create/delete) arrive with the
//! phases that implement those operations.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Keybindings listed in the help modal.
const HELP_ROWS: [(&str, &str); 8] = [
    ("Up / Down", "Navigate list or pages"),
    ("Left / Right", "Change panel focus"),
    ("Tab", "Next panel"),
    ("Enter", "Select or open"),
    ("Esc", "Back / close modal"),
    ("/", "Search (phase 2+)"),
    ("?", "Toggle help"),
    ("Q", "Quit"),
];

/// Centers `width`x`height` inside `area`.
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let vertical =
        Layout::vertical([Constraint::Length(height)]).flex(ratatui::layout::Flex::Center);
    let horizontal =
        Layout::horizontal([Constraint::Length(width)]).flex(ratatui::layout::Flex::Center);
    let [row] = vertical.areas(area);
    let [cell] = horizontal.areas(row);
    cell
}

/// Renders the help modal with a dimmed backdrop.
pub fn render_help(frame: &mut Frame, area: Rect) {
    // Dim overlay across the whole frame.
    frame.render_widget(
        Block::new().style(ratatui::style::Style::new().bg(crate::ui::theme::BG)),
        area,
    );

    let width = 50_u16.min(area.width);
    let height = (HELP_ROWS.len() as u16 + 8).min(area.height);
    let box_area = centered(area, width, height);

    let mut lines: Vec<Line> = HELP_ROWS
        .iter()
        .map(|(key, desc)| {
            let mut line = Line::from(format!("  {key:<14}").set_style(crate::ui::theme::accent()));
            line.push_span(desc.to_string());
            line
        })
        .collect();
    lines.insert(0, Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from("  Esc/? to close").set_style(crate::ui::theme::dim()));

    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {} Help ", crate::ui::icons::HELP).set_style(crate::ui::theme::title()))
        .border_style(crate::ui::theme::border_focused());

    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}
