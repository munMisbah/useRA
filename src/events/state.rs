//! Selection state for the list pages.
//!
//! Kept separate from `App` so pages can read selections without the
//! event handler needing page-specific logic inline.

use std::cell::Cell;

thread_local! {
    static USERS_SELECTED: Cell<usize> = const { Cell::new(0) };
    static GROUPS_SELECTED: Cell<usize> = const { Cell::new(0) };
    static SESSIONS_SELECTED: Cell<usize> = const { Cell::new(0) };
}

/// Selected row index on the Users page.
pub fn users_selected() -> usize {
    USERS_SELECTED.get()
}

/// Selected row index on the Groups page.
pub fn groups_selected() -> usize {
    GROUPS_SELECTED.get()
}

/// Selected row index on the Sessions page.
pub fn sessions_selected() -> usize {
    SESSIONS_SELECTED.get()
}

/// Updates the Users selection (clamped by the caller to row bounds).
pub fn set_users_selected(index: usize) {
    USERS_SELECTED.set(index);
}

/// Updates the Groups selection.
pub fn set_groups_selected(index: usize) {
    GROUPS_SELECTED.set(index);
}

/// Updates the Sessions selection.
pub fn set_sessions_selected(index: usize) {
    SESSIONS_SELECTED.set(index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selections_default_to_first_row() {
        assert_eq!(users_selected(), 0);
        assert_eq!(groups_selected(), 0);
        assert_eq!(sessions_selected(), 0);
    }

    #[test]
    fn selections_update_independently() {
        set_users_selected(2);
        set_groups_selected(1);
        assert_eq!(users_selected(), 2);
        assert_eq!(groups_selected(), 1);
        assert_eq!(sessions_selected(), 0);
        set_users_selected(0);
        set_groups_selected(0);
    }
}
