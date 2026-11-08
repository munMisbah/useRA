//! Key → action mapping for Phase 2 navigation.
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
        (KeyCode::Char('r'), KeyModifiers::NONE) | (KeyCode::Char('R'), KeyModifiers::NONE) => {
            app.reload();
        }
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
fn move_content_selection(app: &mut App, delta: isize) {
    match app.page {
        Page::Users => {
            let len = app.data.users.len();
            app.selections.users =
                crate::ui::components::table::move_selection(app.selections.users, len, delta);
        }
        Page::Groups => {
            let len = app.data.groups.len();
            app.selections.groups =
                crate::ui::components::table::move_selection(app.selections.groups, len, delta);
        }
        Page::Sessions => {
            let len = app.data.sessions.len();
            app.selections.sessions =
                crate::ui::components::table::move_selection(app.selections.sessions, len, delta);
        }
        Page::Dashboard | Page::Settings => {}
    }
}
