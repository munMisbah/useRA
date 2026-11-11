//! Core application state: active page, panel focus, runtime flags, and
//! the live system data snapshot.

use crate::models::{AclInfo, ResourceEntry};
use crate::services::AppData;
use ratatui::widgets::TableState;
use std::path::PathBuf;
/// Identifiable main pages, matching the sidebar navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Users,
    Groups,
    Sessions,
    Resources,
    Settings,
}

/// Ordered list of pages as shown in the sidebar.
pub const PAGES: [Page; 6] = [
    Page::Dashboard,
    Page::Users,
    Page::Groups,
    Page::Sessions,
    Page::Resources,
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
            Page::Resources => "Resources",
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

/// Resource Manager browser state (shared by `Page::Resources` and Hybrid right panel).
#[derive(Debug)]
pub struct ResourceState {
    /// Current directory being browsed.
    pub cwd: PathBuf,
    /// Entries inside `cwd`.
    pub entries: Vec<ResourceEntry>,
    /// Selected row index inside the active resource list.
    pub selected: usize,
    /// Table scroll & selection state.
    pub table: TableState,
    /// Parsed POSIX ACLs for the currently selected entry.
    pub selected_acl: AclInfo,
    /// Last directory read error, if any.
    pub load_error: Option<String>,
    /// Whether the contextual right-side Resource panel is enabled on Users/Groups.
    pub hybrid_panel: bool,
}

impl Default for ResourceState {
    fn default() -> Self {
        Self {
            cwd: crate::services::resource_service::default_start_dir(),
            entries: Vec::new(),
            selected: 0,
            table: TableState::default(),
            selected_acl: AclInfo::default(),
            load_error: None,
            hybrid_panel: true,
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
    /// Whether the search bar is currently focused for typing.
    pub search_active: bool,
    /// Live search query string used for filtering lists.
    pub search_query: String,
    /// Resource Manager browser state.
    pub resources: ResourceState,
    /// Updated each frame when Hybrid mode is active on Users/Groups.
    pub hybrid_active: bool,
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
                self.reload_resources();
            }
            Err(err) => {
                self.load_error = Some(err.to_string());
            }
        }
    }

    /// Moves to the next page (sidebar down) and reloads data.
    pub fn next_page(&mut self) {
        self.page = self.page.next();
        self.search_active = false;
        self.search_query.clear();
        self.reload();
    }

    /// Moves to the previous page (sidebar up) and reloads data.
    pub fn previous_page(&mut self) {
        self.page = self.page.previous();
        self.search_active = false;
        self.search_query.clear();
        self.reload();
    }

    /// Whether the current active page supports searching (Users, Groups, Sessions).
    pub fn is_searchable_page(&self) -> bool {
        matches!(
            self.page,
            Page::Users | Page::Groups | Page::Sessions | Page::Resources
        )
    }

    /// Minimum terminal geometry for Hybrid contextual Resource panel on Users/Groups.
    pub fn has_hybrid_resources(&self, width: u16, height: u16) -> bool {
        matches!(self.page, Page::Users | Page::Groups)
            && self.resources.hybrid_panel
            && width >= 105
            && height >= 24
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

    /// Filtered users matching search_query.
    pub fn filtered_users(&self) -> Vec<&crate::models::User> {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            return self.data.users.iter().collect();
        }
        self.data
            .users
            .iter()
            .filter(|u| {
                u.name.to_lowercase().contains(&q)
                    || u.uid.to_string().contains(&q)
                    || u.gecos.to_lowercase().contains(&q)
                    || u.shell.to_lowercase().contains(&q)
                    || u.account_type().label().to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Filtered groups matching search_query.
    pub fn filtered_groups(&self) -> Vec<&crate::models::Group> {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            return self.data.groups.iter().collect();
        }
        self.data
            .groups
            .iter()
            .filter(|g| {
                g.name.to_lowercase().contains(&q)
                    || g.gid.to_string().contains(&q)
                    || g.members.iter().any(|m| m.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Filtered sessions matching search_query.
    pub fn filtered_sessions(&self) -> Vec<&crate::models::Session> {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            return self.data.sessions.iter().collect();
        }
        self.data
            .sessions
            .iter()
            .filter(|s| {
                s.user.to_lowercase().contains(&q)
                    || s.id.to_lowercase().contains(&q)
                    || s.seat.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Selected user on the Users page, if any.
    pub fn selected_user(&self) -> Option<&crate::models::User> {
        self.filtered_users().get(self.selections.users).copied()
    }

    /// Selected group on the Groups page, if any.
    pub fn selected_group(&self) -> Option<&crate::models::Group> {
        self.filtered_groups().get(self.selections.groups).copied()
    }

    /// Selected session on the Sessions page, if any.
    pub fn selected_session(&self) -> Option<&crate::models::Session> {
        self.filtered_sessions()
            .get(self.selections.sessions)
            .copied()
    }

    /// Reloads directory entries in `resources.cwd` and refreshes ACL for selection.
    pub fn reload_resources(&mut self) {
        match crate::services::resource_service::list_dir(
            &self.resources.cwd,
            &self.data.users,
            &self.data.groups,
        ) {
            Ok(entries) => {
                self.resources.entries = entries;
                self.resources.load_error = None;
            }
            Err(err) => {
                self.resources.entries.clear();
                self.resources.load_error = Some(err.to_string());
            }
        }
        let len = self.active_resource_entries().len();
        self.resources.selected = self.resources.selected.min(len.saturating_sub(1));
        self.resources.table.select(if len == 0 {
            None
        } else {
            Some(self.resources.selected)
        });
        self.refresh_selected_acl();
    }

    /// Refreshes POSIX ACL data for the currently selected resource entry.
    pub fn refresh_selected_acl(&mut self) {
        if let Some(entry) = self.selected_resource() {
            let path = entry.path.clone();
            self.resources.selected_acl = crate::services::resource_service::read_acl(&path);
        } else {
            self.resources.selected_acl = AclInfo::default();
        }
    }

    /// Resource entries filtered by `search_query` only when on `Page::Resources`.
    pub fn active_resource_entries(&self) -> Vec<&ResourceEntry> {
        if self.page != Page::Resources {
            return self.resources.entries.iter().collect();
        }
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            return self.resources.entries.iter().collect();
        }
        self.resources
            .entries
            .iter()
            .filter(|e| {
                e.is_parent_link
                    || e.name.to_lowercase().contains(&q)
                    || e.owner_name.to_lowercase().contains(&q)
                    || e.group_name.to_lowercase().contains(&q)
                    || e.mode_octal().contains(&q)
            })
            .collect()
    }

    /// Currently selected resource entry (if any).
    pub fn selected_resource(&self) -> Option<&ResourceEntry> {
        self.active_resource_entries()
            .get(self.resources.selected)
            .copied()
    }

    /// Moves selection in the Resource browser and updates ACL info.
    pub fn move_resource_selection(&mut self, delta: isize) {
        let len = self.active_resource_entries().len();
        self.resources.selected =
            crate::ui::components::table::move_selection(self.resources.selected, len, delta);
        self.resources.table.select(if len == 0 {
            None
        } else {
            Some(self.resources.selected)
        });
        self.refresh_selected_acl();
    }

    /// Enters the selected directory (or goes up on `..`).
    pub fn open_selected_resource(&mut self) {
        let Some(entry) = self.selected_resource().cloned() else {
            return;
        };
        if entry.kind == crate::models::EntryKind::Directory {
            let prev_dir = if entry.is_parent_link {
                Some(self.resources.cwd.clone())
            } else {
                None
            };
            self.resources.cwd = entry.path;
            self.resources.selected = 0;
            if self.page == Page::Resources {
                self.search_active = false;
                self.search_query.clear();
            }
            self.reload_resources();
            if let Some(prev) = prev_dir {
                self.select_entry_by_path(&prev);
            }
        }
    }

    /// Navigates to parent directory of `resources.cwd`.
    pub fn go_parent_resource(&mut self) {
        if let Some(parent) = self.resources.cwd.parent().map(|p| p.to_path_buf()) {
            let prev = self.resources.cwd.clone();
            self.resources.cwd = parent;
            self.resources.selected = 0;
            if self.page == Page::Resources {
                self.search_active = false;
                self.search_query.clear();
            }
            self.reload_resources();
            self.select_entry_by_path(&prev);
        }
    }

    /// Jumps directly to `path` if it is a readable directory.
    pub fn goto_resource_path(&mut self, path: PathBuf) -> Result<(), String> {
        if !path.is_dir() {
            return Err(format!("not a directory: {}", path.display()));
        }
        self.resources.cwd = path;
        self.resources.selected = 0;
        if self.page == Page::Resources {
            self.search_active = false;
            self.search_query.clear();
        }
        self.reload_resources();
        if let Some(err) = &self.resources.load_error {
            return Err(err.clone());
        }
        Ok(())
    }

    fn select_entry_by_path(&mut self, target: &std::path::Path) {
        if let Some(idx) = self
            .active_resource_entries()
            .iter()
            .position(|e| e.path == target)
        {
            self.resources.selected = idx;
            self.resources.table.select(Some(idx));
            self.refresh_selected_acl();
        }
    }

    /// Clamps selections to the filtered row counts.
    pub fn clamp_selections_to_filtered(&mut self) {
        let u_len = self.filtered_users().len();
        self.selections.users = self.selections.users.min(u_len.saturating_sub(1));
        self.selections.users_table.select(if u_len == 0 {
            None
        } else {
            Some(self.selections.users)
        });

        let g_len = self.filtered_groups().len();
        self.selections.groups = self.selections.groups.min(g_len.saturating_sub(1));
        self.selections.groups_table.select(if g_len == 0 {
            None
        } else {
            Some(self.selections.groups)
        });

        let s_len = self.filtered_sessions().len();
        self.selections.sessions = self.selections.sessions.min(s_len.saturating_sub(1));
        self.selections.sessions_table.select(if s_len == 0 {
            None
        } else {
            Some(self.selections.sessions)
        });

        let r_len = self.active_resource_entries().len();
        self.resources.selected = self.resources.selected.min(r_len.saturating_sub(1));
        self.resources.table.select(if r_len == 0 {
            None
        } else {
            Some(self.resources.selected)
        });
        self.refresh_selected_acl();
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
