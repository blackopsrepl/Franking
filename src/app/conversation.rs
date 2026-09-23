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
        self.subject_aliases.clear();
        let now = now_utc();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        // Load each account's rules once instead of a query per row.
        let mut rules: HashMap<String, HashMap<String, conversations::Rule>> = HashMap::new();
        let mut aliases: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (index, envelope) in self.envelopes.iter().enumerate() {
            let anchors = conversations::anchors_with_root(&self.envelopes, index);
            let Some(account) = envelope
                .account
                .clone()
                .or_else(|| self.account_name.clone())
                .filter(|account| !account.is_empty())
            else {
                continue;
            };
            let account_rules = rules.entry(account.clone()).or_insert_with(|| {
                conversations::rules_for_account(conn, &account).unwrap_or_default()
            });
            let account_aliases = aliases.entry(account.clone()).or_insert_with(|| {
                crate::db::annotations::aliases_for_account(conn, &account).unwrap_or_default()
            });
            let mut muted = false;
            let mut due = false;
            for anchor in &anchors {
                if let Some(rule) = account_rules.get(anchor) {
                    muted |= rule.muted;
                    due |= rule
                        .resurface_at
                        .as_deref()
                        .is_some_and(|at| at <= now.as_str());
                }
                if let Some(alias) = account_aliases.get(anchor) {
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
            self.conversation_anchors
                .insert(envelope.id.clone(), anchors);
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
