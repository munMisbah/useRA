//! Top header strip: hostname, OS, privilege state with icons.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Application display name.
pub const APP_NAME: &str = "useRA";
/// Crate version, taken from Cargo.toml at compile time.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the one-line header: logo, version, OS, privilege.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let os = &app.data.host.os;
    let (priv_icon, priv_label) = if app.data.is_root {
        (crate::ui::icons::STATUS, "root")
    } else {
        (crate::ui::icons::USER, "user")
    };
    let text = Line::from(vec![
        Span::styled(
            format!(" {} v{APP_VERSION} ", crate::ui::icons::LOGO),
            crate::ui::theme::title(),
        ),
        Span::styled(os.clone(), crate::ui::theme::dim()),
        Span::raw("  "),
        Span::styled(priv_icon, crate::ui::theme::dim()),
        Span::styled(format!(" {priv_label}"), crate::ui::theme::dim()),
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
