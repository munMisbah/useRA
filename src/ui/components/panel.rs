//! Focused panel border helper shared by layout.

use ratatui::layout::Rect;
use ratatui::Frame;

/// Draws a border and returns the inner content area.
///
/// Thin wrapper over `layout::draw_border` so components never import
/// layout internals directly.
pub fn frame_area(frame: &mut Frame, area: Rect, title: &str, focused: bool) -> Rect {
    crate::ui::layout::draw_border(frame, area, title, focused)
}
