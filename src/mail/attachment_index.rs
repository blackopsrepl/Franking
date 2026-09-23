/*! A cross-account index of attachments in locally cached mail.
Only messages whose raw bytes were cached can contribute, so the library shows
what the local store holds, not the whole server. */

use anyhow::{Context, Result};
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedAttachment {
    pub account: String,
    pub folder: String,
    pub uid: String,
    pub subject: String,
    pub sender: String,
    pub file_name: String,
    pub content_type: String,
    pub size: usize,
}

/// List attachments from the newest `limit` cached messages that carry them.
pub fn list(conn: &Connection, limit: usize) -> Result<Vec<IndexedAttachment>> {
    let mut stmt = conn.prepare(
        "SELECT account, folder, uid, subject, from_display, raw
         FROM messages
         WHERE has_attachments = 1 AND raw IS NOT NULL
         ORDER BY COALESCE(date_epoch, 0) DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Vec<u8>>(5)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (account, folder, uid, subject, sender, raw) =
            row.context("failed to read a cached message")?;
        let Ok(document) = crate::mail::mime::parse_message(&raw) else {
            continue;
        };
        for attachment in &document.attachments {
            // Inline images are signature and layout noise; skip them.
            if attachment.is_inline {
                continue;
            }
            items.push(IndexedAttachment {
                account: account.clone(),
                folder: folder.clone(),
                uid: uid.clone(),
                subject: subject.clone(),
                sender: sender.clone(),
                file_name: attachment
                    .file_name
                    .clone()
                    .unwrap_or_else(|| "attachment".to_string()),
                content_type: attachment
                    .content_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size: attachment.size,
            });
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cached_message_contributes_its_real_attachments_only() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAECAwQ=\r\n--m\r\nContent-Type: image/png; name=\"sig.png\"\r\nContent-Disposition: inline; filename=\"sig.png\"\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0=\r\n--m--\r\n";
        conn.execute(
            "INSERT INTO messages (account, folder, uid, message_id, subject, from_display, has_attachments, raw)
             VALUES ('work', 'INBOX', '5', 'five@x', 'Report', 'Alice', 1, ?1)",
            rusqlite::params![raw.to_vec()],
        )
        .unwrap();
        let items = list(&conn, 50).unwrap();
        assert_eq!(items.len(), 1, "inline signature image is skipped");
        assert_eq!(items[0].file_name, "report.pdf");
        assert_eq!(items[0].subject, "Report");
        assert_eq!(items[0].account, "work");
    }
}
