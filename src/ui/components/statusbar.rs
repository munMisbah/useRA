//! Bottom status and context-sensitive shortcut bar.
//!
//! Only currently functional shortcuts are shown (AGENTS.md: no
//! advertised-but-dead actions). A/E/D appear once write operations
//! land in a later phase.

use crate::app::{App, Focus, Page};
use ratatui::layout::Rect;

use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Shortcut hint shown in the footer.
struct Hint {
    key: &'static str,
    label: &'static str,
}

/// Context-sensitive hints for the current page and focus.
fn hints(app: &App) -> Vec<Hint> {
    let mut hints = vec![Hint {
        key: "←→/Tab",
        label: "Focus",
    }];
    match app.focus {
        Focus::Sidebar => {
            hints.push(Hint {
                key: "↑↓",
                label: "Page",
            });
        }
        Focus::Content => {
            if matches!(
                app.page,
                Page::Users | Page::Groups | Page::Sessions | Page::Resources
            ) && !list_is_empty(app)
            {
                hints.push(Hint {
                    key: "↑↓",
                    label: "Select",
                });
            }
            if app.page == Page::Users && app.data.is_root {
                hints.push(Hint {
                    key: "A",
                    label: "Add",
                });
                hints.push(Hint {
                    key: "P",
                    label: "Passwd",
                });
                hints.push(Hint {
                    key: "E",
                    label: "Shell",
                });
                hints.push(Hint {
                    key: "L/U",
                    label: "Lock/Unlock",
                });
                hints.push(Hint {
                    key: "G",
                    label: "Groups",
                });
                hints.push(Hint {
                    key: "X",
                    label: "Expiry",
                });
                hints.push(Hint {
                    key: "D",
                    label: "Delete",
                });
            }
            if app.page == Page::Users {
                hints.push(Hint {
                    key: "F",
                    label: if app.resources.hybrid_panel {
                        "HideFiles"
                    } else {
                        "Files"
                    },
                });
            }
            if app.page == Page::Groups && app.data.is_root {
                hints.push(Hint {
                    key: "A",
                    label: "Add",
                });
                hints.push(Hint {
                    key: "E",
                    label: "Rename",
                });
                hints.push(Hint {
                    key: "G",
                    label: "+Members",
                });
                hints.push(Hint {
                    key: "R",
                    label: "-Members",
                });
                hints.push(Hint {
                    key: "D",
                    label: "Delete",
                });
            }
            if app.page == Page::Groups {
                hints.push(Hint {
                    key: "F",
                    label: if app.resources.hybrid_panel {
                        "HideFiles"
                    } else {
                        "Files"
                    },
                });
            }
            if app.page == Page::Resources {
                push_resource_hints(&mut hints, app.data.is_root);
            }
        }
        Focus::Details => {
            if app.hybrid_active {
                hints.push(Hint {
                    key: "↑↓",
                    label: "Select",
                });
                push_resource_hints(&mut hints, app.data.is_root);
                hints.push(Hint {
                    key: "F",
                    label: "HideFiles",
                });
            }
        }
    }
    hints.push(Hint {
        key: "r",
        label: "Reload",
    });
    hints.push(Hint {
        key: "?",
        label: "Help",
    });
    hints.push(Hint {
        key: "Q",
        label: "Quit",
    });
    hints
}

fn push_resource_hints(hints: &mut Vec<Hint>, is_root: bool) {
    hints.push(Hint {
        key: "Enter/⌫",
        label: "Nav",
    });
    hints.push(Hint {
        key: "G",
        label: "Path",
    });
    if is_root {
        hints.push(Hint {
            key: "M",
            label: "Mode",
        });
        hints.push(Hint {
            key: "O",
            label: "Owner",
        });
        hints.push(Hint {
            key: "A/X/B",
            label: "ACL",
        });
    }
}

/// Whether the active page's list has no rows.
fn list_is_empty(app: &App) -> bool {
    match app.page {
        Page::Users => app.data.users.is_empty(),
        Page::Groups => app.data.groups.is_empty(),
        Page::Sessions => app.data.sessions.is_empty(),
        Page::Resources => app.active_resource_entries().is_empty(),
        Page::Dashboard | Page::Settings => true,
    }
}

/// Renders the one-line footer: focus indicator, load error, shortcuts.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focus_label = match app.focus {
        Focus::Sidebar => "NAV",
        Focus::Content => "CONTENT",
        Focus::Details => {
            if app.hybrid_active {
                "RESOURCES"
            } else {
                "DETAILS"
            }
        }
    };
    let mut spans = vec![
        Span::styled(
            format!(" {} ", crate::ui::icons::STATUS),
            crate::ui::theme::accent(),
        ),
        Span::styled(focus_label, crate::ui::theme::accent()),
        Span::raw("  "),
    ];
    if let Some(err) = &app.load_error {
        spans.push(Span::styled("ERR ", crate::ui::theme::error()));
        spans.push(Span::styled(err.clone(), crate::ui::theme::warning()));
        spans.push(Span::raw(" · "));
    }
    if let Some((ok, message)) = &app.notification {
        let (tag, style) = if *ok {
            ("OK ", crate::ui::theme::success())
        } else {
            ("ERR ", crate::ui::theme::error())
        };
        spans.push(Span::styled(tag, style));
        spans.push(Span::styled(message.clone(), style));
        spans.push(Span::raw(" · "));
    }
    for (i, hint) in hints(app).into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::styled(
            format!("[{}]", hint.key),
            crate::ui::theme::dim(),
        ));
        spans.push(Span::raw(format!(" {}", hint.label)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
