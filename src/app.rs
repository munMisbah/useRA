//! Core application state: active page, panel focus, runtime flags, and
//! the live system data snapshot.

use crate::services::AppData;
use ratatui::widgets::TableState;

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
    pub fn previous(self) -> Self {
        match self {
            Focus::Sidebar => Focus::Details,
            Focus::Content => Focus::Sidebar,
            Focus::Details => Focus::Content,
        }
    }
}

/// Selection indexes per list page, clamped to data length on reload.
#[derive(Debug, Default)]
pub struct Selections {
    /// Selected row on the Users page.
    pub users: usize,
    /// Selected row on the Groups page.
    pub groups: usize,
    /// Selected row on the Sessions page.
    pub sessions: usize,
    /// Table scroll & selection state for Users.
    pub users_table: TableState,
    /// Table scroll & selection state for Groups.
    pub groups_table: TableState,
    /// Table scroll & selection state for Sessions.
    pub sessions_table: TableState,
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
    /// Active form/confirm modal (Phase 3 operations).
    pub modal: crate::ui::components::form_modal::Modal,
    /// Transient operation feedback shown in the footer.
    pub notification: Option<(bool, String)>,
    /// List selections per page.
    pub selections: Selections,
    /// Live system snapshot; empty until the first successful load.
    pub data: AppData,
    /// Last load error message, shown in the footer until next load.
    pub load_error: Option<String>,
}

impl App {
    /// Reloads the system snapshot and clamps selections to data length.
    pub fn reload(&mut self) {
        match AppData::load() {
            Ok(data) => {
                self.selections.users = self
                    .selections
                    .users
                    .min(data.users.len().saturating_sub(1));
                self.selections.groups = self
                    .selections
                    .groups
                    .min(data.groups.len().saturating_sub(1));
                self.selections.sessions = self
                    .selections
                    .sessions
                    .min(data.sessions.len().saturating_sub(1));
                self.selections
                    .users_table
                    .select(if data.users.is_empty() {
                        None
                    } else {
                        Some(self.selections.users)
                    });
                self.selections
                    .groups_table
                    .select(if data.groups.is_empty() {
                        None
                    } else {
                        Some(self.selections.groups)
                    });
                self.selections
                    .sessions_table
                    .select(if data.sessions.is_empty() {
                        None
                    } else {
                        Some(self.selections.sessions)
                    });
                self.load_error = None;
                self.data = data;
            }
            Err(err) => {
                self.load_error = Some(err.to_string());
            }
        }
    }

    /// Moves to the next page (sidebar down) and reloads data.
    pub fn next_page(&mut self) {
        self.page = self.page.next();
        self.reload();
    }

    /// Moves to the previous page (sidebar up) and reloads data.
    pub fn previous_page(&mut self) {
        self.page = self.page.previous();
        self.reload();
    }

    /// Cycles panel focus forward.
    pub fn next_focus(&mut self) {
        self.focus = self.focus.next();
    }

    /// Cycles panel focus backward.
    pub fn previous_focus(&mut self) {
        self.focus = self.focus.previous();
    }

    /// Requests application exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Records an operation result as a footer notification.
    pub fn notify(&mut self, ok: bool, message: impl Into<String>) {
        self.notification = Some((ok, message.into()));
    }

    /// Selected user on the Users page, if any.
    pub fn selected_user(&self) -> Option<&crate::models::User> {
        self.data.users.get(self.selections.users)
    }

    /// Selected group on the Groups page, if any.
    pub fn selected_group(&self) -> Option<&crate::models::Group> {
        self.data.groups.get(self.selections.groups)
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
    }

    #[test]
    fn previous_focus_cycles_backward() {
        let mut app = App::default();
        assert_eq!(app.focus, Focus::Content);
        app.previous_focus();
        assert_eq!(app.focus, Focus::Sidebar);
        app.previous_focus();
        assert_eq!(app.focus, Focus::Details);
        app.previous_focus();
        assert_eq!(app.focus, Focus::Content);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::default();
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn reload_populates_users_and_clamps_selection() {
        let mut app = App::default();
        app.selections.users = 10_000;
        app.reload();
        assert!(app.load_error.is_none());
        assert!(!app.data.users.is_empty());
        assert_eq!(app.selections.users, app.data.users.len() - 1);
        // Data is sorted by UID.
        assert!(app.data.users.windows(2).all(|w| w[0].uid <= w[1].uid));
    }

    #[test]
    fn reload_clears_stale_error_on_success() {
        let mut app = App::default();
        app.load_error = Some("stale".to_string());
        app.reload();
        assert!(app.load_error.is_none());
    }
}
