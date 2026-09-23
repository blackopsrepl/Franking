/*! Handling a loaded message: crypto verdicts, read state, contacts. */

use crate::keys::View;
use crate::mail::MessageDocument;

use super::model::App;

impl App {
    pub(crate) fn handle_message_loaded(&mut self, mut message: MessageDocument) {
        self.harvest_contacts_from_message(&message);
        let passphrase = self.crypto_passphrase.clone();
        let smime = super::smime::process_smime(&mut message);
        self.smime_signer = smime.as_ref().and_then(|outcome| outcome.untrusted.clone());
        self.pgp_status = super::pgp::process_pgp(&mut message, &passphrase)
            .or_else(|| smime.map(|outcome| outcome.status));

        self.message_content = Some(message);
        self.message_scroll = 0;
        self.loading = false;
        self.view = View::MessageView;
        self.message_note = self.selected_envelope().cloned().and_then(|envelope| {
            self.db
                .as_ref()
                .and_then(|conn| crate::db::annotations::note(conn, &envelope).ok().flatten())
        });
        self.mark_opened_message_seen();
    }

    /// Reflect "read on open" locally and, when enabled, on the server.
    fn mark_opened_message_seen(&mut self) {
        let Some(idx) = self.envelope_state.selected() else {
            return;
        };
        let already_seen = self
            .envelopes
            .get(idx)
            .map(|envelope| envelope.is_seen())
            .unwrap_or(true);
        if already_seen {
            return;
        }
        if let Some(envelope) = self.envelopes.get_mut(idx) {
            envelope.flags.push("Seen".to_string());
        }
        if !self.mark_read_on_open {
            return;
        }
        let Some(id) = self.selected_envelope_id().map(str::to_string) else {
            return;
        };
        self.worker.flag_add(
            self.selected_account(),
            self.selected_folder(),
            id,
            "seen".to_string(),
        );
    }
}
