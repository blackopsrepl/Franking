/*! Outbox of messages that failed to send. */

use crate::keys::View;
use crate::mail::outbox;
use crate::mail::outbox::OutboxItem;

use super::model::App;

/// Outbox browser and pending-send state.
#[derive(Default)]
pub struct OutboxState {
    pub items: Vec<OutboxItem>,
    pub index: usize,
    /// Queued message awaiting a second discard press.
    pub pending_discard: Option<i64>,
    /// Template and protection of the message currently being sent.
    pub pending_send: Option<(String, bool, bool)>,
}

impl App {
    /// Open the outbox and load its contents.
    pub(crate) fn open_outbox(&mut self) {
        self.outbox.pending_discard = None;
        self.view = View::Outbox;
        self.worker.fetch_outbox();
    }

    pub(crate) fn close_outbox(&mut self) {
        self.outbox.pending_discard = None;
        self.view = View::EnvelopeList;
    }

    pub(crate) fn outbox_next(&mut self) {
        if !self.outbox.items.is_empty() {
            self.outbox.index = (self.outbox.index + 1).min(self.outbox.items.len() - 1);
        }
    }

    pub(crate) fn outbox_prev(&mut self) {
        self.outbox.index = self.outbox.index.saturating_sub(1);
    }

    fn selected_outbox_item(&self) -> Option<&OutboxItem> {
        self.outbox.items.get(self.outbox.index)
    }

    /// Send the highlighted queued message now.
    pub(crate) fn send_outbox_item(&mut self) {
        let Some(item) = self.selected_outbox_item() else {
            return;
        };
        let id = item.id;
        let passphrase = self.crypto_passphrase.clone();
        self.loading = true;
        self.worker.send_outbox_item(id, passphrase);
    }

    /// Discard the highlighted queued message after a second press.
    pub(crate) fn discard_outbox_item(&mut self) {
        let Some(id) = self.selected_outbox_item().map(|item| item.id) else {
            return;
        };
        if self.outbox.pending_discard != Some(id) {
            self.outbox.pending_discard = Some(id);
            self.set_status("Press d again to discard the queued message.");
            return;
        }
        self.outbox.pending_discard = None;
        self.loading = true;
        self.worker.discard_outbox_item(id);
    }

    /// Record the message being sent so a failure can queue it.
    pub(crate) fn remember_pending_send(&mut self, template: String, sign: bool, encrypt: bool) {
        self.outbox.pending_send = Some((template, sign, encrypt));
    }

    /// Queue the last send attempt when it failed.
    pub(crate) fn queue_failed_send(&mut self, account: Option<String>) {
        let Some((template, sign, encrypt)) = self.outbox.pending_send.take() else {
            return;
        };
        let Some(ref conn) = self.db else {
            return;
        };
        let queued = outbox::enqueue(conn, account.as_deref(), &template, sign, encrypt)
            .ok()
            .flatten()
            .is_some();
        if queued {
            if let Ok(count) = outbox::count(conn) {
                self.set_status(&format!(
                    "Send failed; message kept in the outbox ({count} queued)."
                ));
            }
        }
    }

    /// Adopt the queued messages returned by the worker.
    pub(crate) fn handle_outbox(
        &mut self,
        result: Result<Vec<OutboxItem>, crate::mail::MailError>,
    ) {
        self.loading = false;
        match result {
            Ok(items) => {
                self.outbox.items = items;
                if self.outbox.index >= self.outbox.items.len() {
                    self.outbox.index = self.outbox.items.len().saturating_sub(1);
                }
            }
            Err(error) => self.set_error(&format!("Outbox: {error}")),
        }
    }
}
