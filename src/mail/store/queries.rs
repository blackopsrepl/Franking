/*! Shared SQL fragments, row mapping, and FTS query construction. */

use super::model::StoredMessage;

pub(super) const SELECT_COLUMNS: &str = "SELECT
    account, folder, uid, uid_validity, message_id, thread_root, in_reply_to,
    refs, subject, from_display, from_email, to_display, date_epoch,
    flags, size, has_attachments, snippet, body_text, raw
    FROM messages";

pub(super) fn searched_columns() -> String {
    [
        "account",
        "folder",
        "uid",
        "uid_validity",
        "message_id",
        "thread_root",
        "in_reply_to",
        "refs",
        "subject",
        "from_display",
        "from_email",
        "to_display",
        "date_epoch",
        "flags",
        "size",
        "has_attachments",
        "snippet",
        "body_text",
        "raw",
    ]
    .iter()
    .map(|column| format!("m.{column}"))
    .collect::<Vec<_>>()
    .join(", ")
}

pub(super) fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    let references: String = row.get::<_, Option<String>>(7)?.unwrap_or_default();
    let flags: String = row.get::<_, Option<String>>(13)?.unwrap_or_default();
    Ok(StoredMessage {
        account: row.get(0)?,
        folder: row.get(1)?,
        uid: row.get(2)?,
        uid_validity: row.get(3)?,
        message_id: row.get(4)?,
        thread_root: row.get(5)?,
        in_reply_to: row.get(6)?,
        references: references.split_whitespace().map(str::to_string).collect(),
        subject: row.get(8)?,
        from_display: row.get(9)?,
        from_email: row.get(10)?,
        to_display: row.get(11)?,
        date_epoch: row.get(12)?,
        flags: flags.split_whitespace().map(str::to_string).collect(),
        size: row.get::<_, i64>(14)?.max(0) as u64,
        has_attachments: row.get::<_, i64>(15)? != 0,
        snippet: row.get(16)?,
        body_text: row.get(17)?,
        raw: row.get(18)?,
    })
}

/// Build an FTS5 MATCH expression that treats each whitespace term as a
/// literal phrase, avoiding FTS operator injection and syntax errors.
pub(super) fn fts_query(query: &str) -> Option<String> {
    let terms = query
        .split_whitespace()
        .map(|term| term.replace('"', "\"\""))
        .map(|term| format!("\"{term}\""))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}
