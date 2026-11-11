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
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-v" | "--version" | "version" => {
                println!("useRA v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-h" | "--help" | "help" => {
                print_help();
                return Ok(());
            }
            "update" | "--update" => {
                return system::updater::run();
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                eprintln!("Run 'usera --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    // ratatui::run owns terminal init, panic-safe restore, and teardown.
    ratatui::run(run_loop)
}

fn print_help() {
    println!(
        "useRA v{} — Linux User & Group Manager TUI",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    usera [OPTIONS] [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    update          Update useRA to the latest release");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Print help information");
    println!("    -v, --version   Print version information");
    println!();
    println!("For interactive management, run 'usera' (or 'sudo usera' for write operations).");
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
