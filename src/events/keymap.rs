//! Key → action mapping for Phase 1 navigation.
//!
//! Actions never touch the system; they only mutate `App` state.
//! Write-operation keys (A/E/D) are deliberately unbound until their
//! phases implement them.

use crate::app::{App, Focus, Page};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Applies a key press to application state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Any key closes the help modal first; modal input stays simple.
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.show_help = false,
            _ => {}
        }
        return;
    }

    match (key.code, key.modifiers) {
        // Global keys.
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('Q'), KeyModifiers::NONE) => {
            app.quit();
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
        (KeyCode::Char('?'), _) => app.show_help = true,
        (KeyCode::Tab, _) => app.next_focus(),
        (KeyCode::Esc, _) => {}

        // Focus-dependent keys.
        _ => match app.focus {
            Focus::Sidebar => match key.code {
                KeyCode::Down => app.next_page(),
                KeyCode::Up => app.previous_page(),
                KeyCode::Left | KeyCode::Right => app.next_focus(),
                KeyCode::Enter => app.next_focus(),
                _ => {}
            },
            Focus::Content => match key.code {
                KeyCode::Down => move_content_selection(app, 1),
                KeyCode::Up => move_content_selection(app, -1),
                KeyCode::Left | KeyCode::Right => app.next_focus(),
                KeyCode::Enter => {}
                _ => {}
            },
            Focus::Details => match key.code {
                KeyCode::Left | KeyCode::Right => app.next_focus(),
                _ => {}
            },
        },
    }
}

/// Moves the content selection on list pages, clamping to row counts.
fn move_content_selection(app: &App, delta: isize) {
    match app.page {
        Page::Users => {
            let len = crate::ui::pages::mock_data::USERS.len();
            let next = crate::ui::components::table::move_selection(
                crate::events::state::users_selected(),
                len,
                delta,
            );
            crate::events::state::set_users_selected(next);
        }
        Page::Groups => {
            let len = crate::ui::pages::mock_data::GROUPS.len();
            let next = crate::ui::components::table::move_selection(
                crate::events::state::groups_selected(),
                len,
                delta,
            );
            crate::events::state::set_groups_selected(next);
        }
        Page::Sessions => {
            let len = crate::ui::pages::mock_data::SESSIONS.len();
            let next = crate::ui::components::table::move_selection(
                crate::events::state::sessions_selected(),
                len,
                delta,
            );
            crate::events::state::set_sessions_selected(next);
        }
        Page::Dashboard | Page::Settings => {}
    }
}
