//! Dark theme palette and shared styles.
//!
//! Colors follow the approved palette in `docs/UI_DESIGN.md` section 4.

use ratatui::style::{Color, Modifier, Style};

/// Background color of the whole application.
pub const BG: Color = Color::Rgb(0x14, 0x14, 0x14);
/// Panel background color.
pub const PANEL: Color = Color::Rgb(0x33, 0x2C, 0x27);
/// Primary accent color (selections, highlights).
pub const ACCENT: Color = Color::Rgb(0xF5, 0x6A, 0x00);
/// Secondary/dimmed text.
pub const TEXT_DIM: Color = Color::Rgb(0x87, 0x93, 0xA1);
/// Primary text color.
pub const TEXT: Color = Color::Rgb(0xBA, 0xC4, 0xCF);
/// Warning color.
pub const WARNING: Color = Color::Rgb(0xFB, 0xBF, 0x24);
/// Error color.
pub const ERROR: Color = Color::Rgb(0x82, 0x00, 0x00);
/// Success color.
pub const SUCCESS: Color = Color::Rgb(0x34, 0xD3, 0x99);

/// Base style applied to the whole frame.
pub const fn base() -> Style {
    Style::new().bg(BG).fg(TEXT)
}

/// Style for panel borders.
pub const fn border() -> Style {
    Style::new().fg(TEXT_DIM)
}

/// Style for the focused panel border.
pub const fn border_focused() -> Style {
    Style::new().fg(ACCENT)
}

/// Style for panel titles.
pub const fn title() -> Style {
    Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
}

/// Style for the selected/highlighted row.
pub const fn selected() -> Style {
    Style::new().bg(PANEL).fg(TEXT).add_modifier(Modifier::BOLD)
}

/// Style for secondary text (hints, inactive entries).
pub const fn dim() -> Style {
    Style::new().fg(TEXT_DIM)
}

/// Style for accent text (active navigation item, key labels).
pub const fn accent() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Style for success messages (used by operation feedback in phase 3+).
#[allow(dead_code)]
pub const fn success() -> Style {
    Style::new().fg(SUCCESS)
}

/// Style for error messages (used by operation feedback in phase 3+).
#[allow(dead_code)]
pub const fn error() -> Style {
    Style::new().fg(ERROR)
}

/// Style for warning messages.
pub const fn warning() -> Style {
    Style::new().fg(WARNING)
}

/// Outer margin between terminal edge and panels (Superfile-style
/// breathing room).
#[allow(dead_code)]
pub const OUTER_MARGIN: u16 = 1;
/// Outer horizontal margin (breathing room on left and right).
pub const MARGIN_X: u16 = 2;
/// Outer vertical margin (breathing room on top and bottom).
pub const MARGIN_Y: u16 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_matches_design_doc() {
        assert_eq!(BG, Color::Rgb(0x14, 0x14, 0x14));
        assert_eq!(PANEL, Color::Rgb(0x33, 0x2C, 0x27));
        assert_eq!(ACCENT, Color::Rgb(0xF5, 0x6A, 0x00));
        assert_eq!(TEXT_DIM, Color::Rgb(0x87, 0x93, 0xA1));
        assert_eq!(TEXT, Color::Rgb(0xBA, 0xC4, 0xCF));
        assert_eq!(WARNING, Color::Rgb(0xFB, 0xBF, 0x24));
        assert_eq!(ERROR, Color::Rgb(0x82, 0x00, 0x00));
        assert_eq!(SUCCESS, Color::Rgb(0x34, 0xD3, 0x99));
    }
}
