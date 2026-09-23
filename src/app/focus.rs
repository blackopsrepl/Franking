/*! Sequential reply queue: work through Reply later one message at a time. */

use crate::db::message_markers::Marker;
use crate::keys::View;
use crate::mail::types::Envelope;
use crate::mail::MessageDocument;

use super::model::App;

/// One pass through the response queue.
pub struct FocusState {
    pub queue: Vec<Envelope>,
    pub index: usize,
    pub document: Option<MessageDocument>,
    pub scroll: u16,
}

impl App {
    pub(crate) fn open_focus_reply(&mut self) {
        let queue = self
            .db
            .as_ref()
            .and_then(|conn| {
                let account = (self.current_folder != super::model::UNIFIED_INBOX)
                    .then_some(self.account_name.as_deref())
                    .flatten();
                crate::db::message_markers::list(conn, account, Marker::ReplyLater).ok()
            })
            .unwrap_or_default();
        if queue.is_empty() {
            self.set_status("Reply later is empty.");
            return;
        }
        self.focus = Some(FocusState {
            queue,
            index: 0,
            document: None,
            scroll: 0,
        });
        self.view = View::FocusReply;
        self.focus_load_current();
    }

    pub(crate) fn close_focus(&mut self) {
        self.focus = None;
        self.focus_reply_in_flight = false;
        self.view = View::EnvelopeList;
    }

    fn focus_load_current(&mut self) {
        let Some(focus) = self.focus.as_ref() else {
            return;
        };
        let Some(envelope) = focus.queue.get(focus.index).cloned() else {
            let done = focus.queue.len();
            self.focus = None;
            self.view = View::EnvelopeList;
            self.set_status(&format!("Reply queue finished ({done})."));
            return;
        };
        self.loading = true;
        let account = envelope.account.clone().or_else(|| self.acct_owned());
        let folder = envelope
            .folder
            .clone()
            .unwrap_or_else(|| self.current_folder.clone());
        self.worker
            .fetch_focus_message(account, folder, envelope.id);
    }

    pub(crate) fn handle_focus_message(
        &mut self,
        result: Result<MessageDocument, crate::mail::MailError>,
    ) {
        self.loading = false;
        let Some(focus) = self.focus.as_mut() else {
            return;
        };
        match result {
            Ok(document) => {
                focus.document = Some(document);
                focus.scroll = 0;
            }
            Err(error) => {
                focus.document = None;
                self.set_error(&format!("Could not read the message: {error}"));
            }
        }
    }

    pub(crate) fn focus_next(&mut self) {
        if let Some(focus) = self.focus.as_mut() {
            if focus.index + 1 < focus.queue.len() {
                focus.index += 1;
                focus.document = None;
            }
        }
        self.focus_load_current();
    }

    pub(crate) fn focus_prev(&mut self) {
        if let Some(focus) = self.focus.as_mut() {
            focus.index = focus.index.saturating_sub(1);
            focus.document = None;
        }
        self.focus_load_current();
    }

    pub(crate) fn focus_scroll(&mut self, delta: i32) {
        if let Some(focus) = self.focus.as_mut() {
            if delta > 0 {
                focus.scroll = focus.scroll.saturating_add(1);
            } else {
                focus.scroll = focus.scroll.saturating_sub(1);
            }
        }
    }

    /// Drop the current message from the response queue and move on.
    pub(crate) fn focus_done(&mut self) {
        let Some(focus) = self.focus.as_ref() else {
            return;
        };
        let Some(envelope) = focus.queue.get(focus.index).cloned() else {
            return;
        };
        if let Some(conn) = self.db.as_ref() {
            let _ = crate::db::message_markers::set(conn, &envelope, Marker::ReplyLater, false);
        }
        if let Some(focus) = self.focus.as_mut() {
            focus.queue.remove(focus.index);
            if focus.index >= focus.queue.len() {
                focus.index = focus.queue.len().saturating_sub(1);
            }
            focus.document = None;
        }
        self.focus_load_current();
    }

    /// Reply to the message the queue is showing.
    pub(crate) fn focus_reply(&mut self, all: bool) {
        let Some(envelope) = self
            .focus
            .as_ref()
            .and_then(|focus| focus.queue.get(focus.index).cloned())
        else {
            return;
        };
        self.focus_reply_in_flight = true;
        self.reply_to(envelope, all);
    }

    /// After a reply is sent, advance the queue instead of leaving the view.
    pub(crate) fn focus_advance_after_send(&mut self) {
        self.focus_reply_in_flight = false;
        if let Some(focus) = self.focus.as_mut() {
            if focus.index < focus.queue.len() {
                focus.queue.remove(focus.index);
            }
            if focus.index >= focus.queue.len() {
                focus.index = focus.queue.len().saturating_sub(1);
            }
            focus.document = None;
        }
        self.focus_load_current();
    }
}
