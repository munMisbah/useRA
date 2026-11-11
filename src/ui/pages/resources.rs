//! Resources page (Option A) plus Hybrid contextual right-panel and
//! bottom details renderers for Users/Groups.

use crate::app::{App, Focus, Page};
use crate::models::{AclScope, EntryKind, ResourceEntry};
use crate::ui::components::{details, table};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Renders the dedicated Resources page (Center browser + Right Details & ACL).
pub fn render(frame: &mut Frame, app: &mut App, content: Rect, details_area: Rect) {
    render_browser(frame, app, content, app.focus == Focus::Content);

    if details_area.width > 0 {
        render_right_details_and_acl(frame, app, details_area);
    }
}

/// Renders the contextual Resource Manager inside the Right panel during Hybrid mode.
pub fn render_hybrid_right(frame: &mut Frame, app: &mut App, area: Rect) {
    render_browser(frame, app, area, app.focus == Focus::Details);
}

/// Renders the compact bottom Details strip spanning Center + Right in Hybrid mode.
pub fn render_hybrid_bottom_details(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    match app.page {
        Page::Users => {
            if let Some(u) = app.selected_user() {
                let groups = if u.supplementary_groups.is_empty() {
                    "(none)".to_string()
                } else {
                    u.supplementary_groups.join(", ")
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} User: ", crate::ui::icons::USER),
                        crate::ui::theme::accent(),
                    ),
                    Span::styled(u.name.clone(), crate::ui::theme::title()),
                    Span::styled(
                        format!(
                            "  (UID {}, GID {}, {})  Shell: {}  Home: {}",
                            u.uid,
                            u.gid,
                            u.account_type().label(),
                            u.shell,
                            u.home
                        ),
                        crate::ui::theme::base(),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} Groups: ", crate::ui::icons::GROUPS),
                        crate::ui::theme::dim(),
                    ),
                    Span::styled(groups, crate::ui::theme::dim()),
                ]));
            } else {
                lines.push(Line::from(" No user selected").style(crate::ui::theme::dim()));
            }
        }
        Page::Groups => {
            if let Some(g) = app.selected_group() {
                let members = if g.members.is_empty() {
                    "(none)".to_string()
                } else {
                    g.members.join(", ")
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} Group: ", crate::ui::icons::GROUPS),
                        crate::ui::theme::accent(),
                    ),
                    Span::styled(g.name.clone(), crate::ui::theme::title()),
                    Span::styled(
                        format!("  (GID {})  Members: {members}", g.gid),
                        crate::ui::theme::base(),
                    ),
                ]));
            } else {
                lines.push(Line::from(" No group selected").style(crate::ui::theme::dim()));
            }
        }
        _ => {}
    }

    if let Some(res) = app.selected_resource() {
        let acl_mark = if app.resources.selected_acl.has_extended_acl() {
            "+"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} Path: ", icon_for_entry(res)),
                crate::ui::theme::accent(),
            ),
            Span::styled(res.path.display().to_string(), crate::ui::theme::base()),
            Span::styled(
                format!(
                    "  Mode: {} ({}{acl_mark})  Owner: {}",
                    res.mode_rwx(),
                    res.mode_octal(),
                    res.owner_label()
                ),
                crate::ui::theme::dim(),
            ),
        ]));

        let eff_line = match app.page {
            Page::Users => app.selected_user().map(|u| {
                let eff = app.resources.selected_acl.effective_for_user(
                    &u.name,
                    u.uid,
                    u.gid,
                    &u.supplementary_groups,
                    res,
                );
                (format!("POSIX/ACL for '{}': ", u.name), eff)
            }),
            Page::Groups => app.selected_group().map(|g| {
                let eff = app
                    .resources
                    .selected_acl
                    .effective_for_group(&g.name, g.gid, res);
                (format!("POSIX/ACL for '{}': ", g.name), eff)
            }),
            _ => None,
        };
        if let Some((label, eff)) = eff_line {
            let acl_summary = format_acl_summary(&app.resources.selected_acl);
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} {label}", crate::ui::icons::KEY),
                    crate::ui::theme::dim(),
                ),
                Span::styled(eff, crate::ui::theme::success()),
                Span::styled(acl_summary, crate::ui::theme::dim()),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_browser(frame: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    let dot_tag = if app.resources.show_hidden {
        " [.*]"
    } else {
        " [no .*]"
    };
    let sel_count = app.resources.selected_paths.len();
    let sel_tag = if sel_count > 0 {
        format!(" [{sel_count} sel]")
    } else {
        String::new()
    };
    let clip_tag = app
        .resources
        .acl_clipboard
        .as_ref()
        .map(|c| format!(" [ACL:{}]", shorten_path(&c.source_label, 14)))
        .unwrap_or_default();
    let reserve = 18 + sel_tag.len() + clip_tag.len();
    let cwd_text = shorten_path(
        &app.resources.cwd.display().to_string(),
        (area.width as usize).saturating_sub(reserve),
    );
    let path_line = Line::from(vec![
        Span::styled(
            format!(" {} ", crate::ui::icons::FOLDER),
            crate::ui::theme::accent(),
        ),
        Span::styled(cwd_text, crate::ui::theme::title()),
        Span::styled(dot_tag, crate::ui::theme::dim()),
        Span::styled(sel_tag, crate::ui::theme::accent()),
        Span::styled(clip_tag, crate::ui::theme::success()),
    ]);
    frame.render_widget(Paragraph::new(path_line), chunks[0]);
    if let Some(err) = &app.resources.load_error {
        frame.render_widget(
            Paragraph::new(format!(" {err}")).style(crate::ui::theme::error()),
            chunks[1],
        );
        return;
    }

    let selected_idx = app.resources.selected;
    let selected_has_acl = app.resources.selected_acl.has_extended_acl();
    let rows: Vec<table::RowData> = app
        .active_resource_entries()
        .into_iter()
        .enumerate()
        .map(|(idx, e)| {
            let acl_suffix =
                if (e.has_acl || (idx == selected_idx && selected_has_acl)) && !e.is_parent_link {
                    "+"
                } else {
                    ""
                };
            let mode_str = if e.is_parent_link {
                "<up>".to_string()
            } else {
                format!("{}{}", e.mode_octal(), acl_suffix)
            };
            let prefix = if !e.is_parent_link && app.resources.selected_paths.contains(&e.path) {
                "✓ "
            } else {
                ""
            };
            table::RowData {
                icon: icon_for_entry(e),
                name: format!("{prefix}{}", e.name),
                attr1: mode_str,
                attr2: if e.is_parent_link {
                    String::new()
                } else {
                    e.owner_label()
                },
            }
        })
        .collect();

    table::render_with_headers(
        frame,
        chunks[1],
        ["Name", "Mode", "Owner"],
        &rows,
        &mut app.resources.table,
        focused,
    );
}

