/*! Compose lifecycle: new, reply, forward, send, draft, discard. */

use crate::compose::{ComposeMode, ComposeState, FocusedField};
use crate::keys::View;

use crossterm::event::{KeyCode, KeyEvent};

use super::model::App;

impl App {
    pub(crate) fn load_identities_into(&self, cs: &mut ComposeState) {
        let Some(ref conn) = self.db else { return };
        let Some(ref account) = cs.account else {
            return;
        };
        let identities = crate::identities::list_for_account(conn, account).unwrap_or_default();
        // Pre-select the default identity, if any.
        let default_idx = identities.iter().position(|i| i.is_default);
        cs.from_identities = identities;
        cs.from_idx = default_idx;
    }

    pub(crate) fn compose(&mut self) {
        let mut cs = ComposeState::new(ComposeMode::New, self.acct_owned());
        self.load_identities_into(&mut cs);
        self.compose_state = Some(cs);
        self.loading = true;
        self.worker.fetch_template_write(self.acct_owned());
    }

    pub(crate) fn reply(&mut self, all: bool) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let mode = if all {
                ComposeMode::ReplyAll
            } else {
                ComposeMode::Reply
            };
            let mut cs = ComposeState::new(mode, self.acct_owned());
            cs.reply_to_id = Some(id.clone());
            cs.reply_to_folder = Some(self.current_folder.clone());
            self.load_identities_into(&mut cs);
            self.compose_state = Some(cs);
            self.loading = true;
            self.worker.fetch_template_reply(
                self.acct_owned(),
                self.current_folder.clone(),
                id,
                all,
            );
        }
    }

    pub(crate) fn forward(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let mut cs = ComposeState::new(ComposeMode::Forward, self.acct_owned());
            cs.reply_to_id = Some(id.clone());
            cs.reply_to_folder = Some(self.current_folder.clone());
            self.load_identities_into(&mut cs);
            self.compose_state = Some(cs);
            self.loading = true;
            self.worker
                .fetch_template_forward(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    /// Activate the focused compose control.
    pub(crate) fn compose_enter_insert(&mut self) {
        // Grab focused + confirm_discard without keeping a borrow on self.
        let (focused, confirm_discard) = match self.compose_state.as_ref() {
            Some(cs) => (cs.focused, cs.confirm_discard),
            None => return,
        };
        if confirm_discard {
            return;
        }
        match focused {
            FocusedField::From => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.cycle_from_next();
                }
            }
            FocusedField::To | FocusedField::Cc | FocusedField::Bcc | FocusedField::Subject => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.focused = cs.focused.next();
                }
            }
            FocusedField::Body => {}
            FocusedField::Send => {
                self.compose_send();
            }
            FocusedField::Draft => {
                self.compose_save_draft();
            }
            FocusedField::Attach => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.attach_input = Some(String::new());
                }
            }
            FocusedField::Discard => {
                self.compose_discard();
            }
        }
    }

    /// Exit the focused compose control back to the body.
    pub(crate) fn compose_exit_to_nav(&mut self) {
        let Some(ref mut cs) = self.compose_state else {
            return;
        };
        // Handle confirm-discard overlay
        if cs.confirm_discard {
            cs.confirm_discard = false;
            return;
        }
        match cs.focused {
            FocusedField::Send
            | FocusedField::Draft
            | FocusedField::Attach
            | FocusedField::Discard => {
                cs.focused = FocusedField::Body;
            }
            _ => {}
        }
    }

    /// Consume a key while the attach-path prompt is open. Returns false when
    /// the prompt is not active so normal compose handling proceeds.
    pub(crate) fn compose_handle_attach_input(&mut self, key: KeyEvent) -> bool {
        let Some(cs) = self.compose_state.as_mut() else {
            return false;
        };
        if cs.attach_input.is_none() {
            return false;
        }

        let mut submit = false;
        let mut cancel = false;
        {
            let input = cs.attach_input.as_mut().expect("checked above");
            match key.code {
                KeyCode::Enter => submit = true,
                KeyCode::Esc => cancel = true,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => input.push(c),
                _ => {}
            }
        }

        let mut added = None;
        if submit {
            let path = cs
                .attach_input
                .take()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !path.is_empty() {
                cs.attachments.push(path.clone());
                cs.dirty = true;
                added = Some(path);
            }
        } else if cancel {
            cs.attach_input = None;
        }

        if let Some(path) = added {
            self.set_status(&format!("Attached {path}."));
        }
        true
    }

    pub(crate) fn compose_send(&mut self) {
        if let Some(ref cs) = self.compose_state {
            let template = crate::compose::reassemble_template(cs);
            self.loading = true;
            self.worker.send_template(self.acct_owned(), template);
        }
    }

    pub(crate) fn compose_save_draft(&mut self) {
        if let Some(ref cs) = self.compose_state {
            let template = crate::compose::reassemble_template(cs);
            self.loading = true;
            self.worker.save_draft(self.acct_owned(), template);
        }
    }

    pub(crate) fn compose_discard(&mut self) {
        if let Some(ref cs) = self.compose_state {
            if cs.dirty || !crate::compose::body_is_empty(cs) {
                // Ask for confirmation
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = true;
                }
            } else {
                // Empty / pristine — discard immediately
                self.compose_state = None;
                self.view = View::EnvelopeList;
            }
        }
    }
}
