/*! Compose action dispatch. */

use crate::compose::FocusedField;
use crate::keys::{Action, EditMode, View};

use super::model::App;

impl App {
    /// Handle the compose shell actions.
    pub(crate) fn handle_compose_action(&mut self, action: Action) {
        match action {
            Action::ComposeFieldNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeFieldPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeLeaveBodyNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeLeaveBodyPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeSend => self.compose_send(),
            Action::ComposeDiscard => self.compose_discard(),
            Action::ComposeConfirmDiscard => {
                self.clear_autosave();
                self.compose_state = None;
                self.pending_draft = None;
                self.view = View::EnvelopeList;
            }
            Action::ComposeCancelDiscard => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = false;
                }
            }
            Action::ComposeInput(c) => self.compose_input(c),
            Action::ComposeBackspace => {
                let is_address = {
                    let cs = self.compose_state.as_ref();
                    cs.map(|cs| {
                        cs.edit_mode == EditMode::Insert
                            && matches!(
                                cs.focused,
                                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
                            )
                    })
                    .unwrap_or(false)
                };
                if let Some(ref mut cs) = self.compose_state {
                    if cs.edit_mode == EditMode::Insert {
                        if let Some(field) = cs.focused_line_field_mut() {
                            field.pop();
                        }
                    }
                }
                if is_address {
                    self.update_autocomplete();
                }
            }
            Action::ComposeEnterInsert => {
                self.compose_enter_insert();
            }
            Action::ComposeExitToNav => {
                self.compose_exit_to_nav();
            }
            // ── EditorKey: forwarded to the focused compose field ────
            _ => {}
        }
    }
}
