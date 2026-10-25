/*! Reply later: a local checklist, independent of server flags. The table
also holds a retired `saved` column whose rows are left untouched. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::store::{self, StoredMessage};
use crate::mail::types::{Envelope, Sender};

fn source(envelope: &Envelope) -> Result<(&str, &str, &str)> {
    Ok((
        envelope
            .account
            .as_deref()
            .context("message has no receiving account")?,
        envelope
            .folder
            .as_deref()
            .context("message has no source folder")?,
        &envelope.id,
    ))
}

pub fn is_reply_later(conn: &Connection, envelope: &Envelope) -> Result<bool> {
    let (account, folder, uid) = source(envelope)?;
    let value: Option<(Option<String>, bool)> = conn
        .query_row(
            "SELECT message_id, reply_later FROM message_markers
             WHERE account = ?1 AND folder = ?2 AND uid = ?3",
            params![account, folder, uid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    // A reused UID must not inherit a marker belonging to a different mail.
    Ok(value.is_some_and(|(id, value)| value && id == envelope.message_id))
}

pub fn set_reply_later(conn: &Connection, envelope: &Envelope, enabled: bool) -> Result<()> {
    let (account, folder, uid) = source(envelope)?;
    if enabled {
        let cached = StoredMessage::from_envelope(account, folder, envelope);
        store::upsert_envelope(conn, &cached)?;
        conn.execute(
            "INSERT INTO message_markers (account, folder, uid, message_id, reply_later)
             VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(account, folder, uid) DO UPDATE SET
                 message_id = excluded.message_id,
                 reply_later = 1,
                 saved = CASE WHEN message_markers.message_id IS excluded.message_id
                     THEN saved ELSE 0 END",
            params![account, folder, uid, envelope.message_id],
        )?;
    } else {
        conn.execute(
            "UPDATE message_markers SET reply_later = 0
             WHERE account = ?1 AND folder = ?2 AND uid = ?3 AND message_id IS ?4",
            params![account, folder, uid, envelope.message_id],
        )?;
        conn.execute(
            "DELETE FROM message_markers WHERE account = ?1 AND folder = ?2 AND uid = ?3
             AND reply_later = 0 AND saved = 0",
            params![account, folder, uid],
        )?;
    }
    Ok(())
}

/// Reply-later messages for one account, or every account when `None`.
pub fn reply_later(conn: &Connection, account: Option<&str>) -> Result<Vec<Envelope>> {
    let mut stmt = conn.prepare(
        "SELECT account, folder, uid, message_id FROM message_markers
         WHERE reply_later = 1 AND (?1 IS NULL OR account = ?1)
         ORDER BY rowid DESC",
    )?;
    let keys = stmt
        .query_map([account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut envelopes = Vec::new();
    for (account, folder, uid, message_id) in keys {
        if let Some(stored) = store::get_message(conn, &account, &folder, &uid)? {
            if stored.message_id.is_some() && stored.message_id != message_id {
                continue;
            }
            let mut envelope = stored.to_envelope();
            envelope.message_id = message_id;
            if let Some(addr) = stored.from_email {
                envelope.sender = Sender::Structured {
                    name: Some(stored.from_display),
                    addr: Some(addr),
                };
            }
            envelopes.push(envelope);
        }
    }
    Ok(envelopes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(account: &str, uid: &str) -> Envelope {
        Envelope {
            id: uid.into(),
            flags: vec![],
            subject: "Answer this".into(),
            sender: Sender::Structured {
                name: Some("Alice".into()),
                addr: Some("alice@example.org".into()),
            },
            date: "2026-09-23T12:00:00+00:00".into(),
            message_id: Some(format!("{uid}@example.org")),
            in_reply_to: None,
            account: Some(account.into()),
            folder: Some("INBOX".into()),
        }
    }

    #[test]
    fn reply_later_is_account_scoped_and_keeps_legacy_saved_rows() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let personal = envelope("personal", "7");
        let work = envelope("work", "7");
        set_reply_later(&conn, &personal, true).unwrap();
        conn.execute("UPDATE message_markers SET saved = 1", [])
            .unwrap();
        assert!(is_reply_later(&conn, &personal).unwrap());
        assert!(!is_reply_later(&conn, &work).unwrap());
        assert_eq!(
            reply_later(&conn, Some("personal")).unwrap()[0].sender_display(),
            "Alice"
        );
        assert!(reply_later(&conn, Some("work")).unwrap().is_empty());
        set_reply_later(&conn, &personal, false).unwrap();
        assert!(!is_reply_later(&conn, &personal).unwrap());
        let legacy: i64 = conn
            .query_row(
                "SELECT count(*) FROM message_markers WHERE saved = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(legacy, 1, "retired Saved rows are not deleted");
    }

    #[test]
    fn a_reused_uid_does_not_inherit_another_messages_state() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let original = envelope("work", "7");
        set_reply_later(&conn, &original, true).unwrap();
        let mut replacement = original.clone();
        replacement.message_id = Some("different@example.org".into());
        assert!(!is_reply_later(&conn, &replacement).unwrap());
    }
}
