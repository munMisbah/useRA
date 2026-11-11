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
            } else if !app.resources.selected_paths.is_empty() {
                app.clear_selected_resources();
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
                if app.page == Page::Resources {
                    match key.code {
                        KeyCode::Left => app.previous_focus(),
                        KeyCode::Right => app.next_focus(),
                        other => handle_resource_key(app, other),
                    }
                    return;
                }
                let user_page = app.page == Page::Users;
                let group_page = app.page == Page::Groups;
                match key.code {
                    KeyCode::Down => move_content_selection(app, 1),
                    KeyCode::Up => move_content_selection(app, -1),
                    KeyCode::Left => app.previous_focus(),
                    KeyCode::Right => app.next_focus(),
                    KeyCode::Enter => {}
                    KeyCode::Char('f' | 'F') if user_page || group_page => {
                        toggle_hybrid_resources(app);
                    }
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
            Focus::Details => {
                if app.hybrid_active {
                    match key.code {
                        KeyCode::Left => app.previous_focus(),
                        KeyCode::Right => app.next_focus(),
                        KeyCode::Char('f' | 'F') => toggle_hybrid_resources(app),
                        other => handle_resource_key(app, other),
                    }
                } else {
                    match key.code {
                        KeyCode::Left => app.previous_focus(),
                        KeyCode::Right => app.next_focus(),
                        KeyCode::Char('f' | 'F')
                            if matches!(app.page, Page::Users | Page::Groups) =>
                        {
                            toggle_hybrid_resources(app);
                        }
                        _ => {}
                    }
                }
            }
        },
    }
}

