//! Generic selectable table with icons, roomy spacing, and a full-row
//! selection highlight.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Row, Table};
use ratatui::Frame;

/// One row of table data: icon + name plus two attribute columns.
pub struct RowData {
    /// Nerd Font glyph for the row kind.
    pub icon: &'static str,
    /// Primary label (username, group name, ...).
    pub name: String,
    /// First attribute (UID, GID, ...).
    pub attr1: String,
    /// Second attribute (type, status, ...).
    pub attr2: String,
}

/// Column headers for the three columns.
pub const HEADERS: [&str; 3] = ["Name", "ID", "Info"];

/// Renders a three-column table; the selected row gets the accent bar
/// style, every cell is left-padded so text never touches borders, and
/// each data row is followed by a spacer row so list items breathe.
pub fn render(frame: &mut Frame, area: Rect, rows: &[RowData], selected: usize) {
    let header = Row::new(HEADERS.map(|h| Cell::from(Line::from(h))))
        .style(Style::new().add_modifier(Modifier::BOLD));

    // Interleave data rows with empty spacer rows for vertical spacing.
    let body: Vec<Row> = rows
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            let name_line = Line::from(vec![
                Span::styled(format!("{} ", row.icon), crate::ui::theme::dim()),
                Span::raw(row.name.clone()),
            ]);
            let cells = vec![
                Cell::from(name_line),
                Cell::from(row.attr1.clone()),
                Cell::from(row.attr2.clone()),
            ];
            let data_row = if i == selected {
                Row::new(cells).style(crate::ui::theme::selected())
            } else {
                Row::new(cells)
            };
            let spacer = Row::new(vec![Cell::from(""), Cell::from(""), Cell::from("")]);
            // No spacer after the last row.
            if i + 1 < rows.len() {
                vec![data_row, spacer]
            } else {
                vec![data_row]
            }
        })
        .collect();

    let table = Table::new(
        body,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .column_spacing(1)
    .row_highlight_style(crate::ui::theme::selected());

    frame.render_widget(table, area);
}

/// Wraps selection index arithmetic: up/down with wrap-around.
pub fn move_selection(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let len_i = len as isize;
    let next = (current as isize + delta).rem_euclid(len_i);
    next as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_moves_down_with_wrap() {
        assert_eq!(move_selection(0, 3, 1), 1);
        assert_eq!(move_selection(2, 3, 1), 0);
    }

    #[test]
    fn selection_moves_up_with_wrap() {
        assert_eq!(move_selection(1, 3, -1), 0);
        assert_eq!(move_selection(0, 3, -1), 2);
    }

    #[test]
    fn selection_safe_on_empty_list() {
        assert_eq!(move_selection(0, 0, 1), 0);
    }

    #[test]
    fn selection_noop_without_movement() {
        assert_eq!(move_selection(1, 3, 0), 1);
    }
}
