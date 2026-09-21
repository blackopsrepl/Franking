/*! Compose, reply, forward, and template commands. */

use anyhow::Result;

use crate::himalaya::config::{account_args, global_args};

use super::exec::{editor_command, run, run_with_stdin};

pub fn compose_command(account: Option<&str>) -> Result<String> {
    let mut args = vec![];
    args.extend(["message".to_string(), "write".to_string()]);
    args.extend(account_args(account));
    editor_command(&args)
}

/// Build the command to reply to a message.
pub fn reply_command(account: Option<&str>, folder: &str, id: &str, all: bool) -> Result<String> {
    let mut args = vec![];
    args.extend(["message".to_string(), "reply".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string()]);
    if all {
        args.push("-A".to_string());
    }
    args.push(id.to_string());
    editor_command(&args)
}

/// Build the command to forward a message.
pub fn forward_command(account: Option<&str>, folder: &str, id: &str) -> Result<String> {
    let mut args = vec![];
    args.extend(["message".to_string(), "forward".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    editor_command(&args)
}

// ── Template operations (non-interactive compose) ───────────────────
// These generate MML templates without launching $EDITOR.

/// Generate a blank compose template (headers + signature) for the account.
pub fn template_write(account: Option<&str>) -> Result<String> {
    let mut args = global_args();
    args.extend(["template".to_string(), "write".to_string()]);
    args.extend(account_args(account));
    run(&args)
}

/// Generate a reply template for the given message.
pub fn template_reply(account: Option<&str>, folder: &str, id: &str, all: bool) -> Result<String> {
    let mut args = global_args();
    args.extend(["template".to_string(), "reply".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string()]);
    if all {
        args.push("-A".to_string());
    }
    args.push(id.to_string());
    run(&args)
}

/// Generate a forward template for the given message.
pub fn template_forward(account: Option<&str>, folder: &str, id: &str) -> Result<String> {
    let mut args = global_args();
    args.extend(["template".to_string(), "forward".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    run(&args)
}

/// Send a compiled MML template via stdin.
///
/// himalaya's `template send` checks `io::stdin().is_terminal()`: when
/// invoked as a subprocess (non-interactive), it reads the template from
/// stdin and ignores positional args. We pipe the template directly.
pub fn template_send(account: Option<&str>, template: &str) -> Result<String> {
    let mut args = global_args();
    args.extend(["template".to_string(), "send".to_string()]);
    args.extend(account_args(account));
    run_with_stdin(&args, template)
}

// ── Command builder (for testing) ───────────────────────────────────
