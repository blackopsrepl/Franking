/*! Cached server-flagged messages; legacy Saved markers stay independent. */

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::keys::View;
use crate::mail::store;
use crate::mail::types::{Envelope, Sender};

use super::model::{App, UNIFIED_INBOX};

impl App {
    /// Shared mutation guard for cross-folder queues. Batch ids are only UIDs,
    /// so they cannot safely identify messages from different source folders.
    pub(crate) fn guard_virtual_mailbox_mutation(&mut self) -> Option<bool> {
        if !self.starred_view && self.followup_lane.is_none() {
            return None;
        }
        let allowed = self.selected.is_empty()
            && self.selected_envelope().is_some_and(|e| {
                e.account.is_some() && e.account == self.account_name && e.folder.is_some()
            });
        if !allowed {
            self.set_status(
                "Open the source account and clear batch selection before changing this message.",
            );
        }
        Some(allowed)
    }

    pub(crate) fn open_starred(&mut self) {
        self.starred_view = true;
        if self.current_folder != UNIFIED_INBOX {
            self.current_folder = "INBOX".into();
        }
        self.followup_lane = None;
        self.triage_lane = None;
        self.selected.clear();
        self.active_query = None;
        self.page = 1;
        self.folder_index = self.folders.len() + 1;
        self.envelopes.clear();
        self.envelope_state.select(None);
        self.view = View::EnvelopeList;
        self.load_envelopes();
    }

    /// Returns true when this local view handled loading (including failures).
    pub(crate) fn load_starred(&mut self) -> bool {
        if !self.starred_view {
            return false;
        }
        let account = (self.current_folder != UNIFIED_INBOX)
            .then_some(self.account_name.as_deref())
            .flatten();
        let result = self
            .db
            .as_ref()
            .context("local database is unavailable")
            .and_then(|conn| cached_starred(conn, account));
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
        true
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
