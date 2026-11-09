//! Users page: live user table plus selected user's details.

use crate::app::{App, Focus};
use crate::ui::components::{details, table};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the user list and the selected user's details.
pub fn render(frame: &mut Frame, app: &mut App, content: Rect, details_area: Rect) {
    let rows: Vec<table::RowData> = app
        .filtered_users()
        .into_iter()
        .map(|u| table::RowData {
            icon: if u.account_type() == crate::models::AccountType::Regular {
                crate::ui::icons::USER
            } else {
                crate::ui::icons::SHELL
            },
            name: u.name.clone(),
            attr1: u.uid.to_string(),
            attr2: u.account_type().label().to_string(),
        })
        .collect();
    table::render(
        frame,
        content,
        &rows,
        &mut app.selections.users_table,
        app.focus == Focus::Content,
    );

    if details_area.width > 0 {
        render_details(frame, app, details_area);
    }
}

/// Renders details for the currently selected user.
fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let Some(user) = app.selected_user() else {
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
        details::Field::new("Username", user.name.clone()),
        details::Field::new("Full name", full_name),
        details::Field::new("UID", user.uid.to_string()),
        details::Field::new("GID", user.gid.to_string()),
        details::Field::new("Type", user.account_type().label().to_string()),
        details::Field::new("Home", user.home.clone()),
        details::Field::new("Shell", user.shell.clone()),
        details::Field::new("Groups", groups),
    ];
    details::render(frame, area, &fields, app.focus == Focus::Details);
}
