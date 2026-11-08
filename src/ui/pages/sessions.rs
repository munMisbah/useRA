//! Sessions page: mock login session table.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use crate::ui::pages::mock_data;
use ratatui::layout::Rect;

use ratatui::Frame;

/// Renders the mock session list and details.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let rows: Vec<table::RowData> = mock_data::SESSIONS
        .iter()
        .map(|s| table::RowData {
            name: s.user.to_string(),
            attr1: s.id.to_string(),
            attr2: s.seat.to_string(),
        })
        .collect();

    let selected = crate::events::state::sessions_selected();
    table::render(
        frame,
        content,
        &rows,
        if app.focus == Focus::Content {
            selected
        } else {
            usize::MAX
        },
    );

    if details_area.width > 0 {
        let fields = match mock_data::SESSIONS.get(selected) {
            Some(s) => vec![
                details::Field {
                    key: "Session",
                    value: s.id.to_string(),
                },
                details::Field {
                    key: "User",
                    value: s.user.to_string(),
                },
                details::Field {
                    key: "Seat",
                    value: s.seat.to_string(),
                },
            ],
            None => vec![],
        };
        details::render(frame, details_area, &fields, app.focus == Focus::Details);
    }
}
