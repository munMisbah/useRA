//! Right details panel: key/value display for the selected item.

use ratatui::layout::Rect;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// One key/value pair shown in the details panel.
pub struct Field {
    /// Label, e.g. "Username".
    pub key: &'static str,
    /// Current value, e.g. "deploy".
    pub value: String,
}

/// Renders key/value fields; values render dimmed when `focused == false`
/// so the panel visibly responds to focus changes.
pub fn render(frame: &mut Frame, area: Rect, fields: &[Field], focused: bool) {
    let value_style = if focused {
        crate::ui::theme::base()
    } else {
        crate::ui::theme::dim()
    };
    let lines: Vec<Line> = fields
        .iter()
        .map(|f| {
            let mut line = Line::from(format!("{}: ", f.key));
            line.push_span(f.value.clone().set_style(value_style));
            line
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}
