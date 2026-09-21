/*! Local outbox: messages that failed to send and wait for a retry. */

use anyhow::{Context, Result};
use rusqlite::Connection;

/// A queued outgoing message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxItem {
    pub id: i64,
    pub account: Option<String>,
    /// Subject line extracted for display, if any.
    pub subject: String,
    /// Whether the message should be signed / encrypted when sent.
    pub sign: bool,
    pub encrypt: bool,
    /// RFC 3339 time before which the message should not be sent.
    pub send_after: Option<String>,
    pub created_at: String,
    pub template: String,
}

/// Queue a message for a later send attempt.
///
/// An identical message for the same account is not queued twice, so a
/// repeated failure of the same compose session cannot pile up copies.
pub fn enqueue(
    conn: &Connection,
    account: Option<&str>,
    template: &str,
    sign: bool,
    encrypt: bool,
    send_after: Option<&str>,
) -> Result<Option<i64>> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM outbox
             WHERE template = ?1 AND COALESCE(account, '') = COALESCE(?2, '')",
            rusqlite::params![template, account],
            |row| row.get(0),
        )
        .ok();
    if existing.is_some() {
        return Ok(existing);
    }
    conn.execute(
        "INSERT INTO outbox (account, template, sign, encrypt, send_after)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![account, template, sign as i32, encrypt as i32, send_after],
    )
    .context("cannot queue the outgoing message")?;
    Ok(Some(conn.last_insert_rowid()))
}

/// Queued messages, oldest first.
pub fn list(conn: &Connection) -> Result<Vec<OutboxItem>> {
    let mut statement = conn.prepare(
        "SELECT id, account, template, sign, encrypt, created_at, send_after
         FROM outbox ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        let template: String = row.get(2)?;
        Ok(OutboxItem {
            id: row.get(0)?,
            account: row.get(1)?,
            subject: subject_of(&template),
            sign: row.get::<_, i32>(3)? != 0,
            encrypt: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            send_after: row.get(6)?,
            template,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("cannot list the outbox")
}

/// Remove one queued message.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM outbox WHERE id = ?1", [id])
        .context("cannot delete the queued message")?;
    Ok(())
}

/// Queued messages whose scheduled time has arrived.
pub fn due(conn: &Connection, now: &str) -> Result<Vec<OutboxItem>> {
    Ok(list(conn)?
        .into_iter()
        .filter(|item| match item.send_after.as_deref() {
            Some(after) => after <= now,
            None => true,
        })
        .collect())
}

/// Number of queued messages.
pub fn count(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM outbox", [], |row| row.get(0))?;
    Ok(count as usize)
}

fn subject_of(template: &str) -> String {
    template
        .lines()
        .find_map(|line| {
            line.strip_prefix("Subject:")
                .map(|value| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "(no subject)".to_string())
}

#[cfg(test)]
mod tests {
    use super::{count, delete, enqueue, list};

    #[test]
    fn queued_messages_round_trip_in_order() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        let first = enqueue(
            &conn,
            Some("acct"),
            "To: a@example.com\nSubject: First\n\none",
            true,
            false,
            None,
        )
        .unwrap()
        .expect("queued");
        let second = enqueue(
            &conn,
            None,
            "To: b@example.com\nSubject: Second\n\ntwo",
            false,
            true,
            None,
        )
        .unwrap()
        .expect("queued");

        // The same message is not queued twice.
        let again = enqueue(
            &conn,
            Some("acct"),
            "To: a@example.com\nSubject: First\n\none",
            true,
            false,
            None,
        )
        .unwrap();
        assert_eq!(again, Some(first));

        let items = list(&conn).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].subject, "First");
        assert!(items[0].sign);
        assert_eq!(items[1].subject, "Second");
        assert!(items[1].encrypt);
        assert_eq!(count(&conn).unwrap(), 2);

        delete(&conn, second).unwrap();
        assert_eq!(count(&conn).unwrap(), 1);
        assert_eq!(list(&conn).unwrap()[0].subject, "First");
    }

    #[test]
    fn due_lists_only_messages_whose_time_has_come() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        enqueue(
            &conn,
            None,
            "To: a@example.com\nSubject: Now\n\nbody",
            false,
            false,
            None,
        )
        .unwrap();
        enqueue(
            &conn,
            None,
            "To: b@example.com\nSubject: Past\n\nbody",
            false,
            false,
            Some("2026-01-01T00:00:00Z"),
        )
        .unwrap();
        enqueue(
            &conn,
            None,
            "To: c@example.com\nSubject: Future\n\nbody",
            false,
            false,
            Some("2030-01-01T00:00:00Z"),
        )
        .unwrap();

        let due = super::due(&conn, "2026-06-01T00:00:00Z").unwrap();
        let subjects: Vec<&str> = due.iter().map(|item| item.subject.as_str()).collect();
        assert_eq!(subjects, vec!["Now", "Past"], "future messages wait");
    }

    #[test]
    fn messages_without_a_subject_are_labelled() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        enqueue(&conn, None, "To: a@example.com\n\nbody", false, false, None).unwrap();
        assert_eq!(list(&conn).unwrap()[0].subject, "(no subject)");
    }
}
