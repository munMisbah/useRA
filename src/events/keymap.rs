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

    match (key.code, key.modifiers) {
        // Global keys.
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('Q'), KeyModifiers::NONE) => {
            app.quit();
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.quit(),
        (KeyCode::Char('?'), _) => app.show_help = true,
        (KeyCode::Char('r'), KeyModifiers::NONE) | (KeyCode::Char('R'), KeyModifiers::NONE) => {
            app.reload();
        }
        (KeyCode::Tab, _) => app.next_focus(),
        (KeyCode::Esc, _) => app.notification = None,

        // Focus-dependent keys.
        _ => match app.focus {
            Focus::Sidebar => match key.code {
                KeyCode::Down => app.next_page(),
                KeyCode::Up => app.previous_page(),
                KeyCode::Left | KeyCode::Right => app.next_focus(),
                KeyCode::Enter => app.next_focus(),
                _ => {}
            },
            Focus::Content => {
                let user_page = app.page == Page::Users;
                match key.code {
                    KeyCode::Down => move_content_selection(app, 1),
                    KeyCode::Up => move_content_selection(app, -1),
                    KeyCode::Left | KeyCode::Right => app.next_focus(),
                    KeyCode::Enter => {}
                    // Operation keys on the Users page.
                    KeyCode::Char('a' | 'A') if user_page => open_create_user(app),
                    KeyCode::Char('e' | 'E') if user_page => open_edit_shell(app),
                    KeyCode::Char('d' | 'D') if user_page => open_delete_confirm(app),
                    KeyCode::Char('l' | 'L') if user_page => open_lock_confirm(app),
                    KeyCode::Char('u' | 'U') if user_page => open_unlock_confirm(app),
                    KeyCode::Char('g' | 'G') if user_page => open_add_groups(app),
                    KeyCode::Char('x' | 'X') if user_page => open_set_expiry(app),
                    _ => {}
                }
            }
            Focus::Details => match key.code {
                KeyCode::Left | KeyCode::Right => app.next_focus(),
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

// --- Modal openers ---------------------------------------------------

fn open_create_user(app: &mut App) {
    app.modal = form_modal::Modal::create_user();
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

// --- Modal submission -------------------------------------------------

/// Executes the pending operation described by the modal.
fn submit_modal(app: &mut App) {
    let kind = match app.modal.kind.clone() {
        Some(kind) => kind,
        None => return,
    };

    match kind {
        ModalKind::CreateUser => submit_create_user(app),
        ModalKind::EditShell(user) => {
            let shell = app.modal.fields[0].value.trim().to_string();
            let res = crate::services::user_service::set_shell(&user, &shell);
            finish(app, res);
        }
        ModalKind::ConfirmLock(user) => {
            if confirmed(app) {
                finish(app, crate::services::user_service::lock_user(&user));
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::ConfirmUnlock(user) => {
            if confirmed(app) {
                finish(app, crate::services::user_service::unlock_user(&user));
            } else {
                refuse_confirmation(app);
            }
        }
        ModalKind::ConfirmDelete { user, remove_home } => {
            if confirmed(app) {
                finish(
                    app,
                    crate::services::user_service::delete_user(&user, remove_home),
                );
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
            finish(
                app,
                crate::services::user_service::add_to_groups(&user, &groups),
            );
        }
        ModalKind::SetExpiry(user) => {
            let raw = app.modal.fields[0].value.trim().to_string();
            let date = if raw.is_empty() {
                None
            } else {
                Some(raw.as_str())
            };
            finish(app, crate::services::user_service::set_expiry(&user, date));
        }
    }
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
    if res.ok && !groups.is_empty() {
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
            let len = app.data.users.len();
            app.selections.users =
                crate::ui::components::table::move_selection(app.selections.users, len, delta);
        }
        Page::Groups => {
            let len = app.data.groups.len();
            app.selections.groups =
                crate::ui::components::table::move_selection(app.selections.groups, len, delta);
        }
        Page::Sessions => {
            let len = app.data.sessions.len();
            app.selections.sessions =
                crate::ui::components::table::move_selection(app.selections.sessions, len, delta);
        }
        Page::Dashboard | Page::Settings => {}
    }
}
