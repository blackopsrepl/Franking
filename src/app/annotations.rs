/*! Private notes and display-only subject aliases for the selected message. */

use crate::db::annotations;
use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn open_note_prompt(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.set_status("Select a message to annotate.");
            return;
        };
        self.annotation_input = self
            .db
            .as_ref()
            .and_then(|conn| annotations::note(conn, &envelope).ok().flatten())
            .unwrap_or_default();
        self.previous_view = Some(self.view);
        self.view = View::MessageNote;
    }

    pub(crate) fn open_subject_alias_prompt(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.set_status("Select a message to rename.");
            return;
        };
        let anchors = self
            .conversation_anchors
            .get(&envelope.id)
            .cloned()
            .unwrap_or_else(|| crate::db::conversations::anchors(&envelope));
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        self.annotation_input = self
            .db
            .as_ref()
            .and_then(|conn| annotations::alias(conn, &account, &anchors).ok().flatten())
            .unwrap_or_default();
        self.previous_view = Some(self.view);
        self.view = View::SubjectAlias;
    }

    pub(crate) fn annotation_input(&mut self, c: char) {
        self.annotation_input.push(c);
    }

    pub(crate) fn annotation_backspace(&mut self) {
        self.annotation_input.pop();
    }

    pub(crate) fn cancel_annotation(&mut self) {
        self.annotation_input.clear();
        self.view = self.previous_view.take().unwrap_or(View::EnvelopeList);
    }

    /// Store the note or alias typed in the prompt.
    pub(crate) fn submit_annotation(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.cancel_annotation();
            return;
        };
        let text = self.annotation_input.clone();
        let anchors = self
            .conversation_anchors
            .get(&envelope.id)
            .cloned()
            .unwrap_or_else(|| crate::db::conversations::anchors(&envelope));
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        let editing_note = self.view == View::MessageNote;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let result = if editing_note {
            annotations::set_note(conn, &envelope, &text)
        } else {
            annotations::set_alias(conn, &account, &anchors, &text)
        };
        match result {
            Ok(()) => {
                self.annotation_input.clear();
                self.view = self.previous_view.take().unwrap_or(View::EnvelopeList);
                self.set_status(if editing_note {
                    if text.trim().is_empty() {
                        "Note cleared."
                    } else {
                        "Note saved."
                    }
                } else if text.trim().is_empty() {
                    "Subject alias cleared."
                } else {
                    "Subject renamed for you only."
                });
                if editing_note {
                    self.message_note = self
                        .db
                        .as_ref()
                        .and_then(|conn| annotations::note(conn, &envelope).ok().flatten());
                } else {
                    self.load_envelopes();
                }
            }
            Err(error) => self.set_error(&format!("Could not save: {error}")),
        }
    }
}
