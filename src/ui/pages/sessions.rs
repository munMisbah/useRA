//! Sessions page: live login sessions via loginctl.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the session list and selected session details.
pub fn render(frame: &mut Frame, app: &mut App, content: Rect, details_area: Rect) {
    if app.data.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new(" No login sessions reported (loginctl unavailable or no sessions).")
                .style(crate::ui::theme::dim()),
            content,
        );
        return;
    }

    let rows: Vec<table::RowData> = app
        .data
        .sessions
        .iter()
        .map(|s| table::RowData {
            icon: if s.active {
                crate::ui::icons::STATUS
            } else {
                crate::ui::icons::SHELL
            },
            name: s.user.clone(),
            attr1: s.id.clone(),
            attr2: if s.active {
                "active".to_string()
            } else {
                s.seat.clone()
            },
        })
        .collect();
    table::render(
        frame,
        content,
        &rows,
        &mut app.selections.sessions_table,
        app.focus == Focus::Content,
    );

    if details_area.width > 0 {
        render_details(frame, app, details_area);
    }
}

/// Renders details for the currently selected session.
fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let Some(session) = app.data.sessions.get(app.selections.sessions) else {
        return;
    };
    let fields = vec![
        details::Field::new("Session", session.id.clone()),
        details::Field::new("User", session.user.clone()),
        details::Field::new("Seat", session.seat.clone()),
        details::Field::new(
            "State",
            if session.active { "active" } else { "other" }.to_string(),
        ),
    ];
    details::render(frame, area, &fields, app.focus == Focus::Details);
}
