//! Page renderers for the content and details panels.

pub mod dashboard;
pub mod groups;
pub mod resources;
pub mod sessions;
pub mod settings;
pub mod users;
use crate::app::{App, Page};
use ratatui::layout::Rect;
use ratatui::Frame;

/// Dispatches content and details rendering for the active page.
pub fn render(frame: &mut Frame, app: &mut App, content: Rect, details: Rect) {
    match app.page {
        Page::Dashboard => dashboard::render(frame, app, content, details),
        Page::Users => users::render(frame, app, content, details),
        Page::Groups => groups::render(frame, app, content, details),
        Page::Sessions => sessions::render(frame, app, content, details),
        Page::Resources => resources::render(frame, app, content, details),
        Page::Settings => settings::render(frame, app, content, details),
    }
}
