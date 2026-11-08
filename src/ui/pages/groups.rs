//! Groups page: live group table plus selected group's details.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the group list and the selected group's details.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let rows: Vec<table::RowData> = app
        .data
        .groups
        .iter()
        .map(|g| table::RowData {
            name: g.name.clone(),
            attr1: g.gid.to_string(),
            attr2: format!("{} members", g.members.len()),
        })
        .collect();
    table::render(
        frame,
        content,
        &rows,
        if app.focus == Focus::Content {
            app.selections.groups
        } else {
            usize::MAX
        },
    );

    if details_area.width > 0 {
        render_details(frame, app, details_area);
    }
}

/// Renders details for the currently selected group.
fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let Some(group) = app.data.groups.get(app.selections.groups) else {
        frame.render_widget(
            Paragraph::new("No group selected").style(crate::ui::theme::dim()),
            area,
        );
        return;
    };

    let members = if group.members.is_empty() {
        "(none)".to_string()
    } else {
        group.members.join(", ")
    };
    let fields = vec![
        details::Field {
            key: "Group",
            value: group.name.clone(),
        },
        details::Field {
            key: "GID",
            value: group.gid.to_string(),
        },
        details::Field {
            key: "Members",
            value: members,
        },
    ];
    details::render(frame, area, &fields, app.focus == Focus::Details);
}
