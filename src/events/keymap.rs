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
                if state.row == 3 {
                    state.toggle();
                } else {
                    state.toggle_col(0);
                }
            }
            KeyCode::Char('w' | 'W') => state.toggle_col(1),
            KeyCode::Char('x' | 'X') => state.toggle_col(2),
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
        KeyCode::Enter => app.open_selected_resource(),
        KeyCode::Backspace | KeyCode::Char('-') => app.go_parent_resource(),
        KeyCode::Char('g' | 'G') => open_goto_path(app),
        KeyCode::Char('m' | 'M') => open_chmod(app),
        KeyCode::Char('o' | 'O') => open_chown(app),
        KeyCode::Char('a' | 'A') => open_set_acl(app),
        KeyCode::Char('x' | 'X') => open_remove_acl(app),
        KeyCode::Char('b' | 'B') => open_clear_acl(app),
        _ => {}
    }
}

fn open_goto_path(app: &mut App) {
    let cwd = app.resources.cwd.display().to_string();
    app.modal = form_modal::Modal::goto_path(&cwd);
}

fn open_chmod(app: &mut App) {
    let Some(entry) = app.selected_resource().filter(|e| !e.is_parent_link) else {
        app.notify(false, "select a file or directory first");
        return;
    };
    let path = entry.path.display().to_string();
    let mode = entry.mode;
    let is_dir = entry.kind == crate::models::EntryKind::Directory;
    app.modal = form_modal::Modal::chmod(path, mode, is_dir);
}

fn open_chown(app: &mut App) {
    let Some(entry) = app.selected_resource().filter(|e| !e.is_parent_link) else {
        app.notify(false, "select a file or directory first");
        return;
    };
    let path = entry.path.display().to_string();
    let owner = entry.owner_label();
    let is_dir = entry.kind == crate::models::EntryKind::Directory;
    app.modal = form_modal::Modal::chown(path, &owner, is_dir);
}

fn open_set_acl(app: &mut App) {
    let Some(entry) = app.selected_resource().filter(|e| !e.is_parent_link) else {
        app.notify(false, "select a file or directory first");
        return;
    };
    let path = entry.path.display().to_string();
    let is_dir = entry.kind == crate::models::EntryKind::Directory;
    let prefill = contextual_acl_prefill(app, true);
    app.modal = form_modal::Modal::set_acl(path, &prefill, is_dir);
}

fn open_remove_acl(app: &mut App) {
    let Some(entry) = app.selected_resource().filter(|e| !e.is_parent_link) else {
        app.notify(false, "select a file or directory first");
        return;
    };
    let path = entry.path.display().to_string();
    let is_dir = entry.kind == crate::models::EntryKind::Directory;
    let prefill = contextual_acl_prefill(app, false);
    app.modal = form_modal::Modal::remove_acl(path, &prefill, is_dir);
}

fn open_clear_acl(app: &mut App) {
    let Some(entry) = app.selected_resource().filter(|e| !e.is_parent_link) else {
        app.notify(false, "select a file or directory first");
        return;
    };
    let path = entry.path.display().to_string();
    app.modal = form_modal::Modal::confirm(
        ModalKind::ConfirmClearAcl(path.clone()),
        &format!("Clear all extended ACLs on '{path}'? Type YES."),
    );
}

