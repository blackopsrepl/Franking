/*! Local annotations: a private note on a message and a display-only subject
alias for a conversation. Neither touches the server or the wire format. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::types::Envelope;

fn key(envelope: &Envelope) -> Option<(&str, &str, &str)> {
    Some((
        envelope.account.as_deref()?,
        envelope.folder.as_deref()?,
        envelope.id.as_str(),
    ))
}

/// Read the note for a message, if one was written.
pub fn note(conn: &Connection, envelope: &Envelope) -> Result<Option<String>> {
    let Some((account, folder, uid)) = key(envelope) else {
        return Ok(None);
    };
    let value: Option<(Option<String>, String)> = conn
        .query_row(
            "SELECT message_id, body FROM message_notes
             WHERE account = ?1 AND folder = ?2 AND uid = ?3",
            params![account, folder, uid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(value.and_then(|(message_id, body)| (message_id == envelope.message_id).then_some(body)))
}

/// Write a note, or clear it when the text is blank.
pub fn set_note(conn: &Connection, envelope: &Envelope, body: &str) -> Result<()> {
    let Some((account, folder, uid)) = key(envelope) else {
        anyhow::bail!("A note needs a receiving account and folder");
    };
    if body.trim().is_empty() {
        conn.execute(
            "DELETE FROM message_notes
             WHERE account = ?1 AND folder = ?2 AND uid = ?3 AND message_id IS ?4",
            params![account, folder, uid, envelope.message_id],
        )?;
        return Ok(());
    }
    let cached = crate::mail::store::StoredMessage::from_envelope(account, folder, envelope);
    crate::mail::store::upsert_envelope(conn, &cached)?;
    conn.execute(
        "INSERT INTO message_notes (account, folder, uid, message_id, body, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(account, folder, uid) DO UPDATE SET
             message_id = excluded.message_id,
             body = excluded.body,
             updated_at = datetime('now')",
        params![account, folder, uid, envelope.message_id, body.trim()],
    )?;
    Ok(())
}

/// Read the display alias for a conversation anchor, if any.
pub fn alias(conn: &Connection, account: &str, anchors: &[String]) -> Result<Option<String>> {
    for anchor in anchors {
        let value: Option<String> = conn
            .query_row(
                "SELECT alias FROM subject_aliases WHERE account = ?1 AND anchor = ?2",
                params![account, anchor],
                |row| row.get(0),
            )
            .optional()?;
        if value.is_some() {
            return Ok(value);
        }
    }
    Ok(None)
}

/// Set or clear a display alias for every anchor of a conversation.
pub fn set_alias(conn: &Connection, account: &str, anchors: &[String], alias: &str) -> Result<()> {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        for anchor in anchors {
            conn.execute(
                "DELETE FROM subject_aliases WHERE account = ?1 AND anchor = ?2",
                params![account, anchor],
            )?;
        }
        return Ok(());
    }
    for anchor in anchors {
        conn.execute(
            "INSERT INTO subject_aliases (account, anchor, alias, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(account, anchor) DO UPDATE SET
                 alias = excluded.alias,
                 updated_at = datetime('now')",
            params![account, anchor, trimmed],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::types::Sender;

    fn envelope(uid: &str, message_id: &str) -> Envelope {
        Envelope {
            id: uid.into(),
            flags: Vec::new(),
            subject: "original".into(),
            sender: Sender::Plain("a@example.org".into()),
            date: "2026-09-23".into(),
            message_id: Some(message_id.into()),
            in_reply_to: None,
            account: Some("work".into()),
            folder: Some("INBOX".into()),
        }
    }

    #[test]
    fn a_note_round_trips_and_blank_clears_it() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let mail = envelope("7", "seven@x");
        assert_eq!(note(&conn, &mail).unwrap(), None);
        set_note(&conn, &mail, "call back Tuesday").unwrap();
        assert_eq!(
            note(&conn, &mail).unwrap().as_deref(),
            Some("call back Tuesday")
        );
        set_note(&conn, &mail, "  ").unwrap();
        assert_eq!(note(&conn, &mail).unwrap(), None);
    }

    #[test]
    fn an_alias_applies_to_a_conversation_anchor() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let anchors = vec!["root@x".to_string(), "reply@x".to_string()];
        set_alias(&conn, "work", &anchors, "Q3 pricing").unwrap();
        assert_eq!(
            alias(&conn, "work", &["reply@x".to_string()])
                .unwrap()
                .as_deref(),
            Some("Q3 pricing")
        );
        assert_eq!(alias(&conn, "personal", &anchors).unwrap(), None);
        set_alias(&conn, "work", &anchors, "").unwrap();
        assert_eq!(alias(&conn, "work", &anchors).unwrap(), None);
    }
}