fn render_right_details_and_acl(frame: &mut Frame, app: &App, area: Rect) {
    let Some(entry) = app.selected_resource() else {
        frame.render_widget(
            Paragraph::new("No resource selected").style(crate::ui::theme::dim()),
            area,
        );
        return;
    };

    let has_symlink = entry.symlink_target.is_some();
    let detail_rows = if has_symlink { 7 } else { 6 };
    let chunks =
        Layout::vertical([Constraint::Length(detail_rows), Constraint::Min(3)]).split(area);

    let kind_label = match entry.kind {
        EntryKind::Directory => "Directory",
        EntryKind::File => "File",
        EntryKind::Symlink => "Symlink",
        EntryKind::Other => "Special",
    };
    let acl_mark = if app.resources.selected_acl.has_extended_acl() {
        " (+ACL)"
    } else {
        ""
    };

    let mut fields = vec![
        details::Field::new("Path", entry.path.display().to_string()),
        details::Field::new("Type", kind_label.to_string()),
        details::Field::new(
            "Mode",
            format!("{} ({}){acl_mark}", entry.mode_octal(), entry.mode_rwx()),
        ),
        details::Field::new("Owner", entry.owner_label()),
        details::Field::new("Size", entry.size_label()),
    ];
    if let Some(target) = &entry.symlink_target {
        fields.push(details::Field::new("Target", target.clone()));
    }

    let focused = app.focus == Focus::Details;
    details::render(frame, chunks[0], &fields, focused);

    // Bottom half of right panel: Permissions & POSIX ACL list
    let mut acl_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " ─ Permissions & ACL",
            crate::ui::theme::dim(),
        )),
    ];
    if !app.resources.selected_acl.available {
        if let Some(err) = &app.resources.selected_acl.error {
            acl_lines.push(Line::from(format!(" {err}")).style(crate::ui::theme::warning()));
        } else {
            acl_lines.push(Line::from(" Standard POSIX bits only").style(crate::ui::theme::dim()));
        }
    } else if let Some(err) = &app.resources.selected_acl.error {
        acl_lines.push(Line::from(format!(" {err}")).style(crate::ui::theme::error()));
    } else {
        for acl in &app.resources.selected_acl.entries {
            let style = if !acl.qualifier.is_empty() || acl.is_default {
                crate::ui::theme::success()
            } else if acl.scope == AclScope::Mask {
                crate::ui::theme::warning()
            } else if focused {
                crate::ui::theme::base()
            } else {
                crate::ui::theme::dim()
            };
            acl_lines.push(Line::from(Span::styled(
                format!(" {}", acl.display_line()),
                style,
            )));
        }
    }
    frame.render_widget(Paragraph::new(acl_lines), chunks[1]);
}

fn icon_for_entry(entry: &ResourceEntry) -> &'static str {
    match entry.kind {
        EntryKind::Directory => crate::ui::icons::FOLDER,
        EntryKind::Symlink => crate::ui::icons::LINK,
        EntryKind::File | EntryKind::Other => crate::ui::icons::FILE,
    }
}

fn format_acl_summary(acl: &crate::models::AclInfo) -> String {
    if !acl.available {
        return "  (getfacl unavailable)".to_string();
    }
    let named: Vec<String> = acl
        .entries
        .iter()
        .filter(|e| !e.qualifier.is_empty() || e.is_default)
        .take(3)
        .map(|e| {
            let p = if e.is_default { "d:" } else { "" };
            let s = match e.scope {
                AclScope::User => "u",
                AclScope::Group => "g",
                AclScope::Mask => "m",
                AclScope::Other => "o",
            };
            format!("{p}{s}:{}:{}", e.qualifier, e.effective_perms())
        })
        .collect();
    if named.is_empty() {
        String::new()
    } else {
        format!("  |  ACL: {}", named.join(", "))
    }
}

fn shorten_path(path: &str, max_chars: usize) -> String {
    if max_chars < 6 || path.chars().count() <= max_chars {
        return path.to_string();
    }
    let tail: String = path
        .chars()
        .rev()
        .take(max_chars.saturating_sub(3))
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_path_preserves_short_and_truncates_long() {
        assert_eq!(shorten_path("/var/www", 20), "/var/www");
        assert_eq!(
            shorten_path("/var/www/project/releases", 14),
            "...ct/releases"
        );
    }
}
