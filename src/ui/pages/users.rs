//! Users page: live user table plus selected user's details.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the user list and the selected user's details.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    let rows: Vec<table::RowData> = app
        .data
        .users
        .iter()
        .map(|u| table::RowData {
            name: u.name.clone(),
            attr1: u.uid.to_string(),
            attr2: u.account_type().label().to_string(),
        })
        .collect();
    table::render(
        frame,
        content,
        &rows,
        if app.focus == Focus::Content {
            app.selections.users
        } else {
            usize::MAX
        },
    );

    if details_area.width > 0 {
        render_details(frame, app, details_area);
    }
}

/// Renders details for the currently selected user.
fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let Some(user) = app.data.users.get(app.selections.users) else {
        frame.render_widget(
            Paragraph::new("No user selected").style(crate::ui::theme::dim()),
            area,
        );
        return;
    };

    let full_name = if user.gecos.is_empty() {
        "(none)".to_string()
    } else {
        user.gecos.clone()
    };
    let groups = if user.supplementary_groups.is_empty() {
        "(none)".to_string()
    } else {
        user.supplementary_groups.join(", ")
    };
    let fields = vec![
        details::Field {
            key: "Username",
            value: user.name.clone(),
        },
        details::Field {
            key: "Full name",
            value: full_name,
        },
        details::Field {
            key: "UID",
            value: user.uid.to_string(),
        },
        details::Field {
            key: "GID",
            value: user.gid.to_string(),
        },
        details::Field {
            key: "Type",
            value: user.account_type().label().to_string(),
        },
        details::Field {
            key: "Home",
            value: user.home.clone(),
        },
        details::Field {
            key: "Shell",
            value: user.shell.clone(),
        },
        details::Field {
            key: "Groups",
            value: groups,
        },
    ];
    details::render(frame, area, &fields, app.focus == Focus::Details);
}
