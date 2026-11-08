//! Users page: mock user table plus details panel.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use crate::ui::pages::mock_data;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the user list and the selected user's details.
pub fn render(frame: &mut Frame, app: &App, content: Rect, details_area: Rect) {
    if app.focus == Focus::Content {
        table::render(
            frame,
            content,
            &mock_users(),
            crate::events::state::users_selected(),
        );
    } else {
        table::render(frame, content, &mock_users(), usize::MAX);
    }

    if details_area.width > 0 {
        render_details(frame, app, details_area);
    }
}

/// Converts mock users into table rows.
fn mock_users() -> Vec<table::RowData> {
    mock_data::USERS
        .iter()
        .map(|u| table::RowData {
            name: u.name.to_string(),
            attr1: u.uid.to_string(),
            attr2: u.kind.to_string(),
        })
        .collect()
}

/// Renders details for the currently selected user.
fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let selected = crate::events::state::users_selected();
    let Some(user) = mock_data::USERS.get(selected) else {
        frame.render_widget(
            Paragraph::new("No user selected").style(crate::ui::theme::dim()),
            area,
        );
        return;
    };

    let groups = mock_data::groups_of(user.name).join(", ");
    let fields = vec![
        details::Field {
            key: "Username",
            value: user.name.to_string(),
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
            value: user.kind.to_string(),
        },
        details::Field {
            key: "Home",
            value: user.home.to_string(),
        },
        details::Field {
            key: "Shell",
            value: user.shell.to_string(),
        },
        details::Field {
            key: "Groups",
            value: groups,
        },
    ];
    details::render(frame, area, &fields, app.focus == Focus::Details);
}
