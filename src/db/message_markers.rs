/*! Local follow-up and reference state, independent of server flags. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::store::{self, StoredMessage};
use crate::mail::types::{Envelope, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    ReplyLater,
    Saved,
}

impl Marker {
    fn column(self) -> &'static str {
        match self {
            Self::ReplyLater => "reply_later",
            Self::Saved => "saved",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ReplyLater => "Reply later",
            Self::Saved => "Saved",
        }
    }
}

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

pub fn has(conn: &Connection, envelope: &Envelope, marker: Marker) -> Result<bool> {
    let (account, folder, uid) = source(envelope)?;
    let value: Option<(Option<String>, bool)> = conn
        .query_row(
            &format!("SELECT message_id, {} FROM message_markers WHERE account = ?1 AND folder = ?2 AND uid = ?3", marker.column()),
            params![account, folder, uid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    // A reused UID must not inherit a marker belonging to a different mail.
    Ok(value.is_some_and(|(id, value)| value && id == envelope.message_id))
}

pub fn set(conn: &Connection, envelope: &Envelope, marker: Marker, enabled: bool) -> Result<()> {
    let (account, folder, uid) = source(envelope)?;
    if enabled {
        let cached = StoredMessage::from_envelope(account, folder, envelope);
        store::upsert_envelope(conn, &cached)?;
        conn.execute(
            &format!(
                "INSERT INTO message_markers (account, folder, uid, message_id, {})
                VALUES (?1, ?2, ?3, ?4, 1)
                ON CONFLICT(account, folder, uid) DO UPDATE SET
                    message_id = excluded.message_id,
                    {} = 1,
                    {} = CASE WHEN message_markers.message_id IS excluded.message_id
                        THEN {} ELSE 0 END",
                marker.column(),
                marker.column(),
                other(marker).column(),
                other(marker).column()
            ),
            params![account, folder, uid, envelope.message_id],
        )?;
    } else {
        conn.execute(
            &format!(
                "UPDATE message_markers SET {} = 0
                WHERE account = ?1 AND folder = ?2 AND uid = ?3
                AND message_id IS ?4",
                marker.column()
            ),
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

fn other(marker: Marker) -> Marker {
    match marker {
        Marker::ReplyLater => Marker::Saved,
        Marker::Saved => Marker::ReplyLater,
    }
}

pub fn list(conn: &Connection, account: Option<&str>, marker: Marker) -> Result<Vec<Envelope>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT p.account, p.folder, p.uid, p.message_id FROM message_markers p
         WHERE p.{} = 1 AND (?1 IS NULL OR p.account = ?1)
         ORDER BY p.rowid DESC",
        marker.column()
    ))?;
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
            subject: "Keep this".into(),
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
    fn markers_are_independent_and_account_scoped() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let personal = envelope("personal", "7");
        let work = envelope("work", "7");
        set(&conn, &personal, Marker::ReplyLater, true).unwrap();
        set(&conn, &personal, Marker::Saved, true).unwrap();
        set(&conn, &personal, Marker::ReplyLater, false).unwrap();
        assert!(!has(&conn, &personal, Marker::ReplyLater).unwrap());
        assert!(has(&conn, &personal, Marker::Saved).unwrap());
        assert!(!has(&conn, &work, Marker::Saved).unwrap());
        assert_eq!(
            list(&conn, Some("personal"), Marker::Saved).unwrap()[0].sender_display(),
            "Alice"
        );
        assert!(list(&conn, Some("work"), Marker::Saved).unwrap().is_empty());
    }

    #[test]
    fn a_reused_uid_does_not_inherit_another_messages_state() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let original = envelope("work", "7");
        set(&conn, &original, Marker::Saved, true).unwrap();
        let mut replacement = original.clone();
        replacement.message_id = Some("different@example.org".into());
        assert!(!has(&conn, &replacement, Marker::Saved).unwrap());
        set(&conn, &replacement, Marker::ReplyLater, true).unwrap();
        assert!(!has(&conn, &replacement, Marker::Saved).unwrap());
        assert!(has(&conn, &replacement, Marker::ReplyLater).unwrap());
    }
}
