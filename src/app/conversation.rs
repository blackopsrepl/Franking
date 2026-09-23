/*! Quiet conversations and resurfacing over the loaded message list. */

use std::collections::HashMap;

use crate::db::conversations;
use crate::keys::View;
use crate::mail::types::Envelope;

use super::model::App;

/// Current UTC time in the stored timestamp form.
pub fn now_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl App {
    /// Recompute conversation anchors and rules for the loaded list, then order
    /// resurfaced conversations first and quieted ones last.
    pub(crate) fn apply_conversation_view(&mut self) {
        self.conversation_anchors.clear();
        self.muted_ids.clear();
        self.resurfaced_ids.clear();
        self.loud_ids.clear();
        self.stage_of_anchor.clear();
        self.subject_aliases.clear();
        let now = now_utc();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        // Resolve every row's anchors with one Message-ID index.
        let mut anchors_by_row = conversations::anchors_for_list(&self.envelopes);
        // Load each account's state once instead of a query per row.
        let accounts: Vec<String> = self
            .envelopes
            .iter()
            .filter_map(|envelope| {
                envelope
                    .account
                    .clone()
                    .or_else(|| self.account_name.clone())
            })
            .filter(|account| !account.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let mut rules: HashMap<String, HashMap<String, conversations::Rule>> = HashMap::new();
        let mut aliases: HashMap<String, HashMap<String, String>> = HashMap::new();
        for account in &accounts {
            rules.insert(
                account.clone(),
                conversations::rules_for_account(conn, account).unwrap_or_default(),
            );
            aliases.insert(
                account.clone(),
                crate::db::annotations::aliases_for_account(conn, account).unwrap_or_default(),
            );
        }
        for (index, envelope) in self.envelopes.iter().enumerate() {
            let anchors = &anchors_by_row[index];
            let account = envelope
                .account
                .as_deref()
                .or(self.account_name.as_deref())
                .filter(|account| !account.is_empty());
            let account_rules = account.and_then(|account| rules.get(account));
            let account_aliases = account.and_then(|account| aliases.get(account));
            let mut muted = false;
            let mut loud = false;
            let mut due = false;
            for anchor in anchors {
                if let Some(rule) = account_rules.and_then(|rules| rules.get(anchor)) {
                    muted |= rule.muted;
                    loud |= rule.loud;
                    due |= rule
                        .resurface_at
                        .as_deref()
                        .is_some_and(|at| at <= now.as_str());
                }
                if let Some(alias) = account_aliases.and_then(|aliases| aliases.get(anchor)) {
                    self.subject_aliases
                        .entry(envelope.id.clone())
                        .or_insert_with(|| alias.clone());
                }
            }
            if muted {
                self.muted_ids.insert(envelope.id.clone());
            }
            if due {
                self.resurfaced_ids.insert(envelope.id.clone());
            }
            if loud {
                self.loud_ids.insert(envelope.id.clone());
            }
            self.conversation_anchors.insert(
                envelope.id.clone(),
                std::mem::take(&mut anchors_by_row[index]),
            );
        }
        // Keep the visible order stable: resurfaced, ordinary, then quieted.
        let muted = &self.muted_ids;
        let resurfaced = &self.resurfaced_ids;
        self.envelopes.sort_by(|left, right| {
            let rank = |envelope: &Envelope| {
                if resurfaced.contains(&envelope.id) {
                    0
                } else if muted.contains(&envelope.id) {
                    2
                } else {
                    1
                }
            };
            rank(left).cmp(&rank(right))
        });
        // Cover previously seen Inbox mail until the user lifts it.
        self.covered_count = 0;
        if self.cover_seen
            && !self.cover_revealed
            && self.triage_lane == Some(crate::db::sender_routes::Route::Inbox)
        {
            let before = self.envelopes.len();
            self.envelopes.retain(|envelope| !envelope.is_seen());
            self.covered_count = before - self.envelopes.len();
        }
        self.load_stages_and_filter(&accounts);
        self.load_collections_and_filter(&accounts);
        self.load_merge_roots(&accounts);
        // Collapse bundled senders after every other ordering decision.
        self.load_bundled_senders(&accounts);
        self.apply_bundles();
    }

    /// Mute or unmute the selected conversation.
    pub(crate) fn toggle_mute_conversation(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let anchors = self
            .conversation_anchors
            .get(&envelope.id)
            .cloned()
            .unwrap_or_else(|| conversations::anchors(&envelope));
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let muted = conversations::is_muted(conn, &account, &anchors).unwrap_or(false);
        match conversations::set_muted(conn, &account, &anchors, !muted) {
            Ok(()) => {
                self.set_status(if muted {
                    "Conversation is loud again."
                } else {
                    "Conversation quieted; new replies stay out of the way."
                });
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not change the conversation: {error}")),
        }
    }

    /// Ask the selected conversation to notify regardless of the rule.
    pub(crate) fn toggle_loud_conversation(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let anchors = self
            .conversation_anchors
            .get(&envelope.id)
            .cloned()
            .unwrap_or_else(|| conversations::anchors(&envelope));
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let loud = self.loud_ids.contains(&envelope.id);
        match conversations::set_loud(conn, &account, &anchors, !loud) {
            Ok(()) => {
                self.set_status(if loud {
                    "Conversation follows the notification rule again."
                } else {
                    "Conversation will always notify."
                });
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not change the conversation: {error}")),
        }
    }

    // ── Resurface prompt ─────────────────────────────────────────────

    pub(crate) fn open_resurface_prompt(&mut self) {
        if self.selected_envelope().is_none() {
            self.set_status("Select a message to resurface.");
            return;
        }
        self.resurface_input.clear();
        self.view = View::ResurfacePrompt;
    }

    pub(crate) fn resurface_input(&mut self, c: char) {
        self.resurface_input.push(c);
    }

    pub(crate) fn resurface_backspace(&mut self) {
        self.resurface_input.pop();
    }

    pub(crate) fn cancel_resurface(&mut self) {
        self.resurface_input.clear();
        self.view = View::EnvelopeList;
    }

    /// Set or clear the selected conversation's resurface time.
    pub(crate) fn submit_resurface(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.cancel_resurface();
            return;
        };
        let anchors = self
            .conversation_anchors
            .get(&envelope.id)
            .cloned()
            .unwrap_or_else(|| conversations::anchors(&envelope));
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        let typed = self.resurface_input.trim().to_string();
        self.resurface_input.clear();
        self.view = View::EnvelopeList;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let at = if typed.is_empty() || typed.eq_ignore_ascii_case("off") {
            None
        } else {
            match crate::compose::parse_delay(&typed) {
                Some(seconds) => Some(resurface_time_in(seconds)),
                None => {
                    self.set_error("Use a delay like 30m, 2h, or 1d, or 'off' to clear.");
                    return;
                }
            }
        };
        match conversations::set_resurface(conn, &account, &anchors, at.as_deref()) {
            Ok(()) => {
                self.set_status(match at {
                    Some(_) => "Conversation will resurface.",
                    None => "Resurface cleared.",
                });
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not resurface: {error}")),
        }
    }
}

/// UTC RFC 3339 timestamp `seconds` from now.
pub fn resurface_time_in(seconds: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(seconds))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::resurface_time_in;

    #[test]
    fn resurface_time_is_utc_and_in_the_future() {
        let stamp = resurface_time_in(3600);
        assert!(stamp.ends_with('Z'), "stored form is UTC: {stamp}");
        let parsed = chrono::DateTime::parse_from_rfc3339(&stamp).unwrap();
        assert!((3500..=3700).contains(
            &parsed
                .signed_duration_since(chrono::Utc::now())
                .num_seconds()
        ));
    }
}