/// Routes keys inside an open operation modal.
fn handle_modal_key(app: &mut App, key: KeyEvent) {
    if let Some(ModalKind::Chmod(state)) = &mut app.modal.kind {
        match key.code {
            KeyCode::Esc => app.modal.close(),
            KeyCode::Enter => submit_modal(app),
            KeyCode::Up | KeyCode::BackTab => state.move_row(-1),
            KeyCode::Down | KeyCode::Tab => state.move_row(1),
            KeyCode::Left => state.move_col(-1),
            KeyCode::Right => state.move_col(1),
            KeyCode::Char(' ') => state.toggle(),
            KeyCode::Char('r' | 'R') => {
                if state.row == 4 {
                    state.toggle();
                } else if state.row < 3 {
                    state.toggle_col(0);
                }
            }
            KeyCode::Char('w' | 'W') if state.row < 3 => state.toggle_col(1),
            KeyCode::Char('x' | 'X') if state.row < 3 => state.toggle_col(2),
            KeyCode::Char('s' | 'S' | 'u' | 'U') if state.row == 3 => state.toggle_col(0),
            KeyCode::Char('g' | 'G') if state.row == 3 => state.toggle_col(1),
            KeyCode::Char('t' | 'T') if state.row == 3 => state.toggle_col(2),
            _ => {}
        }
        return;
    }

    if let Some(ModalKind::SetAcl(state)) = &mut app.modal.kind {
        match key.code {
            KeyCode::Esc => app.modal.close(),
            KeyCode::Enter => {
                if state.row == 1 && !state.filtered_suggestions().is_empty() {
                    let _ = state.accept_suggestion();
                    state.move_row(1);
                } else {
                    submit_modal(app);
                }
            }
            KeyCode::Tab => {
                if state.row == 1 {
                    let _ = state.accept_suggestion();
                }
                state.move_row(1);
            }
            KeyCode::BackTab => state.move_row(-1),
            KeyCode::Up => {
                if !state.filtered_suggestions().is_empty() {
                    state.move_suggestion(-1);
                } else {
                    state.move_row(-1);
                }
            }
            KeyCode::Down => {
                if !state.filtered_suggestions().is_empty() {
                    state.move_suggestion(1);
                } else {
                    state.move_row(1);
                }
            }
            KeyCode::Left => state.move_col(-1),
            KeyCode::Right => state.move_col(1),
            KeyCode::Backspace => state.backspace(),
            KeyCode::Delete => state.delete(),
            KeyCode::Char(' ') => {
                if state.row != 1 {
                    state.toggle();
                }
            }
            KeyCode::Char(c) if state.row == 1 => state.push_char(c),
            KeyCode::Char('r' | 'R') => {
                if state.row == 2 {
                    state.toggle_read();
                } else if state.row == 5 {
                    state.toggle();
                }
            }
            KeyCode::Char('w' | 'W') if state.row == 2 => state.toggle_write(),
            KeyCode::Char('x') if state.row == 2 => {
                state.set_exec(form_modal::ExecPerm::Always);
            }
            KeyCode::Char('X') if state.row == 2 => {
                state.set_exec(form_modal::ExecPerm::Conditional);
            }
            KeyCode::Char('a' | 'A') if state.row == 3 => state.toggle(),
            KeyCode::Char('d' | 'D') if state.row == 4 => state.toggle(),
            _ => {}
        }
        return;
    }

    if let Some(ModalKind::RemoveAcl(state)) = &mut app.modal.kind {
        match key.code {
            KeyCode::Esc => app.modal.close(),
            KeyCode::Enter => submit_modal(app),
            KeyCode::Up | KeyCode::BackTab => state.move_row(-1),
            KeyCode::Down | KeyCode::Tab => state.move_row(1),
            KeyCode::Char(' ') => state.toggle(),
            _ => {}
        }
        return;
    }

    if let Some(ModalKind::PasteAcl(state)) = &mut app.modal.kind {
        match key.code {
            KeyCode::Esc => app.modal.close(),
            KeyCode::Enter => submit_modal(app),
            KeyCode::Up | KeyCode::BackTab => state.move_row(-1),
            KeyCode::Down | KeyCode::Tab => state.move_row(1),
            KeyCode::Left => state.move_col(-1),
            KeyCode::Right => state.move_col(1),
            KeyCode::Backspace => state.backspace(),
            KeyCode::Delete => state.delete(),
            KeyCode::Char(' ') => state.toggle_recursive(),
            KeyCode::Char(c) => state.push_char(c),
            _ => {}
        }
        return;
    }

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

// --- Resource Manager actions & modal openers -------------------------

fn toggle_hybrid_resources(app: &mut App) {
    app.resources.hybrid_panel = !app.resources.hybrid_panel;
    if !app.resources.hybrid_panel && app.focus == Focus::Details {
        app.focus = Focus::Content;
    }
}

fn handle_resource_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Down => app.move_resource_selection(1),
        KeyCode::Up => app.move_resource_selection(-1),
        KeyCode::Char(' ') => app.toggle_select_resource(),
        KeyCode::Enter => app.open_selected_resource(),
        KeyCode::Backspace | KeyCode::Char('-') => app.go_parent_resource(),
        KeyCode::Char('.') => app.toggle_hidden_resources(),
        KeyCode::Char('h' | 'H') if app.hybrid_active && app.page == Page::Users => {
            open_selected_user_home(app);
        }
        KeyCode::Char('g' | 'G') => open_goto_path(app),
        KeyCode::Char('m' | 'M') => open_chmod(app),
        KeyCode::Char('o' | 'O') => open_chown(app),
        KeyCode::Char('a' | 'A') => open_set_acl(app),
        KeyCode::Char('x' | 'X') => open_remove_acl(app),
        KeyCode::Char('b' | 'B') => open_clear_acl(app),
        KeyCode::Char('c' | 'C') => copy_selected_acl(app),
        KeyCode::Char('v' | 'V') => open_paste_acl(app),
        _ => {}
    }
}

fn open_selected_user_home(app: &mut App) {
    let Some(user) = app.selected_user().cloned() else {
        app.notify(false, "no user selected");
        return;
    };
    let path = std::path::PathBuf::from(&user.home);
    match app.goto_resource_path(path) {
        Ok(()) => app.notify(true, format!("opened {}", user.home)),
        Err(err) => app.notify(false, format!("home '{}': {err}", user.home)),
    }
}

fn open_goto_path(app: &mut App) {
    let cwd = app.resources.cwd.display().to_string();
    app.modal = form_modal::Modal::goto_path(&cwd);
}

fn open_chmod(app: &mut App) {
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select a file or directory first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let mode = targets[0].mode;
    let is_dir = targets
        .iter()
        .any(|e| e.kind == crate::models::EntryKind::Directory);
    app.modal = form_modal::Modal::chmod(paths, mode, is_dir);
}

fn open_chown(app: &mut App) {
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select a file or directory first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let owner = targets[0].owner_label();
    let is_dir = targets
        .iter()
        .any(|e| e.kind == crate::models::EntryKind::Directory);
    app.modal = form_modal::Modal::chown(paths, &owner, is_dir);
}

