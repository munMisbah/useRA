//! Event dispatch glue between the main loop and the keymap.

use crate::app::App;
use ratatui::crossterm::event::KeyEvent;

/// Applies a key event to app state via the keymap.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    crate::events::keymap::handle_key(app, key);
}
