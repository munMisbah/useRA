//! Generic selectable table used by list pages.

use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Cell, Row, Table};
use ratatui::Frame;

/// One row of mock data: name plus two attribute columns.
pub struct RowData {
    /// Primary label (username, group name, ...).
    pub name: String,
    /// First attribute (UID, GID, ...).
    pub attr1: String,
    /// Second attribute (type, status, ...).
    pub attr2: String,
}

/// Column headers for the three columns.
pub const HEADERS: [&str; 3] = ["Name", "ID", "Info"];

/// Renders a three-column table with the selected row highlighted.
pub fn render(frame: &mut Frame, area: Rect, rows: &[RowData], selected: usize) {
    let header = Row::new(HEADERS)
        .style(ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::BOLD));

    let body: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells = vec![
                Cell::from(row.name.clone()),
                Cell::from(row.attr1.clone()),
                Cell::from(row.attr2.clone()),
            ];
            if i == selected {
                Row::new(cells).style(crate::ui::theme::selected())
            } else {
                Row::new(cells)
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
    .column_spacing(1);

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
