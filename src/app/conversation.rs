/*! Conversation identity and explicit mute state for the loaded list. */

use std::collections::HashMap;

use crate::db::conversations;
use crate::keys::View;

use super::model::App;

impl App {
    /// Resolve conversation identity and mute indicators without changing the
    /// server-folder membership, subjects, grouping, or requested sort order.
    pub(crate) fn apply_conversation_view(&mut self) {
        self.conversation_anchors.clear();
        self.muted_ids.clear();
        self.resurfaced_ids.clear();
        self.loud_ids.clear();
        self.stage_of_anchor.clear();
        self.collection_of_anchor.clear();
        self.subject_aliases.clear();
        self.merge_roots.clear();
        self.bundled_senders.clear();
        self.bundled_reps.clear();
        self.covered_count = 0;
        let anchors_by_row = conversations::anchors_for_list(&self.envelopes);
        let mut rules: HashMap<String, HashMap<String, conversations::Rule>> = HashMap::new();
        for (envelope, anchors) in self.envelopes.iter().zip(anchors_by_row) {
            let account = envelope
                .account
                .as_deref()
                .or(self.account_name.as_deref())
                .filter(|account| !account.is_empty());
            if let (Some(account), Some(conn)) = (account, self.db.as_ref()) {
                let account_rules = rules.entry(account.to_string()).or_insert_with(|| {
                    conversations::rules_for_account(conn, account).unwrap_or_default()
                });
                if anchors
                    .iter()
                    .any(|anchor| account_rules.get(anchor).is_some_and(|rule| rule.muted))
                {
                    self.muted_ids.insert(envelope.id.clone());
                }
            }
            self.conversation_anchors
                .insert(envelope.id.clone(), anchors);
        }
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
                    "Conversation unmuted."
                } else {
                    "Conversation muted; notifications are off for its replies."
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