fn contextual_acl_prefill(app: &App, with_perms: bool) -> String {
    if !app.hybrid_active {
        return String::new();
    }
    match app.page {
        Page::Users => app
            .selected_user()
            .map(|u| {
                if with_perms {
                    format!("u:{}:rwx", u.name)
                } else {
                    format!("u:{}", u.name)
                }
            })
            .unwrap_or_default(),
        Page::Groups => app
            .selected_group()
            .map(|g| {
                if with_perms {
                    format!("g:{}:rx", g.name)
                } else {
                    format!("g:{}", g.name)
                }
            })
            .unwrap_or_default(),
        _ => String::new(),
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
        ModalKind::Chown { path, is_dir } => submit_chown(app, path, is_dir),
        ModalKind::SetAcl { path, is_dir } => submit_set_acl(app, path, is_dir),
        ModalKind::RemoveAcl { path, is_dir } => submit_remove_acl(app, path, is_dir),
        ModalKind::ConfirmClearAcl(path) => {
            if confirmed(app) {
                let res = crate::services::resource_service::clear_acl(
                    std::path::Path::new(&path),
                    false,
                );
                audit_and_finish(app, "clear_acl", &path, res);
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

fn prompt_recursive(app: &mut App, path: &str, op: form_modal::RecursiveOp) {
    let p = std::path::Path::new(path);
    if let Err(res) = crate::services::resource_service::guard_recursive_path(p) {
        app.modal.error = Some(res.message);
        return;
    }
    let (count, truncated) = crate::services::resource_service::count_recursive_items(p, 1000);
    let msg = format!(
        "Apply recursively to '{path}' ({}{} items)? Type YES.",
        count,
        if truncated { "+" } else { "" }
    );
    app.modal = form_modal::Modal::confirm_recursive(op, &msg);
}

fn submit_chmod(app: &mut App, state: form_modal::ChmodState) {
    let path = state.path.clone();
    let mode = state.octal_string();
    if state.recursive {
        prompt_recursive(
            app,
            &path,
            form_modal::RecursiveOp::Chmod {
                path: path.clone(),
                mode,
            },
        );
        return;
    }
    let res =
        crate::services::resource_service::set_mode(std::path::Path::new(&path), &mode, false);
    audit_and_finish(app, "chmod", &path, res);
}

fn submit_chown(app: &mut App, path: String, is_dir: bool) {
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
            &path,
            form_modal::RecursiveOp::Chown {
                path: path.clone(),
                owner,
            },
        );
        return;
    }
    let res =
        crate::services::resource_service::set_owner(std::path::Path::new(&path), &owner, false);
    audit_and_finish(app, "chown", &path, res);
}

fn submit_set_acl(app: &mut App, path: String, is_dir: bool) {
    let raw_spec = app.modal.fields[0].value.trim().to_string();
    let spec = match crate::models::resource::validate_acl_spec(&raw_spec) {
        Ok(s) => s,
        Err(err) => {
            app.modal.error = Some(err);
            return;
        }
    };
    if wants_recursive(app, is_dir) {
        prompt_recursive(
            app,
            &path,
            form_modal::RecursiveOp::SetAcl {
                path: path.clone(),
                spec,
            },
        );
        return;
    }
    let res = crate::services::resource_service::set_acl(std::path::Path::new(&path), &spec, false);
    audit_and_finish(app, "set_acl", &path, res);
}

fn submit_remove_acl(app: &mut App, path: String, is_dir: bool) {
    let raw_spec = app.modal.fields[0].value.trim().to_string();
    let spec = match crate::models::resource::validate_acl_remove_spec(&raw_spec) {
        Ok(s) => s,
        Err(err) => {
            app.modal.error = Some(err);
            return;
        }
    };
    if wants_recursive(app, is_dir) {
        prompt_recursive(
            app,
            &path,
            form_modal::RecursiveOp::RemoveAcl {
                path: path.clone(),
                spec,
            },
        );
        return;
    }
    let res =
        crate::services::resource_service::remove_acl(std::path::Path::new(&path), &spec, false);
    audit_and_finish(app, "remove_acl", &path, res);
}

fn submit_recursive_op(app: &mut App, op: form_modal::RecursiveOp) {
    match op {
        form_modal::RecursiveOp::Chmod { path, mode } => {
            let res = crate::services::resource_service::set_mode(
                std::path::Path::new(&path),
                &mode,
                true,
            );
            audit_and_finish(app, "chmod_recursive", &path, res);
        }
        form_modal::RecursiveOp::Chown { path, owner } => {
            let res = crate::services::resource_service::set_owner(
                std::path::Path::new(&path),
                &owner,
                true,
            );
            audit_and_finish(app, "chown_recursive", &path, res);
        }
        form_modal::RecursiveOp::SetAcl { path, spec } => {
            let res = crate::services::resource_service::set_acl(
                std::path::Path::new(&path),
                &spec,
                true,
            );
            audit_and_finish(app, "set_acl_recursive", &path, res);
        }
        form_modal::RecursiveOp::RemoveAcl { path, spec } => {
            let res = crate::services::resource_service::remove_acl(
                std::path::Path::new(&path),
                &spec,
                true,
            );
            audit_and_finish(app, "remove_acl_recursive", &path, res);
        }
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
            assert_eq!(app.modal.fields[0].value, format!("u:{expected_user}:rwx"));
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
        app.modal = form_modal::Modal::chmod("/etc".into(), 0o0755, true);
        // Move to row 3 (Recursive) and toggle with Space
        handle_key(&mut app, key_press(KeyCode::Up));
        handle_key(&mut app, key_press(KeyCode::Char(' ')));
        handle_key(&mut app, key_press(KeyCode::Enter));
        assert!(app.modal.is_open());
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
        app.modal = form_modal::Modal::chmod("/tmp/demo".into(), 0o0640, false);

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
}
