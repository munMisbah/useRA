//! Icon glyphs from Nerd Fonts (patched fonts).
//!
//! The terminal running useRA must use a Nerd Font; when glyphs render
//! as boxes/tofu the font needs installing (see README "Nerd Font").
//! All icons use the double-width range and fall back visually to
//! ASCII markers are never mixed in.

/// Dashboard icon.
pub const DASHBOARD: &str = "\u{f0e4}"; // nf-oct-graph
/// Users icon.
pub const USERS: &str = "\u{eb5b}"; // nf-md-account-multiple
/// Groups icon.
pub const GROUPS: &str = "\u{e62a}"; // nf-fa-users
/// Sessions icon.
pub const SESSIONS: &str = "\u{f482}"; // nf-oct-device_desktop
/// Settings icon.
pub const SETTINGS: &str = "\u{e615}"; // nf-fa-cog
/// User icon for detail headings.
pub const USER: &str = "\u{eb5a}"; // nf-md-account
/// Key icon for IDs.
pub const KEY: &str = "\u{f80f}"; // nf-fa-key
/// Home icon.
pub const HOME: &str = "\u{f015}"; // nf-fa-home
/// Terminal/shell icon.
pub const SHELL: &str = "\u{e795}"; // nf-cod-terminal
/// Tag icon for type/kind.
pub const TAG: &str = "\u{e7e4}"; // nf-fa-tag
/// Circle icon for status.
pub const STATUS: &str = "\u{f111}"; // nf-fa-circle
/// App logo glyph for the header.
pub const LOGO: &str = "\u{e795}"; // nf-cod-terminal
/// Search icon (future use).
#[allow(dead_code)]
pub const SEARCH: &str = "\u{f52a}"; // nf-oct-search
/// Help icon.
pub const HELP: &str = "\u{f059}"; // nf-fa-question_circle
/// Reload icon (footer/status polish, phase 3+).
#[allow(dead_code)]
pub const RELOAD: &str = "\u{e624}"; // nf-fa-refresh
/// Quit icon (footer/status polish, phase 3+).
#[allow(dead_code)]
pub const QUIT: &str = "\u{ea35}"; // nf-fa-sign_out

/// Icon for a sidebar page, in navigation order.
pub fn for_page(page: crate::app::Page) -> &'static str {
    match page {
        crate::app::Page::Dashboard => DASHBOARD,
        crate::app::Page::Users => USERS,
        crate::app::Page::Groups => GROUPS,
        crate::app::Page::Sessions => SESSIONS,
        crate::app::Page::Settings => SETTINGS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_an_icon() {
        for page in crate::app::PAGES {
            let icon = for_page(page);
            assert!(!icon.is_empty());
            // Single code point keeps column math predictable.
            assert_eq!(icon.chars().count(), 1, "icon for {page:?} must be 1 char");
        }
    }

    #[test]
    fn icons_come_from_private_use_area() {
        // Nerd Font glyphs live in BMP PUA; guards against typos.
        for icon in [DASHBOARD, USERS, GROUPS, SESSIONS, SETTINGS, HELP, RELOAD] {
            let c = icon.chars().next().expect("non-empty");
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&c),
                "icon not in PUA: {}",
                c as u32
            );
        }
    }
}
