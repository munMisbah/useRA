//! Top header bar: application name, version, hostname, privilege state.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Application display name.
pub const APP_NAME: &str = "useRA";
/// Crate version, taken from Cargo.toml at compile time.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the one-line header strip: name, version, host, privilege.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let host = &app.data.host.hostname;
    let priv_label = if app.data.is_root { "root" } else { "user" };
    let text = Line::from(vec![
        Span::styled(
            format!(" {APP_NAME} v{APP_VERSION} "),
            crate::ui::theme::title(),
        ),
        Span::styled(format!("{host} · {priv_label}"), crate::ui::theme::dim()),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_toml() {
        assert_eq!(APP_VERSION, "0.1.0");
    }
}
