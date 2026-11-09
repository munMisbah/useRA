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
    /// Selected-item details; hidden (zero-width) on narrow terminals.
    pub details: Rect,
    /// Status and shortcut bar across the bottom.
    pub footer: Rect,
}

impl Regions {
    /// Splits the frame area into the five fixed regions.
    pub fn compute(area: Rect) -> Self {
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

    let regions = Regions::compute(inner_frame);

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

    let details_inner = if regions.details_visible() {
        crate::ui::components::panel::frame_area(
            frame,
            regions.details,
            "Details",
            app.focus == Focus::Details,
        )
    } else {
        regions.details
    };
    crate::ui::pages::render(frame, app, page_area, details_inner);
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
