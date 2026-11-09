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
                return run_update();
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

fn run_update() -> io::Result<()> {
    println!(
        "\x1b[1;35museRA\x1b[0m Checking for updates (current: v{})...",
        env!("CARGO_PKG_VERSION")
    );

    let script_url = "https://raw.githubusercontent.com/munMisbah/useRA/main/install.sh";

    let (fetch_cmd, fetch_args) = if command_exists("curl") {
        ("curl", vec!["-fsSL", script_url])
    } else if command_exists("wget") {
        ("wget", vec!["-qO-", script_url])
    } else {
        eprintln!("\x1b[1;31museRA update failed:\x1b[0m curl or wget is required to update.");
        std::process::exit(1);
    };

    let fetch_output = std::process::Command::new(fetch_cmd)
        .args(&fetch_args)
        .output()?;

    if !fetch_output.status.success() {
        let err = String::from_utf8_lossy(&fetch_output.stderr);
        eprintln!("\x1b[1;31museRA update failed:\x1b[0m could not fetch installer:\n{err}");
        std::process::exit(1);
    }

    let script = String::from_utf8_lossy(&fetch_output.stdout);
    let mut child = std::process::Command::new("bash")
        .arg("-s")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(script.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let p = std::path::Path::new(dir).join(cmd);
            if p.is_file() {
                return true;
            }
        }
    }
    false
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
