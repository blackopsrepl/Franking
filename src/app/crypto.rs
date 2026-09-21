/*! Passphrase unlock prompt for PGP secret keys. */

use crate::keys::View;

use super::model::App;

impl App {
    /// Open the passphrase prompt, seeding it with an empty buffer.
    pub(crate) fn enter_unlock_prompt(&mut self) {
        self.unlock_input.clear();
        self.view = View::PassphrasePrompt;
    }

    pub(crate) fn unlock_input(&mut self, c: char) {
        self.unlock_input.push(c);
    }

    pub(crate) fn unlock_backspace(&mut self) {
        self.unlock_input.pop();
    }

    /// Abandon the prompt without touching the cached passphrase.
    pub(crate) fn cancel_unlock(&mut self) {
        self.unlock_input.clear();
        self.view = View::MessageView;
    }

    /// Adopt the typed passphrase and retry crypto processing on the message.
    pub(crate) fn submit_unlock(&mut self) {
        self.crypto_passphrase = std::mem::take(&mut self.unlock_input);
        self.view = View::MessageView;
        self.reprocess_crypto();
    }

    /// Re-run PGP/S/MIME processing for the loaded message in place.
    pub(crate) fn reprocess_crypto(&mut self) {
        let Some(mut message) = self.message_content.take() else {
            return;
        };
        let passphrase = self.crypto_passphrase.clone();
        self.pgp_status = super::pgp::process_pgp(&mut message, &passphrase)
            .or_else(|| super::smime::process_smime(&mut message));
        self.message_content = Some(message);
    }
}
