/*! Replying to many senders at once. */

use crate::compose::{ComposeMode, ComposeState};

use super::model::App;

impl App {
    /// Compose one message addressed to every selected sender.
    pub(crate) fn bulk_reply(&mut self) {
        let envelopes: Vec<crate::mail::types::Envelope> = if self.selected.is_empty() {
            self.selected_envelope().cloned().into_iter().collect()
        } else {
            self.envelopes
                .iter()
                .filter(|envelope| self.selected.contains(&envelope.id))
                .cloned()
                .collect()
        };
        let mut addresses: Vec<String> = Vec::new();
        for envelope in &envelopes {
            if let Some(address) = crate::db::sender_routes::sender_address(&envelope.sender) {
                if !addresses.contains(&address) {
                    addresses.push(address);
                }
            }
        }
        if addresses.is_empty() {
            self.set_status("No senders to reply to.");
            return;
        }
        self.selected.clear();
        self.pending_reply_marker = None;
        let mut cs = ComposeState::new(ComposeMode::New, self.acct_owned());
        self.load_identities_into(&mut cs);
        self.compose_state = Some(cs);
        self.pending_bulk_to = Some(addresses.join(", "));
        self.loading = true;
        self.worker.fetch_template_write(self.acct_owned());
    }
}
