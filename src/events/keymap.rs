//! Key → action mapping, including Phase 3 operation modals.
//!
//! Navigation keys only mutate `App` state. Operation keys (A/E/D/L/U/
//! G/X on the Users page) open modals; the actual system command runs
//! exclusively through `services::user_service` after the user submits
//! the modal (and, for destructive ops, types YES).

use crate::app::{App, Focus, Page};
use crate::ui::components::form_modal::{self, ModalKind};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Applies a key press to application state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Help modal swallows input first.
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.show_help = false,
            _ => {}
        }
        return;
    }

    // Operation modal: Enter submits, Esc cancels, Tab moves fields.
    if app.modal.is_open() {
        handle_modal_key(app, key);
        return;
    }

    // Search input mode: typing into the search bar.
    if app.search_active {
        handle_search_key(app, key);
        return;
    }
    match (key.code, key.modifiers) {
        // Global keys. `r` reloads except on the Groups page, where it
        // removes members when content is focused.
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('Q'), KeyModifiers::NONE) => {
            app.quit();
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
        (KeyCode::Char('?'), _) => app.show_help = true,
        (KeyCode::Char('r' | 'R'), KeyModifiers::NONE)
            if !(app.page == Page::Groups && app.focus == Focus::Content) =>
        {
            app.reload();
        }
        (KeyCode::Tab, _) => app.next_focus(),
        (KeyCode::BackTab, _) => app.previous_focus(),
        (KeyCode::Char('/'), KeyModifiers::NONE) if app.is_searchable_page() => {
            app.search_active = true;
        }
        (KeyCode::Esc, _) => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.clamp_selections_to_filtered();
            } else {
                app.notification = None;
            }
        }

        // Focus-dependent keys.
        _ => match app.focus {
            Focus::Sidebar => match key.code {
                KeyCode::Down => app.next_page(),
                KeyCode::Up => app.previous_page(),
                KeyCode::Left => app.previous_focus(),
                KeyCode::Right => app.next_focus(),
                KeyCode::Enter => app.next_focus(),
                _ => {}
            },
            Focus::Content => {
                let user_page = app.page == Page::Users;
                let group_page = app.page == Page::Groups;
                match key.code {
                    KeyCode::Down => move_content_selection(app, 1),
                    KeyCode::Up => move_content_selection(app, -1),
                    KeyCode::Left => app.previous_focus(),
                    KeyCode::Right => app.next_focus(),
                    KeyCode::Enter => {}
                    // Operation keys on the Users page.
                    KeyCode::Char('a' | 'A') if user_page => open_create_user(app),
                    KeyCode::Char('p' | 'P') if user_page => open_set_password(app),
                    KeyCode::Char('e' | 'E') if user_page => open_edit_shell(app),
                    KeyCode::Char('d' | 'D') if user_page => open_delete_confirm(app),
                    KeyCode::Char('l' | 'L') if user_page => open_lock_confirm(app),
                    KeyCode::Char('u' | 'U') if user_page => open_unlock_confirm(app),
                    KeyCode::Char('g' | 'G') if user_page => open_add_groups(app),
                    KeyCode::Char('x' | 'X') if user_page => open_set_expiry(app),
                    // Operation keys on the Groups page.
                    KeyCode::Char('a' | 'A') if group_page => open_create_group(app),
                    KeyCode::Char('e' | 'E') if group_page => open_rename_group(app),
                    KeyCode::Char('d' | 'D') if group_page => open_delete_group_confirm(app),
                    KeyCode::Char('g' | 'G') if group_page => open_group_add_members(app),
                    KeyCode::Char('r' | 'R') if group_page => open_group_remove_members(app),
                    _ => {}
                }
            }
            Focus::Details => match key.code {
                KeyCode::Left => app.previous_focus(),
                KeyCode::Right => app.next_focus(),
                _ => {}
            },
        },
    }
}

/// Routes keys inside an open operation modal.
fn handle_modal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.modal.close(),
        KeyCode::Enter => submit_modal(app),
        KeyCode::Tab | KeyCode::Down => app.modal.next_field(),
        KeyCode::BackTab | KeyCode::Up => app.modal.previous_field(),
        KeyCode::Char(c) => app.modal.push_char(c),
        _ => form_modal::handle_input(&mut app.modal, key.code),
    }
}

/// Routes keys while typing into the search bar.
fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_active = false;
            app.search_query.clear();
            app.clamp_selections_to_filtered();
        }
        KeyCode::Enter => {
            app.search_active = false;
            app.focus = Focus::Content;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.clamp_selections_to_filtered();
        }
        KeyCode::Down => move_content_selection(app, 1),
        KeyCode::Up => move_content_selection(app, -1),
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.clamp_selections_to_filtered();
        }
        _ => {}
    }
}

