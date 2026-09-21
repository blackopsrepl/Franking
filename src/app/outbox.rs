/*! Outbox of messages that failed to send. */

use crate::keys::View;
use crate::mail::outbox;
use crate::mail::outbox::OutboxItem;

use super::model::App;

/// Ticks between outbox flushes (250ms each): 240 ticks ≈ 60 seconds.
const FLUSH_INTERVAL: u64 = 240;

/// Outbox browser and pending-send state.
#[derive(Default)]
pub struct OutboxState {
    pub items: Vec<OutboxItem>,
    pub index: usize,
    /// Queued message awaiting a second discard press.
    pub pending_discard: Option<i64>,
    /// Template and protection of the message currently being sent.
    pub pending_send: Option<(String, outbox::Protection)>,
    /// Ticks since the last due-message check.
    pub ticks_since_flush: u64,
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

    /// Queue the message in progress for a scheduled send.
    pub(crate) fn schedule_send(&mut self, send_after: String) {
        let (template, protection, account) = {
            let Some(cs) = self.compose_state.as_ref() else {
                return;
            };
            let template = crate::compose::reassemble_template(cs);
            let options = self.send_options(cs);
            (
                template,
                outbox::Protection::from(&options),
                cs.account.clone(),
            )
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match outbox::enqueue(
            conn,
            account.as_deref(),
            &template,
            protection,
            Some(&send_after),
        ) {
            Ok(Some(_)) => {
                self.compose_state = None;
                self.view = View::EnvelopeList;
                self.clear_autosave();
                self.set_status(&format!("Scheduled to send at {send_after}."));
                self.worker.fetch_outbox();
            }
            Ok(None) => self.set_status("That message is already queued."),
            Err(error) => self.set_error(&format!("Could not schedule the message: {error}")),
        }
    }

    /// Record the message being sent so a failure can queue it.
    pub(crate) fn remember_pending_send(
        &mut self,
        template: String,
        protection: outbox::Protection,
    ) {
        self.outbox.pending_send = Some((template, protection));
    }

    /// Queue the last send attempt when it failed.
    pub(crate) fn queue_failed_send(&mut self, account: Option<String>) {
        let Some((template, protection)) = self.outbox.pending_send.take() else {
            return;
        };
        let Some(ref conn) = self.db else {
            return;
        };
        let queued = outbox::enqueue(conn, account.as_deref(), &template, protection, None)
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

impl App {
    /// Open the send-later prompt for the message in progress.
    pub(crate) fn open_schedule_prompt(&mut self) {
        if self.compose_state.is_none() {
            return;
        }
        self.schedule_input = "1h".to_string();
        self.view = View::SchedulePrompt;
    }

    pub(crate) fn schedule_input(&mut self, c: char) {
        self.schedule_input.push(c);
    }

    pub(crate) fn schedule_backspace(&mut self) {
        self.schedule_input.pop();
    }

    pub(crate) fn cancel_schedule(&mut self) {
        self.schedule_input.clear();
        self.view = View::Compose;
    }

    /// Parse the delay and queue the message for later.
    pub(crate) fn submit_schedule(&mut self) {
        let Some(seconds) = crate::compose::parse_delay(&self.schedule_input) else {
            self.set_error("Use a delay like 30m, 2h, or 1d.");
            return;
        };
        let send_after = crate::compose::send_time_in(seconds);
        self.schedule_input.clear();
        self.schedule_send(send_after);
    }
}

impl App {
    /// Periodically send queued messages whose time has come.
    pub(crate) fn outbox_flush_tick(&mut self) {
        self.outbox.ticks_since_flush += 1;
        if self.outbox.ticks_since_flush < FLUSH_INTERVAL {
            return;
        }
        self.outbox.ticks_since_flush = 0;

        let Some(conn) = self.db.as_ref() else {
            return;
        };
        let now = chrono::Local::now().to_rfc3339();
        let due = outbox::due(conn, &now)
            .map(|items| items.len())
            .unwrap_or(0);
        if due == 0 {
            return;
        }
        let passphrase = self.crypto_passphrase.clone();
        self.set_status(&format!("Sending {due} queued message(s)..."));
        self.worker.flush_outbox(passphrase);
    }
}
