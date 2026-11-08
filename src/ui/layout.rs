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
const SIDEBAR_WIDTH: u16 = 16;
/// Sidebar width in collapsed mode (terminal too narrow).
const SIDEBAR_WIDTH_COLLAPSED: u16 = 12;
/// Below this width the details panel disappears.
const DETAILS_MIN_WIDTH: u16 = 80;
/// Below this width the sidebar collapses.
const SIDEBAR_MIN_WIDTH: u16 = 60;

/// Computed frame regions handed to page renderers.
pub struct Regions {
    /// Header strip across the top.
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
            Constraint::Length(1), // header
            Constraint::Min(1),    // body
            Constraint::Length(1), // footer
        ])
        .split(area);

        let horizontal = Layout::horizontal([
            Constraint::Length(sidebar_width),
            Constraint::Min(20),
            Constraint::Percentage(if show_details { 20 } else { 0 }),
        ])
        .split(vertical[1]);

        Regions {
            header: vertical[0],
            sidebar: horizontal[0],
            content: horizontal[1],
            details: horizontal[2],
            footer: vertical[2],
        }
    }

    /// Whether the details panel is visible on this frame.
    pub fn details_visible(&self) -> bool {
        self.details.width > 0
    }
}

/// Renders the complete frame for the current app state.
pub fn render(frame: &mut Frame, app: &App) {
    let margin = crate::ui::theme::OUTER_MARGIN;
    let frame_area = frame.area();
    let inner_frame = Rect {
        x: frame_area.x + margin,
        y: frame_area.y + margin,
        width: frame_area.width.saturating_sub(margin * 2),
        height: frame_area.height.saturating_sub(margin * 2),
    };
    frame.render_widget(Block::new().style(crate::ui::theme::base()), frame_area);

    let regions = Regions::compute(inner_frame);

    crate::ui::components::header::render(frame, app, regions.header);
    let sidebar_inner = crate::ui::components::panel::frame_area(
        frame,
        regions.sidebar,
        crate::ui::components::header::APP_NAME,
        app.focus == Focus::Sidebar,
    );
    crate::ui::components::sidebar::render(frame, app, sidebar_inner);
    let content_inner = crate::ui::components::panel::frame_area(
        frame,
        regions.content,
        app.page.title(),
        app.focus == Focus::Content,
    );
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
    crate::ui::pages::render(frame, app, content_inner, details_inner);
    crate::ui::components::statusbar::render(frame, app, regions.footer);

    if app.show_help {
        crate::ui::components::modal::render_help(frame, frame.area());
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