fn open_set_acl(app: &mut App) {
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select a file or directory first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let is_dir = targets
        .iter()
        .any(|e| e.kind == crate::models::EntryKind::Directory);
    let existing_acl = if app.selected_resource().is_some_and(|e| e.path == paths[0]) {
        app.resources.selected_acl.clone()
    } else {
        crate::services::resource_service::read_acl(&paths[0])
    };

    let users: Vec<form_modal::CandidateItem> = app
        .data
        .users
        .iter()
        .map(|u| form_modal::CandidateItem {
            name: u.name.clone(),
            meta: format!("UID {}", u.uid),
            is_regular: u.uid >= 1000,
        })
        .collect();
    let groups: Vec<form_modal::CandidateItem> = app
        .data
        .groups
        .iter()
        .map(|g| form_modal::CandidateItem {
            name: g.name.clone(),
            meta: format!("GID {}", g.gid),
            is_regular: g.gid >= 1000,
        })
        .collect();

    let (scope, prefill_name) = if app.hybrid_active {
        match app.page {
            Page::Users => (
                form_modal::AclTargetScope::User,
                app.selected_user()
                    .map(|u| u.name.clone())
                    .unwrap_or_default(),
            ),
            Page::Groups => (
                form_modal::AclTargetScope::Group,
                app.selected_group()
                    .map(|g| g.name.clone())
                    .unwrap_or_default(),
            ),
            _ => (form_modal::AclTargetScope::User, String::new()),
        }
    } else {
        (form_modal::AclTargetScope::User, String::new())
    };
    app.modal = form_modal::Modal::set_acl(
        paths,
        scope,
        &prefill_name,
        is_dir,
        users,
        groups,
        existing_acl,
    );
}

fn open_remove_acl(app: &mut App) {
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select a file or directory first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let is_dir = targets
        .iter()
        .any(|e| e.kind == crate::models::EntryKind::Directory);
    let acls: Vec<crate::models::AclInfo> = paths
        .iter()
        .map(|p| {
            if app.selected_resource().is_some_and(|e| &e.path == p) {
                app.resources.selected_acl.clone()
            } else {
                crate::services::resource_service::read_acl(p)
            }
        })
        .collect();
    if acls.iter().all(|a| !a.available) {
        app.notify(false, "getfacl unavailable — install `acl` package");
        return;
    }
    let pre_user = if app.hybrid_active && app.page == Page::Users {
        app.selected_user().map(|u| u.name.as_str())
    } else {
        None
    };
    let pre_group = if app.hybrid_active && app.page == Page::Groups {
        app.selected_group().map(|g| g.name.as_str())
    } else {
        None
    };
    let state =
        form_modal::RemoveAclState::from_acls(paths.clone(), is_dir, &acls, pre_user, pre_group);
    if state.items.is_empty() {
        if acls.iter().any(|a| a.has_extended_acl()) {
            app.notify(
                false,
                "only mask/base ACL entries remain — press B to clear all extended ACLs",
            );
        } else {
            app.notify(
                false,
                format!(
                    "no extended ACL entries to remove on {}",
                    form_modal::format_paths_title(&paths)
                ),
            );
        }
        return;
    }
    app.modal = form_modal::Modal::remove_acl(state);
}

fn open_clear_acl(app: &mut App) {
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select a file or directory first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let label = form_modal::format_paths_title(&paths);
    app.modal = form_modal::Modal::confirm(
        ModalKind::ConfirmClearAcl(paths),
        &format!("Clear all extended ACLs on '{label}'? Type YES."),
    );
}

fn copy_selected_acl(app: &mut App) {
    let Some(entry) = app
        .selected_resource()
        .filter(|e| !e.is_parent_link)
        .cloned()
    else {
        app.notify(false, "select a file or directory to copy its ACL");
        return;
    };
    match crate::services::resource_service::export_acl(&entry.path) {
        Ok(rules_text) => {
            let rule_count = rules_text.lines().count();
            app.resources.acl_clipboard = Some(crate::app::CopiedAcl {
                source_label: entry.name.clone(),
                rules_text,
                rule_count,
            });
            app.notify(
                true,
                format!("copied ACL from '{}' ({rule_count} rules)", entry.name),
            );
        }
        Err(err) => app.notify(false, err),
    }
}

