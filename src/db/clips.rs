/*! Saved text excerpts. A clip references its source message but deleting a
clip never touches the mail. */

use anyhow::Result;
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub id: i64,
    pub account: String,
    pub folder: String,
    pub uid: String,
    pub message_id: Option<String>,
    pub body: String,
    pub source: String,
}

/// Where a clip was taken from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipSource {
    pub account: Option<String>,
    pub folder: Option<String>,
    pub uid: String,
    pub message_id: Option<String>,
    /// Human label for the list, such as `Subject · Sender`.
    pub label: String,
}

pub fn list(conn: &Connection) -> Result<Vec<Clip>> {
    let mut stmt = conn.prepare(
        "SELECT id, account, folder, uid, message_id, body, source
         FROM clips ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Clip {
            id: row.get(0)?,
            account: row.get(1)?,
            folder: row.get(2)?,
            uid: row.get(3)?,
            message_id: row.get(4)?,
            body: row.get(5)?,
            source: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn add(conn: &Connection, source: &ClipSource, body: &str) -> Result<i64> {
    if body.trim().is_empty() {
        anyhow::bail!("A clip needs some text");
    }
    conn.execute(
        "INSERT INTO clips (account, folder, uid, message_id, body, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            source.account,
            source.folder,
            source.uid,
            source.message_id,
            body.trim(),
            source.label,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM clips WHERE id = ?1", [id])? > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> ClipSource {
        ClipSource {
            account: Some("work".into()),
            folder: Some("INBOX".into()),
            uid: "7".into(),
            message_id: Some("seven@x".into()),
            label: "Report · Alice".into(),
        }
    }

    #[test]
    fn a_clip_round_trips_and_deletes_without_touching_mail() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        conn.execute(
            "INSERT INTO messages (account, folder, uid, subject) VALUES ('work', 'INBOX', '7', 'Report')",
            [],
        )
        .unwrap();
        let id = add(&conn, &source(), "Confirmation 1234").unwrap();
        let items = list(&conn).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].body, "Confirmation 1234");
        assert_eq!(items[0].source, "Report · Alice");
        assert!(delete(&conn, id).unwrap());
        let mail: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mail, 1, "the message is untouched");
    }
}
