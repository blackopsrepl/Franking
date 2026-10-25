/*! Conversation identity and explicit mute state for the loaded list. */

use std::collections::HashMap;

use crate::db::conversations;

use super::model::App;

impl App {
    /// Resolve conversation identity and mute indicators without changing the
    /// server-folder membership, subjects, grouping, or requested sort order.
    pub(crate) fn apply_conversation_view(&mut self) {
        self.conversation_anchors.clear();
        self.muted_ids.clear();
        let anchors_by_row = conversations::anchors_for_list(&self.envelopes);
        let mut muted: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for (envelope, anchors) in self.envelopes.iter().zip(anchors_by_row) {
            let account = envelope
                .account
                .as_deref()
                .or(self.account_name.as_deref())
                .filter(|account| !account.is_empty());
            if let (Some(account), Some(conn)) = (account, self.db.as_ref()) {
                let account_muted = muted.entry(account.to_string()).or_insert_with(|| {
                    conversations::muted_anchors(conn, account).unwrap_or_default()
                });
                if anchors.iter().any(|anchor| account_muted.contains(anchor)) {
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
}
