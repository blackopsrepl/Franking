/*! Local message store.
A durable, offline cache of messages and folders with an FTS5 full-text index
and per-folder sync state. Backends write here after fetching; the UI can then
list and search without touching the network. */

use std::collections::HashSet;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::model::MessageDocument;

/// A message persisted in the local store.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredMessage {
    pub account: String,
    pub folder: String,
    pub uid: String,
    pub uid_validity: Option<u32>,
    pub message_id: Option<String>,
    pub thread_root: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub subject: String,
    pub from_display: String,
    pub from_email: Option<String>,
    pub to_display: String,
    pub date_epoch: Option<i64>,
    pub flags: Vec<String>,
    pub size: u64,
    pub has_attachments: bool,
    pub snippet: String,
    pub body_text: String,
    pub raw: Option<Vec<u8>>,
}

/// Per-folder synchronization cursor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncState {
    pub account: String,
    pub folder: String,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
    pub highest_modseq: Option<u64>,
    pub last_synced_at: Option<String>,
}

impl StoredMessage {
    /// Project a parsed document into a storable row.
    pub fn from_document(
        account: &str,
        folder: &str,
        uid: &str,
        uid_validity: Option<u32>,
        flags: &[String],
        document: &MessageDocument,
        raw: Option<Vec<u8>>,
    ) -> Self {
        let message_id = document.thread.message_id.clone();
        let thread_root = document
            .thread
            .root_id()
            .map(str::to_string)
            .or_else(|| message_id.clone());
        let from_display = document
            .headers
            .from
            .first()
            .map(|address| address.display())
            .unwrap_or_default();
        let from_email = document
            .headers
            .from
            .first()
            .and_then(|address| address.normalized_email());
        let to_display = document
            .headers
            .recipients()
            .map(|address| address.display())
            .collect::<Vec<_>>()
            .join(", ");
        let body_text = document.search_text();

        Self {
            account: account.to_string(),
            folder: folder.to_string(),
            uid: uid.to_string(),
            uid_validity,
            message_id,
            thread_root,
            in_reply_to: document.thread.parent_id().map(str::to_string),
            references: document.thread.references.clone(),
            subject: document.subject().to_string(),
            from_display,
            from_email,
            to_display,
            date_epoch: document.headers.date.as_ref().map(|date| date.timestamp()),
            flags: flags.to_vec(),
            size: raw
                .as_ref()
                .map(|bytes| bytes.len() as u64)
                .unwrap_or(body_text.len() as u64),
            has_attachments: !document.attachments.is_empty(),
            snippet: snippet(&body_text),
            body_text,
            raw,
        }
    }
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

pub fn get_sync_state(conn: &Connection, account: &str, folder: &str) -> Result<Option<SyncState>> {
    conn.query_row(
        "SELECT account, folder, uid_validity, uid_next, highest_modseq, last_synced_at
         FROM sync_state WHERE account = ?1 AND folder = ?2",
        params![account, folder],
        |row| {
            Ok(SyncState {
                account: row.get(0)?,
                folder: row.get(1)?,
                uid_validity: row.get(2)?,
                uid_next: row.get(3)?,
                highest_modseq: row.get::<_, Option<i64>>(4)?.map(|value| value as u64),
                last_synced_at: row.get(5)?,
            })
        },
    )
    .optional()
    .context("failed to load sync state")
}

pub fn set_sync_state(conn: &Connection, state: &SyncState) -> Result<()> {
    conn.execute(
        "INSERT INTO sync_state (
             account, folder, uid_validity, uid_next, highest_modseq, last_synced_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(account, folder) DO UPDATE SET
             uid_validity = excluded.uid_validity,
             uid_next = excluded.uid_next,
             highest_modseq = excluded.highest_modseq,
             last_synced_at = datetime('now')",
        params![
            state.account,
            state.folder,
            state.uid_validity,
            state.uid_next,
            state.highest_modseq.map(|value| value as i64),
        ],
    )
    .context("failed to persist sync state")?;
    Ok(())
}

const SELECT_COLUMNS: &str = "SELECT
    account, folder, uid, uid_validity, message_id, thread_root, in_reply_to,
    refs, subject, from_display, from_email, to_display, date_epoch,
    flags, size, has_attachments, snippet, body_text, raw
    FROM messages";

fn searched_columns() -> String {
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

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    let references: String = row.get(7)?;
    let flags: String = row.get(13)?;
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
fn fts_query(query: &str) -> Option<String> {
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

fn snippet(body: &str) -> String {
    body.trim()
        .chars()
        .take(200)
        .collect::<String>()
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        count_messages, delete_message, get_message, get_sync_state, list_messages,
        search_messages, set_sync_state, thread_messages, upsert_message, StoredMessage, SyncState,
    };

    fn store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        conn
    }

    fn message(uid: &str, subject: &str, body: &str) -> StoredMessage {
        StoredMessage {
            account: "work".to_string(),
            folder: "INBOX".to_string(),
            uid: uid.to_string(),
            subject: subject.to_string(),
            from_display: "Alice <alice@example.com>".to_string(),
            to_display: "bob@example.com".to_string(),
            date_epoch: Some(1_700_000_000),
            body_text: body.to_string(),
            snippet: body.to_string(),
            ..StoredMessage::default()
        }
    }

    #[test]
    fn upsert_and_get_round_trip() {
        let conn = store();
        let mut row = message("42", "Project update", "the body");
        row.message_id = Some("m42@example.com".to_string());
        row.thread_root = Some("root@example.com".to_string());
        row.flags = vec!["Seen".to_string()];

        upsert_message(&conn, &row).unwrap();

        let loaded = get_message(&conn, "work", "INBOX", "42").unwrap().unwrap();
        assert_eq!(loaded.subject, "Project update");
        assert_eq!(loaded.message_id.as_deref(), Some("m42@example.com"));
        assert_eq!(loaded.flags, vec!["Seen".to_string()]);

        upsert_message(&conn, &row).unwrap();
        assert_eq!(count_messages(&conn, "work", "INBOX").unwrap(), 1);
    }

    #[test]
    fn search_indexes_subject_and_body() {
        let conn = store();
        upsert_message(&conn, &message("1", "Quarterly report", "revenue grew")).unwrap();
        upsert_message(&conn, &message("2", "Lunch", "sandwich")).unwrap();

        let by_subject = search_messages(&conn, Some("work"), "quarterly", 10).unwrap();
        assert_eq!(by_subject.len(), 1);
        assert_eq!(by_subject[0].uid, "1");

        let by_body = search_messages(&conn, None, "sandwich", 10).unwrap();
        assert_eq!(by_body.len(), 1);
        assert_eq!(by_body[0].uid, "2");

        assert!(search_messages(&conn, None, "missing", 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn listing_and_threading() {
        let conn = store();
        let mut child = message("2", "Re: Project", "reply");
        child.thread_root = Some("root@example.com".to_string());
        child.message_id = Some("child@example.com".to_string());
        let mut root = message("1", "Project", "start");
        root.thread_root = Some("root@example.com".to_string());
        root.message_id = Some("root@example.com".to_string());
        root.date_epoch = Some(1_600_000_000);

        upsert_message(&conn, &child).unwrap();
        upsert_message(&conn, &root).unwrap();

        let listed = list_messages(&conn, "work", "INBOX", 10, 0).unwrap();
        assert_eq!(listed[0].uid, "2", "newest first");

        let thread = thread_messages(&conn, "work", "root@example.com").unwrap();
        assert_eq!(thread.len(), 2);

        delete_message(&conn, "work", "INBOX", "2").unwrap();
        assert_eq!(count_messages(&conn, "work", "INBOX").unwrap(), 1);
        assert!(search_messages(&conn, None, "reply", 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn sync_state_round_trip() {
        let conn = store();
        assert!(get_sync_state(&conn, "work", "INBOX").unwrap().is_none());

        set_sync_state(
            &conn,
            &SyncState {
                account: "work".to_string(),
                folder: "INBOX".to_string(),
                uid_validity: Some(7),
                uid_next: Some(100),
                highest_modseq: Some(5000),
                last_synced_at: None,
            },
        )
        .unwrap();

        let loaded = get_sync_state(&conn, "work", "INBOX").unwrap().unwrap();
        assert_eq!(loaded.uid_validity, Some(7));
        assert_eq!(loaded.uid_next, Some(100));
        assert_eq!(loaded.highest_modseq, Some(5000));
        assert!(loaded.last_synced_at.is_some());
    }
}
