//! useRA — Linux User & Group Manager (TUI).
//!
//! Binary entrypoint: terminal setup/teardown and the main event loop.

mod app;
mod errors;
mod events;
mod models;
mod services;
mod system;
mod ui;

use app::App;
use ratatui::crossterm::event;
use std::io;

fn main() -> io::Result<()> {
    // ratatui::run owns terminal init, panic-safe restore, and teardown.
    ratatui::run(run_loop)
}

/// Main loop: draw a frame, then block for the next key event.
fn run_loop(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let mut app = App::default();
    app.reload();
    while !app.should_quit {
        terminal.draw(|frame| ui::layout::render(frame, &mut app))?;
        if let Some(key) = next_key()? {
            events::handler::handle_key(&mut app, key);
        }
    }
    Ok(())
}

/// Returns the next key press, or None for resize/other events.
fn next_key() -> io::Result<Option<event::KeyEvent>> {
    Ok(match event::read()? {
        event::Event::Key(key) if key.kind == event::KeyEventKind::Press => Some(key),
        _ => None,
    })
}
