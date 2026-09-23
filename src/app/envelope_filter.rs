/*! Lane filtering for a loaded page, including sender, placement, and bypass
decisions resolved once per account. */

use std::collections::HashMap;

use crate::db::{account_policy, message_routes, sender_routes};
use crate::mail::types::Envelope;

use super::model::App;

impl App {
    /// Keep only envelopes whose effective lane matches the active triage lane.
    /// Returns `None` when the local database is unavailable or a lookup fails,
    /// after reporting the failure.
    pub(crate) fn filter_triage_lane(&mut self, envelopes: Vec<Envelope>) -> Option<Vec<Envelope>> {
        let lane = self.triage_lane?;
        let Some(conn) = self.db.as_ref() else {
            self.loading = false;
            self.set_error("Triage needs the local database.");
            return None;
        };
        // Load each account's routes once instead of a query per row.
        let mut placements: HashMap<String, message_routes::PlacementMap> = HashMap::new();
        let mut senders: HashMap<String, HashMap<String, sender_routes::Route>> = HashMap::new();
        let mut tokens: HashMap<String, Option<String>> = HashMap::new();
        let mut matching = Vec::new();
        for envelope in envelopes {
            let Some(account) = envelope
                .account
                .clone()
                .or_else(|| self.account_name.clone())
                .filter(|account| !account.is_empty())
            else {
                continue;
            };
            let account_placements = placements.entry(account.clone()).or_insert_with(|| {
                message_routes::overrides_for_account(conn, &account).unwrap_or_default()
            });
            let account_senders = senders.entry(account.clone()).or_insert_with(|| {
                sender_routes::routes_for_account(conn, &account).unwrap_or_default()
            });
            let folder = envelope.folder.clone().unwrap_or_default();
            let placement = account_placements
                .get(&(folder, envelope.id.clone()))
                .filter(|(message_id, _)| *message_id == envelope.message_id)
                .map(|(_, route)| *route);
            let effective = placement.unwrap_or_else(|| {
                match sender_routes::sender_address(&envelope.sender) {
                    Some(sender) => account_senders
                        .get(&sender)
                        .copied()
                        .unwrap_or(sender_routes::Route::Screening),
                    // An unparseable sender stays visible for manual handling.
                    None => sender_routes::Route::Inbox,
                }
            });
            // A trusting stranger's bypass token lifts Screening only.
            let effective = if effective == sender_routes::Route::Screening {
                let token = tokens.entry(account.clone()).or_insert_with(|| {
                    account_policy::bypass_token(conn, &account).unwrap_or(None)
                });
                match token.as_deref() {
                    Some(token) if account_policy::subject_has_token(&envelope.subject, token) => {
                        sender_routes::Route::Inbox
                    }
                    _ => effective,
                }
            } else {
                effective
            };
            if effective == lane {
                matching.push(envelope);
            }
        }
        Some(matching)
    }
}
