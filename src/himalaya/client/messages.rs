/*! Folder and message operations. */

use anyhow::{Context, Result};

use crate::himalaya::config::{account_args, global_args};
use crate::himalaya::types::*;

use super::exec::run;

pub fn list_folders(account: Option<&str>) -> Result<Vec<Folder>> {
    let mut args = global_args();
    args.extend(["folder".to_string(), "list".to_string()]);
    args.extend(account_args(account));
    let output = run(&args)?;
    let folders: Vec<Folder> =
        serde_json::from_str(&output).context("failed to parse folder list")?;
    Ok(folders)
}

// ── Envelope operations ─────────────────────────────────────────────

/// List envelopes in a folder with optional search query.
pub fn list_envelopes(
    account: Option<&str>,
    folder: &str,
    page: usize,
    page_size: usize,
    query: Option<&str>,
) -> Result<Vec<Envelope>> {
    let mut args = global_args();
    args.extend(["envelope".to_string(), "list".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        "-p".to_string(),
        page.to_string(),
        "-s".to_string(),
        page_size.to_string(),
    ]);
    if let Some(q) = query {
        // Split query into words for himalaya's query language
        for word in q.split_whitespace() {
            args.push(word.to_string());
        }
    }
    let output = run(&args)?;
    let envelopes: Vec<Envelope> =
        serde_json::from_str(&output).context("failed to parse envelope list")?;
    Ok(envelopes)
}

/// List envelopes threaded by conversation.
pub fn list_envelopes_threaded(
    account: Option<&str>,
    folder: &str,
    query: Option<&str>,
) -> Result<Vec<Envelope>> {
    let mut args = global_args();
    args.extend(["envelope".to_string(), "thread".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string()]);
    if let Some(q) = query {
        for word in q.split_whitespace() {
            args.push(word.to_string());
        }
    }
    let output = run(&args)?;
    let envelopes: Vec<Envelope> =
        serde_json::from_str(&output).context("failed to parse threaded envelope list")?;
    Ok(envelopes)
}

// ── Message operations ──────────────────────────────────────────────

/// Read a message body (plain text).
pub fn read_message(account: Option<&str>, folder: &str, id: &str) -> Result<String> {
    let mut args = global_args();
    args.extend(["message".to_string(), "read".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    run(&args)
}

/// Read a message in preview mode (don't mark as seen).
pub fn preview_message(account: Option<&str>, folder: &str, id: &str) -> Result<String> {
    let mut args = global_args();
    args.extend(["message".to_string(), "read".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-p".to_string(),
        "-f".to_string(),
        folder.to_string(),
        id.to_string(),
    ]);
    run(&args)
}

/// Delete a message (moves to trash, or flags as deleted if in trash).
pub fn delete_message(account: Option<&str>, folder: &str, id: &str) -> Result<()> {
    let mut args = global_args();
    args.extend(["message".to_string(), "delete".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    run(&args)?;
    Ok(())
}

/// Move a message to a target folder.
pub fn move_message(account: Option<&str>, folder: &str, target: &str, id: &str) -> Result<()> {
    let mut args = global_args();
    args.extend(["message".to_string(), "move".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        target.to_string(),
        id.to_string(),
    ]);
    run(&args)?;
    Ok(())
}

/// Copy a message to a target folder.
pub fn copy_message(account: Option<&str>, folder: &str, target: &str, id: &str) -> Result<()> {
    let mut args = global_args();
    args.extend(["message".to_string(), "copy".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        target.to_string(),
        id.to_string(),
    ]);
    run(&args)?;
    Ok(())
}

// ── Flag operations ─────────────────────────────────────────────────

/// Add a flag to an envelope.
pub fn flag_add(account: Option<&str>, folder: &str, id: &str, flag: &str) -> Result<()> {
    let mut args = global_args();
    args.extend(["flag".to_string(), "add".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        id.to_string(),
        flag.to_string(),
    ]);
    run(&args)?;
    Ok(())
}

/// Remove a flag from an envelope.
pub fn flag_remove(account: Option<&str>, folder: &str, id: &str, flag: &str) -> Result<()> {
    let mut args = global_args();
    args.extend(["flag".to_string(), "remove".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        id.to_string(),
        flag.to_string(),
    ]);
    run(&args)?;
    Ok(())
}

// ── Attachment operations ───────────────────────────────────────────

/// Download attachments for a message.
pub fn download_attachments(account: Option<&str>, folder: &str, id: &str) -> Result<String> {
    let mut args = global_args();
    args.extend(["attachment".to_string(), "download".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    run(&args)
}
