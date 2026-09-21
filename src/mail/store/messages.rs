/*! Message row persistence and lookup. */

use std::collections::HashSet;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::model::StoredMessage;
use super::queries::{fts_query, row_to_message, searched_columns, SELECT_COLUMNS};

pub fn upsert_envelope(conn: &Connection, message: &StoredMessage) -> Result<()> {
    conn.execute(
        "INSERT INTO messages (
             account, folder, uid, subject, from_display, from_email,
             to_display, date_epoch, flags, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
         ON CONFLICT(account, folder, uid) DO UPDATE SET
             subject = excluded.subject,
             from_display = excluded.from_display,
             from_email = excluded.from_email,
             to_display = excluded.to_display,
             date_epoch = excluded.date_epoch,
             flags = excluded.flags,
             updated_at = datetime('now')",
        params![
            &message.account,
            &message.folder,
            &message.uid,
            &message.subject,
            &message.from_display,
            &message.from_email,
            &message.to_display,
            message.date_epoch,
            message.flags.join(" "),
        ],
    )
    .context("failed to upsert envelope")?;
    Ok(())
}

/// Insert or update one message, keeping the FTS index in sync via triggers.
pub fn upsert_message(conn: &Connection, message: &StoredMessage) -> Result<()> {
    conn.execute(
        "INSERT INTO messages (
             account, folder, uid, uid_validity, message_id, thread_root,
             in_reply_to, refs, subject, from_display, from_email,
             to_display, date_epoch, flags, size, has_attachments,
             snippet, body_text, raw, updated_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
             ?15, ?16, ?17, ?18, ?19, datetime('now')
         )
         ON CONFLICT(account, folder, uid) DO UPDATE SET
             uid_validity = excluded.uid_validity,
             message_id = excluded.message_id,
             thread_root = excluded.thread_root,
             in_reply_to = excluded.in_reply_to,
             refs = excluded.refs,
             subject = excluded.subject,
             from_display = excluded.from_display,
             from_email = excluded.from_email,
             to_display = excluded.to_display,
             date_epoch = excluded.date_epoch,
             flags = excluded.flags,
             size = excluded.size,
             has_attachments = excluded.has_attachments,
             snippet = excluded.snippet,
             body_text = excluded.body_text,
             raw = excluded.raw,
             updated_at = datetime('now')",
        params![
            message.account,
            message.folder,
            message.uid,
            message.uid_validity,
            message.message_id,
            message.thread_root,
            message.in_reply_to,
            message.references.join(" "),
            message.subject,
            message.from_display,
            message.from_email,
            message.to_display,
            message.date_epoch,
            message.flags.join(" "),
            message.size as i64,
            message.has_attachments as i64,
            message.snippet,
            message.body_text,
            message.raw,
        ],
    )
    .context("failed to upsert message")?;
    Ok(())
}

pub fn get_message(
    conn: &Connection,
    account: &str,
    folder: &str,
    uid: &str,
) -> Result<Option<StoredMessage>> {
    conn.query_row(
        &format!("{SELECT_COLUMNS} WHERE account = ?1 AND folder = ?2 AND uid = ?3"),
        params![account, folder, uid],
        row_to_message,
    )
    .optional()
    .context("failed to load message")
}

