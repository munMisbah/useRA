//! Frame layout: header, sidebar, content, details, footer.
//!
//! Implements the approved split-panel design from `docs/UI_DESIGN.md`
//! section 2 with the responsive behavior of section 7:
//! - width < 80: details panel hidden.
//! - width < 60: sidebar collapsed to a narrow strip.

use crate::app::{App, Focus};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Styled;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

/// Sidebar width in normal mode.
const SIDEBAR_WIDTH: u16 = 20;
/// Sidebar width in collapsed mode (terminal too narrow).
const SIDEBAR_WIDTH_COLLAPSED: u16 = 12;
/// Below this width the details panel disappears.
const DETAILS_MIN_WIDTH: u16 = 80;
/// Below this width the sidebar collapses.
const SIDEBAR_MIN_WIDTH: u16 = 60;

/// Computed frame regions handed to page renderers.
pub struct Regions {
    /// Header strip across the top (reserved for overlays/legacy).
    #[allow(dead_code)]
    pub header: Rect,
    /// Navigation sidebar.
    pub sidebar: Rect,
    /// Active page content.
    pub content: Rect,
    /// Selected-item details (or Hybrid Resource panel on wide Users/Groups); zero-width on narrow terminals.
    pub details: Rect,
    /// Bottom details strip in Hybrid mode; zero-height in standard mode.
    pub bottom_details: Rect,
    /// Status and shortcut bar across the bottom.
    pub footer: Rect,
}

impl Regions {
    /// Splits the frame area into standard regions.
    #[allow(dead_code)]
    pub fn compute(area: Rect) -> Self {
        Self::compute_for(area, false)
    }

    /// Splits the frame area into standard 3-column or Hybrid layout.
    pub fn compute_for(area: Rect, hybrid: bool) -> Self {
        let show_details = area.width >= DETAILS_MIN_WIDTH;
        let sidebar_width = if area.width >= SIDEBAR_MIN_WIDTH {
            SIDEBAR_WIDTH
        } else {
            SIDEBAR_WIDTH_COLLAPSED
        };

        let vertical = Layout::vertical([
            Constraint::Min(1),    // body
            Constraint::Length(1), // footer
        ])
        .split(area);

        if hybrid {
            let body_cols =
                Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(30)])
                    .spacing(1)
                    .split(vertical[0]);

            let right_rows =
                Layout::vertical([Constraint::Min(8), Constraint::Length(6)]).split(body_cols[1]);

            let top_cols =
                Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)])
                    .spacing(1)
                    .split(right_rows[0]);

            return Regions {
                header: Rect::default(),
                sidebar: body_cols[0],
                content: top_cols[0],
                details: top_cols[1],
                bottom_details: right_rows[1],
                footer: vertical[1],
            };
        }

        let horizontal = Layout::horizontal([
            Constraint::Length(sidebar_width),
            Constraint::Min(20),
            Constraint::Percentage(if show_details { 25 } else { 0 }),
        ])
        .spacing(1)
        .split(vertical[0]);

        Regions {
            header: Rect::default(),
            sidebar: horizontal[0],
            content: horizontal[1],
            details: horizontal[2],
            bottom_details: Rect::default(),
            footer: vertical[1],
        }
    }
    /// Whether the details panel is visible on this frame.
    pub fn details_visible(&self) -> bool {
        self.details.width > 0
    }
}

/// Renders the complete frame for the current app state.
pub fn render(frame: &mut Frame, app: &mut App) {
    let margin_x = crate::ui::theme::MARGIN_X;
    let margin_y = crate::ui::theme::MARGIN_Y;
    let frame_area = frame.area();
    let inner_frame = Rect {
        x: frame_area.x + margin_x,
        y: frame_area.y + margin_y,
        width: frame_area.width.saturating_sub(margin_x * 2),
        height: frame_area.height.saturating_sub(margin_y * 2),
    };
    frame.render_widget(Block::new().style(crate::ui::theme::base()), frame_area);

    let hybrid = app.has_hybrid_resources(inner_frame.width, inner_frame.height);
    app.hybrid_active = hybrid;
    let regions = Regions::compute_for(inner_frame, hybrid);

    // Sidebar without border
    crate::ui::components::sidebar::render(frame, app, regions.sidebar);

    // Center Content Panel with title
    let content_inner = crate::ui::components::panel::frame_area(
        frame,
        regions.content,
        app.page.title(),
        app.focus == Focus::Content,
    );

    let page_area = if app.is_searchable_page() {
        // Search bar at top of center panel on list pages
        let content_chunks = Layout::vertical([
            Constraint::Length(1), // 🔍 (/) Type something
            Constraint::Min(1),    // page content
        ])
        .split(content_inner);

        crate::ui::components::search_bar::render(frame, content_chunks[0], app);
        content_chunks[1]
    } else {
        content_inner
    };

    if hybrid {
        crate::ui::pages::render(frame, app, page_area, Rect::default());
        let right_title = match app.page {
            crate::app::Page::Users => app
                .selected_user()
                .map(|u| format!("Resources (ACL: {})", u.name))
                .unwrap_or_else(|| "Resources".to_string()),
            crate::app::Page::Groups => app
                .selected_group()
                .map(|g| format!("Resources (ACL: {})", g.name))
                .unwrap_or_else(|| "Resources".to_string()),
            _ => "Resources".to_string(),
        };
        let right_inner = crate::ui::components::panel::frame_area(
            frame,
            regions.details,
            &right_title,
            app.focus == Focus::Details,
        );
        crate::ui::pages::resources::render_hybrid_right(frame, app, right_inner);

        let bottom_inner = crate::ui::components::panel::frame_area(
            frame,
            regions.bottom_details,
            "Details",
            false,
        );
        crate::ui::pages::resources::render_hybrid_bottom_details(frame, app, bottom_inner);
    } else {
        let details_title = if app.page == crate::app::Page::Resources {
            "Details & ACL"
        } else {
            "Details"
        };
        let details_inner = if regions.details_visible() {
            crate::ui::components::panel::frame_area(
                frame,
                regions.details,
                details_title,
                app.focus == Focus::Details,
            )
        } else {
            regions.details
        };
        crate::ui::pages::render(frame, app, page_area, details_inner);
    }
    crate::ui::components::statusbar::render(frame, app, regions.footer);

    if app.show_help {
        crate::ui::components::modal::render_help(frame, inner_frame);
    }
    if app.modal.is_open() {
        crate::ui::components::form_modal::render(frame, &app.modal, inner_frame);
    }
}

/// Draws a titled border around `area` and returns the inner content area.
pub fn draw_border(frame: &mut Frame, area: Rect, title: &str, focused: bool) -> Rect {
    let block = Block::new()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .borders(Borders::ALL)
        .title(format!(" {title} ").set_style(crate::ui::theme::title()))
        .padding(ratatui::widgets::Padding::horizontal(1))
        .border_style(if focused {
            crate::ui::theme::border_focused()
        } else {
            crate::ui::theme::border()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn hybrid_layout_activates_on_wide_users_and_collapses_on_narrow() {
        let mut app = App::default();
        app.page = crate::app::Page::Users;
        app.reload();

        let mut wide = Terminal::new(TestBackend::new(120, 32)).unwrap();
        wide.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(app.hybrid_active);

        let mut narrow = Terminal::new(TestBackend::new(80, 24)).unwrap();
        narrow.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(!app.hybrid_active);
    }

    #[test]
    fn resources_page_renders_cleanly() {
        let mut app = App::default();
        app.page = crate::app::Page::Resources;
        app.reload();

        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(!app.hybrid_active);
    }
}
