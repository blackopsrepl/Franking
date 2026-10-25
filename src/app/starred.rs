/*! Starred: server-flagged messages from locally cached folders. */

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::mail::store;
use crate::mail::types::{Envelope, Sender};

use super::model::App;

impl App {
    pub(crate) fn load_starred(&mut self) {
        let result = self
            .db
            .as_ref()
            .context("local database is unavailable")
            .and_then(|conn| cached_starred(conn, self.local_view_account()));
        match result {
            Ok(envelopes) => {
                self.handle_envelopes_loaded(envelopes);
                self.set_status(
                    "Starred · cached messages across folders; refresh source folders to update.",
                );
            }
            Err(error) => {
                self.loading = false;
                self.set_error(&format!("Could not open Starred (cached): {error}"));
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/sidebar.rs"]
mod tests;

fn cached_starred(conn: &Connection, account: Option<&str>) -> Result<Vec<Envelope>> {
    let mut statement = conn.prepare(
        "SELECT account, folder, uid FROM messages
         WHERE (?1 IS NULL OR account = ?1)
           AND instr(' ' || lower(flags) || ' ', ' flagged ') > 0
         ORDER BY date_epoch DESC, account, folder, uid",
    )?;
    let keys = statement
        .query_map([account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut envelopes = Vec::new();
    for (account, folder, uid) in keys {
        // Older Maildir caches used the flag-bearing filename as the UID.
        // Once refreshed, the stable base row is authoritative, even unstarred.
        if let Some((base, _)) = uid.split_once(":2,") {
            if store::get_message(conn, &account, &folder, base)?.is_some() {
                continue;
            }
        }
        if let Some(stored) = store::get_message(conn, &account, &folder, &uid)? {
            let mut envelope = stored.to_envelope();
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
