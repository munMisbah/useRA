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
            if matches!(app.page, Page::Users | Page::Groups) {
                hints.push(Hint {
                    key: "↑↓",
                    label: "Select",
                });
            }
        }
        Focus::Details => {}
    }
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

/// Renders the one-line footer: focus indicator plus shortcuts.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focus_label = match app.focus {
        Focus::Sidebar => "NAV",
        Focus::Content => "CONTENT",
        Focus::Details => "DETAILS",
    };
    let mut spans = vec![
        Span::styled(focus_label, crate::ui::theme::accent()),
        Span::raw("  "),
    ];
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
