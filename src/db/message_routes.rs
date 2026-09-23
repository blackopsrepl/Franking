/*! A per-message placement that overrides the sender's route without changing
future delivery. Keyed by account, folder, and UID, guarded by Message-ID so a
reused UID cannot inherit another message's placement. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::types::Envelope;

use super::sender_routes::Route;

fn stored(route: Route) -> &'static str {
    match route {
        Route::Inbox => "inbox",
        Route::Reading => "reading",
        Route::Receipts => "receipts",
        Route::Blocked => "blocked",
        Route::Screening => "screening",
    }
}

fn parse(value: &str) -> Option<Route> {
    match value {
        "inbox" => Some(Route::Inbox),
        "reading" => Some(Route::Reading),
        "receipts" => Some(Route::Receipts),
        "blocked" => Some(Route::Blocked),
        "screening" => Some(Route::Screening),
        _ => None,
    }
}

/// Account placements keyed by `(folder, uid)`, with the Message-ID guard.
pub type PlacementMap = std::collections::HashMap<(String, String), (Option<String>, Route)>;

/// Every placement for one account, keyed by `(folder, uid)` with the
/// Message-ID guard. Loading them once avoids a query per listed row.
pub fn overrides_for_account(conn: &Connection, account: &str) -> Result<PlacementMap> {
    let mut stmt = conn
        .prepare("SELECT folder, uid, message_id, route FROM message_routes WHERE account = ?1")?;
    let rows = stmt.query_map([account], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut routes = std::collections::HashMap::new();
    for row in rows {
        let (folder, uid, message_id, route) = row?;
        if let Some(route) = parse(&route) {
            routes.insert((folder, uid), (message_id, route));
        }
    }
    Ok(routes)
}

/// The stored override for an envelope, if any.
pub fn get(conn: &Connection, envelope: &Envelope) -> Result<Option<Route>> {
    let (Some(account), Some(folder)) = (envelope.account.as_deref(), envelope.folder.as_deref())
    else {
        return Ok(None);
    };
    let value: Option<(Option<String>, String)> = conn
        .query_row(
            "SELECT message_id, route FROM message_routes
             WHERE account = ?1 AND folder = ?2 AND uid = ?3",
            params![account, folder, envelope.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(value.and_then(|(message_id, route)| {
        (message_id == envelope.message_id)
            .then(|| parse(&route))
            .flatten()
    }))
}

/// Set or clear the override for one message.
pub fn set(conn: &Connection, envelope: &Envelope, route: Option<Route>) -> Result<()> {
    let (Some(account), Some(folder)) = (envelope.account.as_deref(), envelope.folder.as_deref())
    else {
        anyhow::bail!("A message placement needs a receiving account and folder");
    };
    match route {
        Some(route) => {
            let cached =
                crate::mail::store::StoredMessage::from_envelope(account, folder, envelope);
            crate::mail::store::upsert_envelope(conn, &cached)?;
            conn.execute(
                "INSERT INTO message_routes (account, folder, uid, message_id, route)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(account, folder, uid) DO UPDATE SET
                     message_id = excluded.message_id,
                     route = excluded.route",
                params![
                    account,
                    folder,
                    envelope.id,
                    envelope.message_id,
                    stored(route)
                ],
            )
            .context("failed to store the message placement")?;
        }
        None => {
            conn.execute(
                "DELETE FROM message_routes
                 WHERE account = ?1 AND folder = ?2 AND uid = ?3 AND message_id IS ?4",
                params![account, folder, envelope.id, envelope.message_id],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::types::Sender;

    fn envelope(uid: &str, message_id: Option<&str>) -> Envelope {
        Envelope {
            id: uid.into(),
            flags: Vec::new(),
            subject: "s".into(),
            sender: Sender::Plain("a@example.org".into()),
            date: "2026-09-23".into(),
            message_id: message_id.map(str::to_string),
            in_reply_to: None,
            account: Some("work".into()),
            folder: Some("INBOX".into()),
        }
    }

    #[test]
    fn a_placement_overrides_only_its_own_message() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let first = envelope("7", Some("seven@x"));
        let second = envelope("8", Some("eight@x"));
        set(&conn, &first, Some(Route::Reading)).unwrap();
        assert_eq!(get(&conn, &first).unwrap(), Some(Route::Reading));
        assert_eq!(get(&conn, &second).unwrap(), None);
        set(&conn, &first, None).unwrap();
        assert_eq!(get(&conn, &first).unwrap(), None);
    }

    #[test]
    fn a_reused_uid_does_not_inherit_a_placement() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let original = envelope("7", Some("seven@x"));
        set(&conn, &original, Some(Route::Receipts)).unwrap();
        let mut replacement = original.clone();
        replacement.message_id = Some("different@x".into());
        assert_eq!(get(&conn, &replacement).unwrap(), None);
    }
}
