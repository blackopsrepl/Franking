/*! Receiving-account-specific decisions about incoming senders. */

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::mail::types::{Envelope, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Screening,
    Inbox,
    Reading,
    Receipts,
    Blocked,
}

impl Route {
    fn stored(self) -> Option<&'static str> {
        match self {
            Self::Screening => None,
            Self::Inbox => Some("inbox"),
            Self::Reading => Some("reading"),
            Self::Receipts => Some("receipts"),
            Self::Blocked => Some("blocked"),
        }
    }

    fn from_stored(value: &str) -> Self {
        match value {
            "inbox" => Self::Inbox,
            "reading" => Self::Reading,
            "receipts" => Self::Receipts,
            "blocked" => Self::Blocked,
            _ => Self::Screening,
        }
    }
}

/// Only a parsed mailbox is a stable policy key. Display names alone aren't.
pub fn sender_address(sender: &Sender) -> Option<String> {
    let value = match sender {
        Sender::Structured {
            addr: Some(addr), ..
        } => addr.clone(),
        Sender::Plain(text) => crate::contacts::parse_address_list(text)
            .into_iter()
            .next()
            .map(|(_, email)| email)?,
        _ => return None,
    };
    let value = value.trim().to_ascii_lowercase();
    (value.contains('@') && !value.contains(char::is_whitespace)).then_some(value)
}

pub fn get(conn: &Connection, account: &str, sender: &str) -> Result<Route> {
    use rusqlite::OptionalExtension;
    let route: Option<String> = conn
        .query_row(
            "SELECT route FROM sender_routes WHERE account = ?1 AND sender = ?2",
            params![account, sender.to_ascii_lowercase()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(route
        .as_deref()
        .map(Route::from_stored)
        .unwrap_or(Route::Screening))
}

pub fn set(conn: &Connection, account: &str, sender: &str, route: Route) -> Result<()> {
    let sender = sender.trim().to_ascii_lowercase();
    if account.is_empty() || !sender.contains('@') || sender.contains(char::is_whitespace) {
        anyhow::bail!("A receiving account and sender mailbox are required");
    }
    if let Some(value) = route.stored() {
        conn.execute(
            "INSERT INTO sender_routes (account, sender, route) VALUES (?1, ?2, ?3)
             ON CONFLICT(account, sender) DO UPDATE SET route = excluded.route",
            params![account, sender, value],
        )?;
    } else {
        conn.execute(
            "DELETE FROM sender_routes WHERE account = ?1 AND sender = ?2",
            params![account, sender],
        )?;
    }
    Ok(())
}

pub fn for_envelope(conn: &Connection, envelope: &Envelope) -> Result<Route> {
    let (Some(account), Some(sender)) = (
        envelope.account.as_deref(),
        sender_address(&envelope.sender),
    ) else {
        // Unknown sender mailboxes remain visible for manual handling.
        return Ok(Route::Inbox);
    };
    get(conn, account, &sender)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_are_isolated_by_receiving_account_and_reversible() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        set(&conn, "personal", "Alice@Example.org", Route::Reading).unwrap();
        set(&conn, "work", "alice@example.org", Route::Blocked).unwrap();
        assert_eq!(
            get(&conn, "personal", "alice@example.org").unwrap(),
            Route::Reading
        );
        assert_eq!(
            get(&conn, "work", "alice@example.org").unwrap(),
            Route::Blocked
        );
        set(&conn, "work", "alice@example.org", Route::Screening).unwrap();
        assert_eq!(
            get(&conn, "work", "alice@example.org").unwrap(),
            Route::Screening
        );
        assert_eq!(
            get(&conn, "personal", "alice@example.org").unwrap(),
            Route::Reading
        );
        crate::db::init_for_test(&conn).unwrap();
        assert_eq!(
            get(&conn, "personal", "alice@example.org").unwrap(),
            Route::Reading
        );
    }

    #[test]
    fn display_name_is_not_a_sender_address() {
        assert_eq!(sender_address(&Sender::Plain("Just a name".into())), None);
        assert_eq!(
            sender_address(&Sender::Plain("Alice <Alice@Example.org>".into())),
            Some("alice@example.org".into())
        );
    }
}
