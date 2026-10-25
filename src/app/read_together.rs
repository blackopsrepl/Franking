/*! Read several selected messages together in one scroll. */

use crate::keys::View;
use crate::mail::types::Envelope;
use crate::mail::MessageDocument;

use super::model::App;

impl App {
    /// Load the selected messages (or the cursor row) as one document stream.
    pub(crate) fn open_read_together(&mut self) {
        // Build targets by borrowing, so no full envelope is cloned.
        let mut targets: Vec<(String, String, String)> = Vec::new();
        if self.selected.is_empty() {
            if let Some(envelope) = self.selected_envelope() {
                targets.push(self.target_for(envelope));
            }
        } else {
            for envelope in self
                .envelopes
                .iter()
                .filter(|envelope| self.selected.contains(&envelope.id))
            {
                targets.push(self.target_for(envelope));
            }
        }
        if targets.is_empty() {
            self.set_status("Select one or more messages to read together.");
            return;
        }
        self.selected.clear();
        self.loading = true;
        self.set_status(&format!("Reading {} messages...", targets.len()));
        self.worker.fetch_read_together(targets);
    }

    /// Account, folder, and id for one message, defaulting to the current view.
    fn target_for(&self, envelope: &Envelope) -> (String, String, String) {
        (
            envelope
                .account
                .clone()
                .or_else(|| self.acct_owned())
                .unwrap_or_default(),
            envelope
                .folder
                .clone()
                .unwrap_or_else(|| self.current_folder.clone()),
            envelope.id.clone(),
        )
    }

    pub(crate) fn handle_read_together(&mut self, documents: Vec<MessageDocument>) {
        self.loading = false;
        if documents.is_empty() {
            self.set_status("Nothing to read together.");
            return;
        }
        self.read_together = Some(documents);
        self.read_together_scroll = 0;
        self.status_message.clear();
        self.status_is_error = false;
        self.view = View::ReadTogether;
    }

    pub(crate) fn close_read_together(&mut self) {
        self.read_together = None;
        self.read_together_scroll = 0;
        self.view = View::EnvelopeList;
    }
}

impl App {
    /// Render every loaded message into one scrollable document.
    pub(crate) fn render_read_together(&self, width: usize) -> String {
        let Some(documents) = self.read_together.as_ref() else {
            return String::new();
        };
        let rule = "\u{2500}".repeat(width.min(60));
        documents
            .iter()
            .enumerate()
            .map(|(index, document)| {
                let subject = document.subject();
                let from = document
                    .headers
                    .from
                    .first()
                    .map(|address| address.display())
                    .unwrap_or_default();
                format!(
                    "[{}/{}] {subject}\nFrom: {from}\n{rule}\n{}",
                    index + 1,
                    documents.len(),
                    document.render(width)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