/// Most recent messages in a folder, newest first.
pub fn list_messages(
    conn: &Connection,
    account: &str,
    folder: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<StoredMessage>> {
    let mut statement = conn
        .prepare(&format!(
            "{SELECT_COLUMNS} WHERE account = ?1 AND folder = ?2
             ORDER BY date_epoch DESC, id DESC LIMIT ?3 OFFSET ?4"
        ))
        .context("failed to prepare folder listing")?;
    let rows = statement
        .query_map(
            params![account, folder, limit as i64, offset as i64],
            row_to_message,
        )
        .context("failed to list messages")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read messages")
}

/// Full-text search across the local store, newest first.
pub fn search_messages(
    conn: &Connection,
    account: Option<&str>,
    query: &str,
    limit: usize,
) -> Result<Vec<StoredMessage>> {
    let Some(match_query) = fts_query(query) else {
        return Ok(Vec::new());
    };

    let sql = match account {
        Some(_) => format!(
            "SELECT {} FROM messages_fts f JOIN messages m ON m.id = f.rowid
             WHERE messages_fts MATCH ?1 AND m.account = ?2
             ORDER BY m.date_epoch DESC, m.id DESC LIMIT ?3",
            searched_columns()
        ),
        None => format!(
            "SELECT {} FROM messages_fts f JOIN messages m ON m.id = f.rowid
             WHERE messages_fts MATCH ?1
             ORDER BY m.date_epoch DESC, m.id DESC LIMIT ?2",
            searched_columns()
        ),
    };
    let mut statement = conn.prepare(&sql).context("failed to prepare search")?;

    let rows = match account {
        Some(account) => {
            statement.query_map(params![match_query, account, limit as i64], row_to_message)
        }
        None => statement.query_map(params![match_query, limit as i64], row_to_message),
    }
    .context("failed to run search")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read search results")
}

/// All locally stored members of a thread, newest first.
pub fn thread_messages(
    conn: &Connection,
    account: &str,
    thread_root: &str,
) -> Result<Vec<StoredMessage>> {
    let mut statement = conn
        .prepare(&format!(
            "{SELECT_COLUMNS} WHERE account = ?1 AND (thread_root = ?2 OR message_id = ?2)
             ORDER BY date_epoch ASC, id ASC"
        ))
        .context("failed to prepare thread query")?;
    let rows = statement
        .query_map(params![account, thread_root], row_to_message)
        .context("failed to load thread")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read thread")
}

pub fn count_messages(conn: &Connection, account: &str, folder: &str) -> Result<usize> {
    let count = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE account = ?1 AND folder = ?2",
            params![account, folder],
            |row| row.get::<_, i64>(0),
        )
        .context("failed to count messages")?;
    Ok(count as usize)
}

pub fn delete_message(conn: &Connection, account: &str, folder: &str, uid: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM messages WHERE account = ?1 AND folder = ?2 AND uid = ?3",
        params![account, folder, uid],
    )
    .context("failed to delete cached message")?;
    Ok(())
}

/// Drop cached messages in a folder that no longer exist on the server.
pub fn retain_uids(conn: &Connection, account: &str, folder: &str, keep: &[String]) -> Result<()> {
    let existing = {
        let mut statement = conn
            .prepare("SELECT uid FROM messages WHERE account = ?1 AND folder = ?2")
            .context("failed to prepare uid scan")?;
        let rows = statement
            .query_map(params![account, folder], |row| row.get::<_, String>(0))
            .context("failed to scan cached uids")?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read cached uids")?
    };

    let keep: HashSet<&str> = keep.iter().map(String::as_str).collect();
    for uid in existing {
        if !keep.contains(uid.as_str()) {
            delete_message(conn, account, folder, &uid)?;
        }
    }
    Ok(())
}

/// Add or remove one flag on a cached message.
pub fn set_flag(
    conn: &Connection,
    account: &str,
    folder: &str,
    uid: &str,
    flag: &str,
    present: bool,
) -> Result<()> {
    let Some(message) = get_message(conn, account, folder, uid)? else {
        return Ok(());
    };

    let mut flags = message.flags;
    flags.retain(|existing| !existing.eq_ignore_ascii_case(flag));
    if present {
        flags.push(flag.to_string());
    }

    conn.execute(
        "UPDATE messages SET flags = ?4, updated_at = datetime('now')
         WHERE account = ?1 AND folder = ?2 AND uid = ?3",
        params![account, folder, uid, flags.join(" ")],
    )
    .context("failed to update cached flags")?;
    Ok(())
}

/// Move a cached message to another folder.
pub fn move_message(
    conn: &Connection,
    account: &str,
    folder: &str,
    uid: &str,
    target: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE messages SET folder = ?4, updated_at = datetime('now')
         WHERE account = ?1 AND folder = ?2 AND uid = ?3",
        params![account, folder, uid, target],
    )
    .context("failed to move cached message")?;
    Ok(())
}
