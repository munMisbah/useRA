//! Generic selectable table with icons, roomy spacing, and a full-row
//! selection highlight.

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
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
/// style, and every cell is left-padded so text never touches borders.
/// Rows are compact (no spacers — terminal line-height handles spacing).
pub fn render(
    frame: &mut Frame,
    area: Rect,
    rows: &[RowData],
    state: &mut TableState,
    focused: bool,
) {
    let header = Row::new(HEADERS.map(|h| Cell::from(Line::from(format!(" {h}")))))
        .style(Style::new().add_modifier(Modifier::BOLD));

    let body: Vec<Row> = rows
        .iter()
        .map(|row| {
            let name_line = Line::from(vec![
                Span::styled(format!("{} ", row.icon), crate::ui::theme::dim()),
                Span::raw(row.name.clone()),
            ]);
            let cells = vec![
                Cell::from(name_line),
                Cell::from(format!(" {}", row.attr1)),
                Cell::from(format!(" {}", row.attr2)),
            ];
            Row::new(cells)
        })
        .collect();

    // Ensure a valid selection when rows exist.
    if !rows.is_empty() {
        if state.selected().is_none() {
            state.select(Some(0));
        } else if let Some(sel) = state.selected() {
            if sel >= rows.len() {
                state.select(Some(rows.len().saturating_sub(1)));
            }
        }
    }

    let highlight_style = if focused {
        crate::ui::theme::selected()
    } else {
        Style::new()
            .bg(ratatui::style::Color::Rgb(0x35, 0x1A, 0x40))
            .fg(crate::ui::theme::TEXT)
    };

    let table = Table::new(
        body,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .column_spacing(2)
    .row_highlight_style(highlight_style);

    frame.render_stateful_widget(table, area, state);

    let visible_rows = area.height.saturating_sub(1) as usize;
    if rows.len() > visible_rows {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("┃")
            .style(crate::ui::theme::dim());
        let mut scrollbar_state =
            ScrollbarState::new(rows.len().saturating_sub(visible_rows)).position(state.offset());
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
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

    #[test]
    fn test_table_state_scrolls_with_selection() {
        use ratatui::buffer::Buffer;
        use ratatui::widgets::StatefulWidget;

        let rows: Vec<Row> = (0..50)
            .map(|i| Row::new(vec![Cell::from(format!("row {i}"))]))
            .collect();
        let table = Table::new(rows, [Constraint::Percentage(100)]);

        let mut state = TableState::default();
        state.select(Some(25));
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        StatefulWidget::render(table.clone(), Rect::new(0, 0, 20, 5), &mut buf, &mut state);
        assert!(
            state.offset() > 0,
            "offset should scroll past 0 for item 25 in 5-row view"
        );
        let offset_at_25 = state.offset();

        // Moving up to 24 should keep offset if still in view
        state.select(Some(24));
        let mut buf2 = Buffer::empty(Rect::new(0, 0, 20, 5));
        StatefulWidget::render(table, Rect::new(0, 0, 20, 5), &mut buf2, &mut state);
        assert_eq!(state.offset(), offset_at_25);
    }
}
