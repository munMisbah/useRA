//! Top header bar: application name, version, and static system slot.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Application display name.
pub const APP_NAME: &str = "useRA";
/// Crate version, taken from Cargo.toml at compile time.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the one-line header strip.
pub fn render(frame: &mut Frame, _app: &App, area: Rect) {
    let text = format!(
        " {} v{} {}",
        APP_NAME,
        APP_VERSION,
        "Debian System Manager".dim()
    );
    frame.render_widget(Paragraph::new(text).style(crate::ui::theme::title()), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_toml() {
        assert_eq!(APP_VERSION, "0.1.0");
    }
}