fn open_paste_acl(app: &mut App) {
    let Some(clip) = app.resources.acl_clipboard.clone() else {
        app.notify(false, "ACL clipboard empty — press C on an item first");
        return;
    };
    let targets = app.target_resource_entries();
    if targets.is_empty() {
        app.notify(false, "select target file(s) or directory(s) first");
        return;
    }
    let paths: Vec<std::path::PathBuf> = targets.iter().map(|e| e.path.clone()).collect();
    let is_dir = targets
        .iter()
        .any(|e| e.kind == crate::models::EntryKind::Directory);
    let state = form_modal::PasteAclState::new(
        paths,
        is_dir,
        clip.source_label,
        clip.rules_text,
        clip.rule_count,
    );
    app.modal = form_modal::Modal::paste_acl(state);
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
    if kind == ModalKind::GoToPath {
        let raw = app.modal.fields[0].value.trim().to_string();
        if raw.is_empty() {
            app.modal.error = Some("directory path is required".to_string());
            return;
        }
        match app.goto_resource_path(std::path::PathBuf::from(&raw)) {
            Ok(()) => {
                app.modal.close();
                app.notify(true, format!("opened {raw}"));
            }
            Err(err) => {
                app.modal.error = Some(err);
            }
        }
        return;
    }

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
        ModalKind::GoToPath => unreachable!("handled before root gate"),
        ModalKind::Chmod(state) => submit_chmod(app, state),
        ModalKind::Chown { paths, is_dir } => submit_chown(app, paths, is_dir),
        ModalKind::SetAcl(state) => submit_set_acl(app, state),
        ModalKind::RemoveAcl(state) => submit_remove_acl(app, state),
        ModalKind::PasteAcl(state) => submit_paste_acl(app, state),
        ModalKind::ConfirmClearAcl(paths) => {
            if confirmed(app) {
                let res = crate::services::resource_service::clear_acl(&paths, false);
                audit_resources_and_finish(app, "clear_acl", &paths, res);
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::ConfirmRecursive(op) => {
            if confirmed(app) {
                submit_recursive_op(app, op);
            } else {
                refuse_confirmation(app);
            }
        }
    }
}

fn wants_recursive(app: &App, is_dir: bool) -> bool {
    is_dir
        && app
            .modal
            .fields
            .get(1)
            .is_some_and(|f| matches!(f.value.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

fn prompt_recursive(app: &mut App, paths: &[std::path::PathBuf], op: form_modal::RecursiveOp) {
    if let Err(res) = crate::services::resource_service::guard_recursive_paths(paths) {
        app.modal.error = Some(res.message);
        return;
    }
    let (count, truncated) =
        crate::services::resource_service::count_recursive_items_many(paths, 1000);
    let label = form_modal::format_paths_title(paths);
    let msg = format!(
        "Apply recursively to '{label}' ({}{} items)? Type YES.",
        count,
        if truncated { "+" } else { "" }
    );
    app.modal = form_modal::Modal::confirm_recursive(op, &msg);
}

fn submit_chmod(app: &mut App, state: form_modal::ChmodState) {
    let paths = state.paths.clone();
    let mode = state.octal_string();
    if state.recursive {
        prompt_recursive(
            app,
            &paths,
            form_modal::RecursiveOp::Chmod {
                paths: paths.clone(),
                mode,
            },
        );
        return;
    }
    let res = crate::services::resource_service::set_mode(&paths, &mode, false);
    audit_resources_and_finish(app, "chmod", &paths, res);
}

fn submit_chown(app: &mut App, paths: Vec<std::path::PathBuf>, is_dir: bool) {
    let raw_owner = app.modal.fields[0].value.trim().to_string();
    let owner = match crate::models::resource::validate_owner(&raw_owner) {
        Ok(o) => o,
        Err(err) => {
            app.modal.error = Some(err);
            return;
        }
    };
    if wants_recursive(app, is_dir) {
        prompt_recursive(
            app,
            &paths,
            form_modal::RecursiveOp::Chown {
                paths: paths.clone(),
                owner,
            },
        );
        return;
    }
    let res = crate::services::resource_service::set_owner(&paths, &owner, false);
    audit_resources_and_finish(app, "chown", &paths, res);
}

fn submit_set_acl(app: &mut App, state: form_modal::SetAclState) {
    let paths = state.paths.clone();
    let specs = state.build_specs();
    if specs.is_empty() {
        app.modal.error = Some(
            if state.scope.needs_name() && state.name.trim().is_empty() {
                "select or enter target name".to_string()
            } else {
                "enable Access and/or Default ACL".to_string()
            },
        );
        return;
    }
    for s in &specs {
        if let Err(err) = crate::models::resource::validate_acl_spec(s) {
            app.modal.error = Some(err);
            return;
        }
    }
    if state.recursive {
        prompt_recursive(
            app,
            &paths,
            form_modal::RecursiveOp::SetAcl {
                paths: paths.clone(),
                specs,
            },
        );
        return;
    }
    let res = crate::services::resource_service::set_acl_many(&paths, &specs, false);
    audit_resources_and_finish(app, "set_acl", &paths, res);
}

fn submit_remove_acl(app: &mut App, state: form_modal::RemoveAclState) {
    let paths = state.paths.clone();
    let specs = state.selected_specs();
    let clear_default = state.wants_clear_default();
    if specs.is_empty() && !clear_default {
        app.modal.error = Some("select at least one ACL entry (Space)".to_string());
        return;
    }
    if state.recursive {
        prompt_recursive(
            app,
            &paths,
            form_modal::RecursiveOp::RemoveAcl {
                paths: paths.clone(),
                specs,
                clear_default,
            },
        );
        return;
    }
    let res =
        crate::services::resource_service::remove_acl_many(&paths, &specs, clear_default, false);
    audit_resources_and_finish(app, "remove_acl", &paths, res);
}

fn submit_paste_acl(app: &mut App, state: form_modal::PasteAclState) {
    if !state.is_confirmed() {
        app.modal.error = Some("type YES to confirm overwriting target ACLs".to_string());
        return;
    }
    if state.recursive {
        if let Err(res) = crate::services::resource_service::guard_recursive_paths(&state.paths) {
            app.modal.error = Some(res.message);
            return;
        }
    }
    let res = crate::services::resource_service::paste_acl(
        &state.paths,
        &state.acl_text,
        state.recursive,
    );
    let op = if state.recursive {
        "paste_acl_recursive"
    } else {
        "paste_acl"
    };
    audit_resources_and_finish(app, op, &state.paths, res);
}

fn submit_recursive_op(app: &mut App, op: form_modal::RecursiveOp) {
    match op {
        form_modal::RecursiveOp::Chmod { paths, mode } => {
            let res = crate::services::resource_service::set_mode(&paths, &mode, true);
            audit_resources_and_finish(app, "chmod_recursive", &paths, res);
        }
        form_modal::RecursiveOp::Chown { paths, owner } => {
            let res = crate::services::resource_service::set_owner(&paths, &owner, true);
            audit_resources_and_finish(app, "chown_recursive", &paths, res);
        }
        form_modal::RecursiveOp::SetAcl { paths, specs } => {
            let res = crate::services::resource_service::set_acl_many(&paths, &specs, true);
            audit_resources_and_finish(app, "set_acl_recursive", &paths, res);
        }
        form_modal::RecursiveOp::RemoveAcl {
            paths,
            specs,
            clear_default,
        } => {
            let res = crate::services::resource_service::remove_acl_many(
                &paths,
                &specs,
                clear_default,
                true,
            );
            audit_resources_and_finish(app, "remove_acl_recursive", &paths, res);
        }
    }
}

fn audit_resources_and_finish(
    app: &mut App,
    operation: &'static str,
    paths: &[std::path::PathBuf],
    mut res: crate::services::user_service::OpResult,
) {
    let mut audit_err = None;
    for p in paths {
        let target = p.display().to_string();
        let entry =
            crate::system::audit::AuditEntry::from_result(operation, &target, res.ok, &res.message);
        if let Err(err) = crate::system::audit::record(&entry) {
            audit_err = Some(err);
        }
    }
    if res.ok {
        app.clear_selected_resources();
    }
    if let Some(err) = audit_err {
        res.message = format!("{} (audit log: {err})", res.message);
    }
    finish(app, res);
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
    if !res.ok {
        audit_and_finish(app, "create_user", &name, res);
        return;
    }

    let mut audit_notes: Vec<String> = Vec::new();
    record_audit_best_effort("create_user", &name, &res, &mut audit_notes);

    let mut final_ok = true;
    let mut message = res.message.clone();

    if !groups.is_empty() {
        // Supplementary groups right after creation.
        let list: Vec<&str> = groups
            .split(',')
            .map(|g| g.trim())
            .filter(|g| !g.is_empty())
            .collect();
        if !list.is_empty() {
            let res_grp = crate::services::user_service::add_to_groups(&name, &list);
            record_audit_best_effort("add_to_groups", &name, &res_grp, &mut audit_notes);
            if !res_grp.ok {
                final_ok = false;
                message = format!("user created, but groups failed: {}", res_grp.message);
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
        record_audit_best_effort("set_password", &name, &res_pw, &mut audit_notes);
        if !res_pw.ok {
            final_ok = false;
            if message == res.message {
                message = format!("user created, but password failed: {}", res_pw.message);
            } else {
                message.push_str(&format!("; password failed: {}", res_pw.message));
            }
        }
    }

    if !audit_notes.is_empty() {
        message.push_str(&format!(" (audit log: {})", audit_notes.join("; ")));
    }
    app.modal.close();
    app.notify(final_ok, message);
    app.reload();
}

fn record_audit_best_effort(
    operation: &'static str,
    target: &str,
    res: &crate::services::user_service::OpResult,
    notes: &mut Vec<String>,
) {
    let entry =
        crate::system::audit::AuditEntry::from_result(operation, target, res.ok, &res.message);
    if let Err(err) = crate::system::audit::record(&entry) {
        notes.push(err);
    }
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
        Page::Resources => app.move_resource_selection(delta),
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

    #[test]
    fn hybrid_panel_toggle_and_contextual_acl_prefill() {
        let mut app = App::default();
        app.page = Page::Users;
        app.focus = Focus::Details;
        app.hybrid_active = true;
        app.reload();
        assert!(!app.data.users.is_empty());

        let expected_user = app.selected_user().unwrap().name.clone();
        let temp_file = std::env::temp_dir().join("usera_keymap_hybrid_test");
        let _ = std::fs::write(&temp_file, b"ok");
        app.goto_resource_path(std::env::temp_dir()).unwrap();
        if let Some(idx) = app
            .active_resource_entries()
            .iter()
            .position(|e| e.path == temp_file)
        {
            app.resources.selected = idx;
            handle_key(&mut app, key_press(KeyCode::Char('a')));
            assert!(app.modal.is_open());
            match &app.modal.kind {
                Some(form_modal::ModalKind::SetAcl(st)) => {
                    assert_eq!(st.scope, form_modal::AclTargetScope::User);
                    assert_eq!(st.name, expected_user);
                    assert_eq!(st.build_specs(), vec![format!("u:{expected_user}:rw-")]);
                }
                other => panic!("expected SetAcl modal, got {other:?}"),
            }
            app.modal.close();
        }
        let _ = std::fs::remove_file(&temp_file);

        // Pressing F toggles hybrid panel off and shifts focus back to Content
        handle_key(&mut app, key_press(KeyCode::Char('f')));
        assert!(!app.resources.hybrid_panel);
        assert_eq!(app.focus, Focus::Content);
    }

    #[test]
    fn recursive_chmod_refuses_protected_system_paths_in_modal() {
        let mut app = App::default();
        app.data.is_root = true;
        app.modal = form_modal::Modal::chmod(vec![std::path::PathBuf::from("/etc")], 0o0755, true);
        // Move to row 4 (Recursive) and toggle with Space
        handle_key(&mut app, key_press(KeyCode::Up));
        handle_key(&mut app, key_press(KeyCode::Char(' ')));
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(app
            .modal
            .error
            .as_deref()
            .unwrap_or("")
            .contains("protected system path"));
    }

    #[test]
    fn chmod_modal_checkbox_navigation_updates_mode() {
        let mut app = App::default();
        app.modal =
            form_modal::Modal::chmod(vec![std::path::PathBuf::from("/tmp/demo")], 0o0640, false);

        // Toggle Owner Execute (row 0, col 2) using right arrows + Space -> 0740
        handle_key(&mut app, key_press(KeyCode::Right));
        handle_key(&mut app, key_press(KeyCode::Right));
        handle_key(&mut app, key_press(KeyCode::Char(' ')));
        // Move down to Group (row 1) and quick-toggle Write via 'w' -> 0760
        handle_key(&mut app, key_press(KeyCode::Down));
        handle_key(&mut app, key_press(KeyCode::Char('w')));

        match &app.modal.kind {
            Some(form_modal::ModalKind::Chmod(state)) => {
                assert_eq!(state.octal_string(), "0760");
                assert_eq!(state.rwx_string(), "-rwxrw----");
            }
            other => panic!("expected Chmod modal, got {other:?}"),
        }
    }

    #[test]
    fn dot_toggles_hidden_dotfiles_and_h_jumps_to_user_home() {
        let base = std::env::temp_dir().join(format!("usera_dot_home_test_{}", std::process::id()));
        let home_dir = base.join("home_alice");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&home_dir).unwrap();
        std::fs::write(base.join(".env"), b"SECRET=1").unwrap();
        std::fs::write(base.join("readme.txt"), b"ok").unwrap();

        let mut app = App::default();
        app.page = Page::Users;
        app.focus = Focus::Details;
        app.hybrid_active = true;
        app.data.users = vec![crate::models::User {
            name: "alice".into(),
            uid: 1001,
            gid: 1001,
            gecos: String::new(),
            home: home_dir.display().to_string(),
            shell: "/bin/bash".into(),
            supplementary_groups: vec![],
        }];
        app.goto_resource_path(base.clone()).unwrap();

        // Dotfiles shown by default
        assert!(app
            .active_resource_entries()
            .iter()
            .any(|e| e.name == ".env"));

        // Pressing '.' hides dotfiles
        handle_key(&mut app, key_press(KeyCode::Char('.')));
        assert!(!app.resources.show_hidden);
        assert!(!app
            .active_resource_entries()
            .iter()
            .any(|e| e.name == ".env"));
        assert!(app
            .active_resource_entries()
            .iter()
            .any(|e| e.name == "readme.txt"));

        // Pressing 'H' in Hybrid Users mode jumps straight to alice's home directory
        handle_key(&mut app, key_press(KeyCode::Char('h')));
        assert_eq!(app.resources.cwd, home_dir);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn set_acl_typing_shows_autocomplete_and_tab_completes_into_perms() {
        let mut app = App::default();
        app.page = Page::Resources;
        app.focus = Focus::Content;
        app.data.users = vec![crate::models::User {
            name: "deploy".into(),
            uid: 1001,
            gid: 1001,
            gecos: String::new(),
            home: "/home/deploy".into(),
            shell: "/bin/bash".into(),
            supplementary_groups: vec![],
        }];
        let tmp = std::env::temp_dir().join("usera_acl_tab_test");
        let _ = std::fs::write(&tmp, b"x");
        app.goto_resource_path(std::env::temp_dir()).unwrap();
        let idx = app
            .active_resource_entries()
            .iter()
            .position(|e| e.path == tmp)
            .unwrap();
        app.resources.selected = idx;
        let _ = std::fs::remove_file(&tmp);

        handle_key(&mut app, key_press(KeyCode::Char('a')));
        // Type 'd' then 'e', then press Tab to autocomplete 'deploy' and move to Perms row
        handle_key(&mut app, key_press(KeyCode::Char('d')));
        handle_key(&mut app, key_press(KeyCode::Char('e')));
        // Press Enter when dropdown is visible: selects 'deploy' and moves to Perms (does NOT submit!)
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(app.modal.is_open());

        match &app.modal.kind {
            Some(form_modal::ModalKind::SetAcl(st)) => {
                assert_eq!(st.name, "deploy");
                assert_eq!(st.row, 2);
                assert_eq!(st.build_specs(), vec!["u:deploy:rw-".to_string()]);
            }
            other => panic!("expected SetAcl modal, got {other:?}"),
        }
    }

    #[test]
    fn set_acl_loads_existing_perms_and_supports_capital_x_and_multi_select() {
        use crate::models::{AclEntry, AclInfo, AclScope};

        let base =
            std::env::temp_dir().join(format!("usera_multi_acl_test_{}", std::process::id()));
        let d1 = base.join("dir_a");
        let d2 = base.join("dir_b");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&d1).unwrap();
        std::fs::create_dir_all(&d2).unwrap();

        let mut app = App::default();
        app.page = Page::Resources;
        app.focus = Focus::Content;
        app.data.users = vec![crate::models::User {
            name: "deploy".into(),
            uid: 1001,
            gid: 1001,
            gecos: String::new(),
            home: "/home/deploy".into(),
            shell: "/bin/bash".into(),
            supplementary_groups: vec![],
        }];
        app.goto_resource_path(base.clone()).unwrap();

        // Move down past '..' to dir_a, press Space (marks dir_a and advances to dir_b), press Space
        handle_key(&mut app, key_press(KeyCode::Down));
        handle_key(&mut app, key_press(KeyCode::Char(' ')));
        handle_key(&mut app, key_press(KeyCode::Char(' ')));
        assert_eq!(app.resources.selected_paths.len(), 2);
        assert!(app.resources.selected_paths.contains(&d1));
        assert!(app.resources.selected_paths.contains(&d2));

        // Inject existing ACL for 'deploy' (r-x access + default) on selected_acl
        app.resources.selected_acl = AclInfo {
            available: true,
            entries: vec![
                AclEntry {
                    is_default: false,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "r-x".into(),
                    effective: None,
                },
                AclEntry {
                    is_default: true,
                    scope: AclScope::User,
                    qualifier: "deploy".into(),
                    perms: "r-x".into(),
                    effective: None,
                },
            ],
            error: None,
        };
        app.resources.selected = 1; // dir_a

        // Press 'a' -> multi-target modal opens with 2 paths
        handle_key(&mut app, key_press(KeyCode::Char('a')));
        // Type 'de' and press Enter -> selects 'deploy', loads existing r-x + Access & Default!
        handle_key(&mut app, key_press(KeyCode::Char('d')));
        handle_key(&mut app, key_press(KeyCode::Char('e')));
        handle_key(&mut app, key_press(KeyCode::Enter));
        // Switch Exec to Conditional capital 'X'
        handle_key(&mut app, key_press(KeyCode::Char('X')));

        match &app.modal.kind {
            Some(form_modal::ModalKind::SetAcl(st)) => {
                assert_eq!(st.paths.len(), 2);
                assert!(st.loaded_existing);
                assert!(st.read && !st.write);
                assert_eq!(st.exec, form_modal::ExecPerm::Conditional);
                assert_eq!(st.build_specs(), vec!["u:deploy:r-X", "d:u:deploy:r-X"]);
            }
            other => panic!("expected SetAcl modal, got {other:?}"),
        }
        app.modal.close();

        // Esc clears multi-select
        handle_key(&mut app, key_press(KeyCode::Esc));
        assert!(app.resources.selected_paths.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_and_paste_acl_clones_from_source_and_guards_protected_recursive() {
        let base = std::env::temp_dir().join(format!("usera_copy_paste_{}", std::process::id()));
        let src = base.join("src_dir");
        let dst = base.join("dst_dir");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        crate::system::acl::set_acl(&[src.clone()], "u:root:rwx,d:u:root:rwx", false).unwrap();

        let mut app = App::default();
        app.page = Page::Resources;
        app.focus = Focus::Content;
        app.data.is_root = true;
        app.goto_resource_path(base.clone()).unwrap();

        // Copy from src_dir (C)
        let src_idx = app
            .active_resource_entries()
            .iter()
            .position(|e| e.path == src)
            .unwrap();
        app.resources.selected = src_idx;
        handle_key(&mut app, key_press(KeyCode::Char('c')));
        let clip = app.resources.acl_clipboard.as_ref().expect("ACL copied");
        assert_eq!(clip.source_label, "src_dir");
        assert!(clip.rules_text.contains("user:root:rwx"));

        // Move to dst_dir and press V -> PasteAcl modal opens
        let dst_idx = app
            .active_resource_entries()
            .iter()
            .position(|e| e.path == dst)
            .unwrap();
        app.resources.selected = dst_idx;
        handle_key(&mut app, key_press(KeyCode::Char('v')));
        match &app.modal.kind {
            Some(form_modal::ModalKind::PasteAcl(st)) => {
                assert_eq!(st.paths, vec![dst.clone()]);
                assert_eq!(st.source_label, "src_dir");
            }
            other => panic!("expected PasteAcl modal, got {other:?}"),
        }

        // Submitting without YES keeps modal open with inline error
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(app.modal.is_open());
        assert!(app.modal.error.as_deref().unwrap_or("").contains("YES"));

        // Typing YES and pressing Enter pastes ACL onto dst_dir cleanly
        for c in "YES".chars() {
            handle_key(&mut app, key_press(KeyCode::Char(c)));
        }
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(!app.modal.is_open());
        let dst_acl = crate::services::resource_service::read_acl(&dst);
        assert!(dst_acl
            .entries
            .iter()
            .any(|e| !e.is_default && e.qualifier == "root"));

        let _ = std::fs::remove_dir_all(&base);
    }
}
