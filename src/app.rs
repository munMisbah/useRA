//! Core application state: active page, panel focus, runtime flags.
//!
//! State mutations happen exclusively through event handling; rendering
//! only reads this struct. `Focus::previous` and `App::previous_focus`
//! are part of the Phase 1 keymap contract (Shift+Tab lands with text
//! input in a later phase) and are unit-tested.

/// Identifiable main pages, matching the sidebar navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Users,
    Groups,
    Sessions,
    Settings,
}

/// Ordered list of pages as shown in the sidebar.
pub const PAGES: [Page; 5] = [
    Page::Dashboard,
    Page::Users,
    Page::Groups,
    Page::Sessions,
    Page::Settings,
];

impl Page {
    /// Title shown in the sidebar and content panel header.
    pub const fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::Users => "Users",
            Page::Groups => "Groups",
            Page::Sessions => "Sessions",
            Page::Settings => "Settings",
        }
    }

    fn index(self) -> usize {
        PAGES
            .iter()
            .position(|p| *p == self)
            .expect("Page is in PAGES")
    }

    fn from_index(index: usize) -> Self {
        PAGES[index]
    }

    /// Next page in sidebar order, wrapping around.
    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % PAGES.len())
    }

    /// Previous page in sidebar order, wrapping around.
    pub fn previous(self) -> Self {
        Self::from_index((self.index() + PAGES.len() - 1) % PAGES.len())
    }
}

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    Sidebar,
    #[default]
    Content,
    Details,
}

impl Focus {
    /// Next panel in focus order, wrapping around.
    pub fn next(self) -> Self {
        match self {
            Focus::Sidebar => Focus::Content,
            Focus::Content => Focus::Details,
            Focus::Details => Focus::Sidebar,
        }
    }

    /// Previous panel in focus order, wrapping around.
    #[allow(dead_code)]
    pub fn previous(self) -> Self {
        match self {
            Focus::Sidebar => Focus::Details,
            Focus::Content => Focus::Sidebar,
            Focus::Details => Focus::Content,
        }
    }
}

/// Root application state.
#[derive(Debug, Default)]
pub struct App {
    /// Currently active page.
    pub page: Page,
    /// Panel holding keyboard focus.
    pub focus: Focus,
    /// Set when the user requested to quit.
    pub should_quit: bool,
    /// Set while the help modal is open.
    pub show_help: bool,
}

impl App {
    /// Moves to the next page (sidebar down / Tab on sidebar).
    pub fn next_page(&mut self) {
        self.page = self.page.next();
    }

    /// Moves to the previous page (sidebar up).
    pub fn previous_page(&mut self) {
        self.page = self.page.previous();
    }

    /// Cycles panel focus forward.
    pub fn next_focus(&mut self) {
        self.focus = self.focus.next();
    }

    /// Cycles panel focus backward.
    #[allow(dead_code)]
    pub fn previous_focus(&mut self) {
        self.focus = self.focus.previous();
    }

    /// Requests application exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_navigation_wraps_around() {
        assert_eq!(Page::Dashboard.next(), Page::Users);
        assert_eq!(Page::Settings.next(), Page::Dashboard);
        assert_eq!(Page::Users.previous(), Page::Dashboard);
        assert_eq!(Page::Dashboard.previous(), Page::Settings);
    }

    #[test]
    fn focus_cycles_through_all_panels() {
        let mut focus = Focus::Sidebar;
        focus = focus.next();
        assert_eq!(focus, Focus::Content);
        focus = focus.next();
        assert_eq!(focus, Focus::Details);
        focus = focus.next();
        assert_eq!(focus, Focus::Sidebar);
        focus = focus.previous();
        assert_eq!(focus, Focus::Details);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::default();
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn default_state_opens_dashboard() {
        let app = App::default();
        assert_eq!(app.page, Page::Dashboard);
        assert!(!app.show_help);
    }
}
