use std::io::{self, stdout};
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use solverforge_mail::app::App;
use solverforge_mail::event::{Event, EventHandler};
use solverforge_mail::{db, identities, import, setup, ui};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--accounts") {
        return setup::print_account_status();
    }

    let account = if args.iter().any(|arg| arg == "--setup") {
        match setup::run_wizard()? {
            Some(account) => Some(account),
            None => return Ok(()),
        }
    } else {
        parse_account_flag(&args)
    };

    // ── PGP key generation (non-interactive, exits after writing) ───
    if let Some(uid) = flag_value(&args, "--pgp-keygen") {
        return run_pgp_keygen(&uid);
    }

    // ── Contact import (non-interactive, exits after import) ────────
    if let Some(path) = parse_import_flag(&args) {
        return run_import(&path);
    }

    // ── Identity management (non-interactive, exits after command) ──
    if let Some(cmd) = parse_identity_command(&args) {
        return run_identity_cmd(cmd);
    }

    // ── Install panic hook that restores the terminal ───────────────
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    // ── Terminal setup ──────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // ── Run ─────────────────────────────────────────────────────────
    let events = EventHandler::new(Duration::from_millis(250));
    let mut app = App::new(account);
    app.init();
    let result = run(&mut terminal, &mut app, &events);

    // ── Teardown ────────────────────────────────────────────────────
    restore_terminal()?;
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &EventHandler,
) -> Result<()> {
    while app.running {
        // Render
        terminal.draw(|frame| ui::render(app, frame))?;

        // Handle next event
        match events.next()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            Event::Tick => app.tick(),
            Event::Resize(_, _) => {} // ratatui handles resize
        }

        // Run external open requests between frames while we still own the
        // terminal so the child process never inherits the alternate screen.
        if let Some(open) = app.pending_open_command.take() {
            restore_terminal()?;
            let status = std::process::Command::new(&open.program)
                .args(&open.args)
                .status();
            // Re-enter raw/alternate mode with mouse
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
            terminal.clear()?;
            match status {
                Ok(s) if s.success() => app.set_status("Opened HTML in the external viewer."),
                Ok(s) => app.set_status(&format!("Open command exited with {s}")),
                Err(e) => app.set_status(&format!("Failed to launch external viewer: {e}")),
            }
        }
    }
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let position = args.iter().position(|arg| arg == flag)?;
    args.get(position + 1).cloned()
}

fn run_pgp_keygen(uid: &str) -> Result<()> {
    use solverforge_mail::mail::pgp;

    let dir = pgp::default_keys_dir();
    let (secret, public) = pgp::generate_keypair(uid)?;
    let name: String = uid
        .split('@')
        .next()
        .unwrap_or("key")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    pgp::write_keypair(&dir, &name, &secret, &public)?;
    println!(
        "Wrote {name}.pub.asc and {name}.sec.asc to {}",
        dir.display()
    );
    Ok(())
}

fn parse_account_flag(args: &[String]) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--account" || arg == "-a" {
            return iter.next().cloned();
        }
        if let Some(val) = arg.strip_prefix("--account=") {
            return Some(val.to_string());
        }
    }
    None
}

fn parse_import_flag(args: &[String]) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--import-contacts" {
            return iter.next().cloned();
        }
        if let Some(val) = arg.strip_prefix("--import-contacts=") {
            return Some(val.to_string());
        }
    }
    None
}

enum IdentityCmd {
    List { account: Option<String> },
    Remove { id: i64 },
}

fn parse_identity_command(args: &[String]) -> Option<IdentityCmd> {
    let mut iter = args.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--list-identities" => {
                // Optional account argument (next positional, not a flag)
                let account = if iter.peek().map(|a| !a.starts_with('-')).unwrap_or(false) {
                    iter.next().cloned()
                } else {
                    None
                };
                return Some(IdentityCmd::List { account });
            }
            "--remove-identity" => {
                let id: i64 = iter.next()?.parse().ok()?;
                return Some(IdentityCmd::Remove { id });
            }
            _ => {}
        }
    }
    None
}

fn run_identity_cmd(cmd: IdentityCmd) -> Result<()> {
    let conn = db::open()?;
    match cmd {
        IdentityCmd::List { account } => {
            let rows: Vec<identities::Identity> = if let Some(ref acct) = account {
                identities::list_for_account(&conn, acct)?
            } else {
                // List all identities across all accounts.
                let mut stmt = conn.prepare(
                    "SELECT id, account, name, display_name, email, is_default
                     FROM identities ORDER BY account, is_default DESC, name, email",
                )?;
                let collected = stmt
                    .query_map([], |row| {
                        Ok(identities::Identity {
                            id: row.get(0)?,
                            account: row.get(1)?,
                            name: row.get(2)?,
                            display_name: row.get(3)?,
                            email: row.get(4)?,
                            is_default: row.get::<_, i32>(5)? != 0,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                collected
            };
            if rows.is_empty() {
                println!("No identities configured.");
            } else {
                for i in &rows {
                    let default_marker = if i.is_default { " *" } else { "" };
                    println!("[{}] {} ({}){}", i.id, i.label(), i.account, default_marker);
                }
                println!("  (* = default for account)");
            }
        }
        IdentityCmd::Remove { id } => {
            identities::delete(&conn, id)?;
            println!("Identity {id} removed.");
        }
    }
    Ok(())
}

/// Import contacts from a vCard (.vcf) or CSV file, then exit.
fn run_import(path: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    let content =
        fs::read_to_string(path).map_err(|e| anyhow::anyhow!("cannot read '{}': {}", path, e))?;

    let conn = db::open()?;

    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let result = match ext.as_str() {
        "vcf" | "vcard" => import::import_vcf(&conn, &content)?,
        "csv" => import::import_csv(&conn, &content)?,
        other => anyhow::bail!("unsupported file extension '.{}'. Use .vcf or .csv", other),
    };

    println!(
        "Import complete: {} added, {} skipped.",
        result.added, result.skipped
    );
    for err in &result.errors {
        eprintln!("  warning: {}", err);
    }

    Ok(())
}
