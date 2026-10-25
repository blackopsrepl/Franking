/*! Decode RFC 2047 encoded-words already stored in the message cache.

Earlier builds cached IMAP ENVELOPE subjects and sender names verbatim, so
cached rows read `=?UTF-8?Q?...?=` until each message was listed again. Decode
them in place with the same rules the listing now uses. */

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::mail::header_text;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    let rows: Vec<(i64, String, String)> = conn
        .prepare(
            "SELECT id, subject, from_display FROM messages
             WHERE subject LIKE '%=?%' OR from_display LIKE '%=?%'",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut update =
        conn.prepare("UPDATE messages SET subject = ?2, from_display = ?3 WHERE id = ?1")?;
    for (id, subject, from) in rows {
        update.execute(params![
            id,
            header_text::decode(subject.as_bytes()),
            header_text::decode(from.as_bytes())
        ])?;
    }
    Ok(())
}