// --- Modal openers ---------------------------------------------------

fn open_create_user(app: &mut App) {
    app.modal = form_modal::Modal::create_user();
}

fn open_set_password(app: &mut App) {
    if let Some(user) = app.selected_user() {
        let name = user.name.clone();
        app.modal = form_modal::Modal::set_password(name);
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_edit_shell(app: &mut App) {
    if let Some(user) = app.selected_user() {
        let (name, shell) = (user.name.clone(), user.shell.clone());
        app.modal = form_modal::Modal::edit_shell(name, &shell);
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_delete_confirm(app: &mut App) {
    if let Some(user) = app.selected_user() {
        let name = user.name.clone();
        app.modal = form_modal::Modal::confirm(
            ModalKind::ConfirmDelete {
                user: name.clone(),
                remove_home: false,
            },
            &format!(
                "Delete user '{name}'? Home is KEPT unless you re-run with [H]. \
                 Type YES to confirm."
            ),
        );
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_lock_confirm(app: &mut App) {
    if let Some(user) = app.selected_user() {
        let name = user.name.clone();
        app.modal = form_modal::Modal::confirm(
            ModalKind::ConfirmLock(name.clone()),
            &format!("Lock account '{name}' (disables password login)? Type YES."),
        );
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_unlock_confirm(app: &mut App) {
    if let Some(user) = app.selected_user() {
        let name = user.name.clone();
        app.modal = form_modal::Modal::confirm(
            ModalKind::ConfirmUnlock(name.clone()),
            &format!("Unlock account '{name}'? Type YES."),
        );
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_add_groups(app: &mut App) {
    if let Some(user) = app.selected_user() {
        app.modal = form_modal::Modal::add_groups(user.name.clone());
    } else {
        app.notify(false, "no user selected");
    }
}

fn open_set_expiry(app: &mut App) {
    if let Some(user) = app.selected_user() {
        app.modal = form_modal::Modal::set_expiry(user.name.clone());
    } else {
        app.notify(false, "no user selected");
    }
}

// --- Group modal openers (Phase 4) ------------------------------------

fn open_create_group(app: &mut App) {
    app.modal = form_modal::Modal::create_group();
}

fn open_rename_group(app: &mut App) {
    if let Some(group) = app.selected_group() {
        app.modal = form_modal::Modal::rename_group(group.name.clone());
    } else {
        app.notify(false, "no group selected");
    }
}

fn open_delete_group_confirm(app: &mut App) {
    if let Some(group) = app.selected_group() {
        let name = group.name.clone();
        app.modal = form_modal::Modal::confirm(
            ModalKind::ConfirmDeleteGroup(name.clone()),
            &format!(
                "Delete group '{name}'? Members lose this group; user accounts \
                 are untouched. Type YES to confirm."
            ),
        );
    } else {
        app.notify(false, "no group selected");
    }
}

fn open_group_add_members(app: &mut App) {
    if let Some(group) = app.selected_group() {
        app.modal = form_modal::Modal::group_add_members(group.name.clone());
    } else {
        app.notify(false, "no group selected");
    }
}

fn open_group_remove_members(app: &mut App) {
    if let Some(group) = app.selected_group() {
        let members = group.members.clone();
        app.modal = form_modal::Modal::group_remove_members(group.name.clone(), &members);
    } else {
        app.notify(false, "no group selected");
    }
}

// --- Modal submission -------------------------------------------------

/// Executes the pending operation described by the modal.
///
/// Write operations are refused outright when not running as root
/// (SECURITY.md §2): the modal closes with a clear message instead of
/// letting the underlying tool fail.
fn submit_modal(app: &mut App) {
    let kind = match app.modal.kind.clone() {
        Some(kind) => kind,
        None => return,
    };

    // Privilege gate: every variant below mutates the system.
    if !app.data.is_root {
        app.modal.close();
        app.notify(
            false,
            "requires root — run `sudo usera` for administrative operations",
        );
        return;
    }

    match kind {
        ModalKind::CreateUser => submit_create_user(app),
        ModalKind::SetPassword(user) => {
            let pw = app.modal.fields[0].value.clone();
            let confirm = app.modal.fields[1].value.clone();
            if pw != confirm {
                app.modal.error = Some("passwords do not match".to_string());
                return;
            }
            if pw.is_empty() {
                app.modal.error = Some("password cannot be empty".to_string());
                return;
            }
            let res = crate::services::user_service::set_password(&user, &pw);
            audit_and_finish(app, "set_password", &user, res);
        }
        ModalKind::EditShell(user) => {
            let shell = app.modal.fields[0].value.trim().to_string();
            let res = crate::services::user_service::set_shell(&user, &shell);
            audit_and_finish(app, "set_shell", &user, res);
        }
        ModalKind::ConfirmLock(user) => {
            if confirmed(app) {
                if let Err(res) =
                    crate::services::user_service::guard_system_account(&user, &app.data.users)
                {
                    audit_and_finish(app, "lock_user", &user, res);
                } else {
                    let res = crate::services::user_service::lock_user(&user);
                    audit_and_finish(app, "lock_user", &user, res);
                }
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::ConfirmUnlock(user) => {
            if confirmed(app) {
                if let Err(res) =
                    crate::services::user_service::guard_system_account(&user, &app.data.users)
                {
                    audit_and_finish(app, "unlock_user", &user, res);
                } else {
                    let res = crate::services::user_service::unlock_user(&user);
                    audit_and_finish(app, "unlock_user", &user, res);
                }
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::ConfirmDelete { user, remove_home } => {
            if confirmed(app) {
                if let Err(res) =
                    crate::services::user_service::guard_system_account(&user, &app.data.users)
                {
                    audit_and_finish(app, "delete_user", &user, res);
                } else {
                    let res = crate::services::user_service::delete_user(&user, remove_home);
                    audit_and_finish(app, "delete_user", &user, res);
                }
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::AddGroups(user) => {
            let raw = app.modal.fields[0].value.trim().to_string();
            let groups: Vec<&str> = raw
                .split(',')
                .map(|g| g.trim())
                .filter(|g| !g.is_empty())
                .collect();
            if groups.is_empty() {
                app.modal.error = Some("enter at least one group".to_string());
                return;
            }
            let res = crate::services::user_service::add_to_groups(&user, &groups);
            audit_and_finish(app, "add_user_groups", &user, res);
        }
        ModalKind::SetExpiry(user) => {
            let raw = app.modal.fields[0].value.trim().to_string();
            let date = if raw.is_empty() {
                None
            } else {
                Some(raw.as_str())
            };
            let res = crate::services::user_service::set_expiry(&user, date);
            audit_and_finish(app, "set_expiry", &user, res);
        }
        ModalKind::CreateGroup => submit_create_group(app),
        ModalKind::RenameGroup(group) => {
            let new = app.modal.fields[0].value.trim().to_string();
            if new.is_empty() {
                app.modal.error = Some("new name is required".to_string());
                return;
            }
            let res = crate::services::group_service::rename_group(&group, &new);
            audit_and_finish(app, "rename_group", &group, res);
        }
        ModalKind::ConfirmDeleteGroup(group) => {
            if confirmed(app) {
                let res = crate::services::group_service::delete_group(&group);
                audit_and_finish(app, "delete_group", &group, res);
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::GroupAddMembers(group) => match parse_user_list(app) {
            Some(list) => {
                let res = crate::services::group_service::add_members(&group, &list);
                audit_and_finish(app, "group_add_members", &group, res);
            }
            None => {
                app.modal.error = Some("enter at least one username".to_string());
            }
        },
        ModalKind::GroupRemoveMembers(group) => match parse_user_list(app) {
            Some(list) => {
                let res = crate::services::group_service::remove_members(&group, &list);
                audit_and_finish(app, "group_remove_members", &group, res);
            }
            None => {
                app.modal.error = Some("enter at least one username".to_string());
            }
        },
    }
}

/// Parses the comma-separated user list from the first form field.
fn parse_user_list(app: &App) -> Option<Vec<String>> {
    let raw = app.modal.fields[0].value.trim().to_string();
    if raw.is_empty() {
        return None;
    }
    let list: Vec<String> = raw
        .split(',')
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .collect();
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// Create-group submit: name required, GID optional numeric.
fn submit_create_group(app: &mut App) {
    let name = app.modal.fields[0].value.trim().to_string();
    let gid_raw = app.modal.fields[1].value.trim().to_string();
    if name.is_empty() {
        app.modal.error = Some("group name is required".to_string());
        return;
    }
    let gid = if gid_raw.is_empty() {
        None
    } else {
        match gid_raw.parse::<u32>() {
            Ok(id) => Some(id),
            Err(_) => {
                app.modal.error = Some("GID must be a number".to_string());
                return;
            }
        }
    };
    let res = crate::services::group_service::create_group(&name, gid);
    audit_and_finish(app, "create_group", &name, res);
}

/// Checks the confirmation field says YES.
fn confirmed(app: &App) -> bool {
    app.modal
        .fields
        .first()
        .map(|f| f.value.trim() == "YES")
        .unwrap_or(false)
}

fn refuse_confirmation(app: &mut App) {
    app.modal.close();
    app.notify(false, "cancelled — confirmation did not match YES");
}

/// Create-user submit: validates fields then calls the service.
fn submit_create_user(app: &mut App) {
    let name = app.modal.fields[0].value.trim().to_string();
    let gecos = app.modal.fields[1].value.trim().to_string();
    let home = app.modal.fields[2].value.trim().to_string();
    let shell_field = app.modal.fields[3].value.trim().to_string();
    let groups = app.modal.fields[4].value.trim().to_string();

    if name.is_empty() {
        app.modal.error = Some("username is required".to_string());
        return;
    }
    let shell = if shell_field.is_empty() {
        "/bin/bash"
    } else {
        shell_field.as_str()
    };
    let gecos_opt = if gecos.is_empty() {
        None
    } else {
        Some(gecos.as_str())
    };
    let home_opt = if home.is_empty() {
        None
    } else {
        Some(home.as_str())
    };

    let res = crate::services::user_service::create_user(&name, gecos_opt, home_opt, shell, true);
    if res.ok {
        if !groups.is_empty() {
            // Supplementary groups right after creation.
            let list: Vec<&str> = groups
                .split(',')
                .map(|g| g.trim())
                .filter(|g| !g.is_empty())
                .collect();
            if !list.is_empty() {
                let res2 = crate::services::user_service::add_to_groups(&name, &list);
                if !res2.ok {
                    app.modal.close();
                    app.notify(
                        false,
                        format!("user created, but groups failed: {}", res2.message),
                    );
                    app.reload();
                    return;
                }
            }
        }
        let password = app
            .modal
            .fields
            .get(5)
            .map(|f| f.value.clone())
            .unwrap_or_default();
        if !password.is_empty() {
            let res_pw = crate::services::user_service::set_password(&name, &password);
            if !res_pw.ok {
                app.modal.close();
                app.notify(
                    false,
                    format!("user created, but password failed: {}", res_pw.message),
                );
                app.reload();
                return;
            }
        }
    }
    audit_and_finish(app, "create_user", &name, res);
}

/// Records the operation in the audit log, then shows the result.
/// Audit failures append to the notification but never replace it.
fn audit_and_finish(
    app: &mut App,
    operation: &'static str,
    target: &str,
    res: crate::services::user_service::OpResult,
) {
    let entry =
        crate::system::audit::AuditEntry::from_result(operation, target, res.ok, &res.message);
    if let Err(err) = crate::system::audit::record(&entry) {
        app.modal.close();
        app.notify(res.ok, format!("{} (audit log: {err})", res.message));
        if res.ok {
            app.reload();
        }
        return;
    }
    finish(app, res);
}

/// Closes the modal, shows the result, refreshes data on success.
fn finish(app: &mut App, res: crate::services::user_service::OpResult) {
    let ok = res.ok;
    let message = res.message;
    app.modal.close();
    app.notify(ok, message);
    if ok {
        app.reload();
    }
}

/// Moves the content selection on list pages, clamping to row counts.
fn move_content_selection(app: &mut App, delta: isize) {
    match app.page {
        Page::Users => {
            let len = app.filtered_users().len();
            app.selections.users =
                crate::ui::components::table::move_selection(app.selections.users, len, delta);
            app.selections.users_table.select(if len == 0 {
                None
            } else {
                Some(app.selections.users)
            });
        }
        Page::Groups => {
            let len = app.filtered_groups().len();
            app.selections.groups =
                crate::ui::components::table::move_selection(app.selections.groups, len, delta);
            app.selections.groups_table.select(if len == 0 {
                None
            } else {
                Some(app.selections.groups)
            });
        }
        Page::Sessions => {
            let len = app.filtered_sessions().len();
            app.selections.sessions =
                crate::ui::components::table::move_selection(app.selections.sessions, len, delta);
            app.selections.sessions_table.select(if len == 0 {
                None
            } else {
                Some(app.selections.sessions)
            });
        }
        Page::Dashboard | Page::Settings => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key_press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn left_arrow_moves_focus_left() {
        let mut app = App::default();
        assert_eq!(app.focus, Focus::Content);

        // Pressing Left on Content should go to Sidebar (leftward)
        handle_key(&mut app, key_press(KeyCode::Left));
        assert_eq!(app.focus, Focus::Sidebar);

        // Pressing Right on Sidebar should go to Content (rightward)
        handle_key(&mut app, key_press(KeyCode::Right));
        assert_eq!(app.focus, Focus::Content);

        // Pressing Right on Content should go to Details (rightward)
        handle_key(&mut app, key_press(KeyCode::Right));
        assert_eq!(app.focus, Focus::Details);

        // Pressing Left on Details should go to Content (leftward)
        handle_key(&mut app, key_press(KeyCode::Left));
        assert_eq!(app.focus, Focus::Content);
    }

    #[test]
    fn content_selection_updates_table_state() {
        let mut app = App::default();
        app.page = Page::Users;
        app.data.users = (0..30)
            .map(|i| crate::models::User {
                name: format!("user{i}"),
                uid: 1000 + i as u32,
                gid: 1000 + i as u32,
                gecos: String::new(),
                home: format!("/home/user{i}"),
                shell: "/bin/bash".into(),
                supplementary_groups: vec![],
            })
            .collect();
        app.reload();

        assert_eq!(app.selections.users, 0);
        assert_eq!(app.selections.users_table.selected(), Some(0));

        // Move down 5 times
        for i in 1..=5 {
            handle_key(&mut app, key_press(KeyCode::Down));
            assert_eq!(app.selections.users, i);
            assert_eq!(app.selections.users_table.selected(), Some(i));
        }

        // Move up 2 times
        handle_key(&mut app, key_press(KeyCode::Up));
        assert_eq!(app.selections.users, 4);
        assert_eq!(app.selections.users_table.selected(), Some(4));
    }

    #[test]
    fn search_mode_filters_users_and_clears_on_esc() {
        let mut app = App::default();
        app.page = Page::Users;
        app.reload();
        app.data.users = vec![
            crate::models::User {
                name: "alice".into(),
                uid: 1001,
                gid: 1001,
                gecos: "Alice Smith".into(),
                home: "/home/alice".into(),
                shell: "/bin/bash".into(),
                supplementary_groups: vec![],
            },
            crate::models::User {
                name: "bob".into(),
                uid: 1002,
                gid: 1002,
                gecos: "Bob Jones".into(),
                home: "/home/bob".into(),
                shell: "/bin/zsh".into(),
                supplementary_groups: vec![],
            },
            crate::models::User {
                name: "charlie".into(),
                uid: 1003,
                gid: 1003,
                gecos: "Charlie Brown".into(),
                home: "/home/charlie".into(),
                shell: "/bin/bash".into(),
                supplementary_groups: vec![],
            },
        ];
        app.clamp_selections_to_filtered();
        assert_eq!(app.filtered_users().len(), 3);

        // Press / to activate search
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(app.search_active);

        // Type 'b' then 'o' -> 'bo' matches bob
        handle_key(&mut app, key_press(KeyCode::Char('b')));
        assert_eq!(app.search_query, "b");

        // Type 'o' -> 'bo' matches bob
        handle_key(&mut app, key_press(KeyCode::Char('o')));
        assert_eq!(app.search_query, "bo");
        assert_eq!(app.filtered_users().len(), 1);
        assert_eq!(app.selected_user().unwrap().name, "bob");

        // Press Enter -> search mode closes, filter remains
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(!app.search_active);
        assert_eq!(app.filtered_users().len(), 1);

        // Press Esc -> clears search filter and restores all 3 users
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert_eq!(app.search_query, "");
        assert_eq!(app.filtered_users().len(), 3);
    }

    #[test]
    fn search_only_activates_on_searchable_pages() {
        let mut app = App::default();

        // Dashboard: not searchable
        app.page = Page::Dashboard;
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(!app.search_active);

        // Settings: not searchable
        app.page = Page::Settings;
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(!app.search_active);

        // Users: searchable
        app.page = Page::Users;
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(app.search_active);
        app.search_active = false;

        // Groups: searchable
        app.page = Page::Groups;
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(app.search_active);
        app.search_active = false;

        // Sessions: searchable
        app.page = Page::Sessions;
        handle_key(&mut app, key_press(KeyCode::Char('/')));
        assert!(app.search_active);
    }

    #[test]
    fn p_key_opens_set_password_modal() {
        let mut app = App::default();
        app.page = Page::Users;
        app.focus = Focus::Content;
        app.reload();
        assert!(!app.data.users.is_empty());

        let selected_name = app.selected_user().unwrap().name.clone();
        handle_key(&mut app, key_press(KeyCode::Char('p')));

        assert!(app.modal.is_open());
        assert_eq!(
            app.modal.kind,
            Some(form_modal::ModalKind::SetPassword(selected_name))
        );
        assert_eq!(app.modal.fields.len(), 2);
        assert!(app.modal.fields[0].masked);
        assert!(app.modal.fields[1].masked);
    }
}
