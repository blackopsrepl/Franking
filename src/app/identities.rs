/*! Identity list and edit form handling. */

use crate::identity_edit::IdentityEditState;
use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn reload_identities(&mut self) {
        if let Some(ref conn) = self.db {
            let account = self.account_name.as_deref().unwrap_or("").to_string();
            self.identities =
                crate::identities::list_for_account(conn, &account).unwrap_or_default();
            if self.identities.is_empty() {
                self.identity_index = None;
            } else {
                let idx = self
                    .identity_index
                    .unwrap_or(0)
                    .min(self.identities.len() - 1);
                self.identity_index = Some(idx);
            }
        }
    }

    pub(crate) fn open_identities(&mut self) {
        self.reload_identities();
        self.previous_view = Some(self.view);
        self.view = View::IdentityList;
    }

    pub(crate) fn identity_list_move(&mut self, delta: i32) {
        let len = self.identities.len();
        if len == 0 {
            return;
        }
        let current = self.identity_index.unwrap_or(0);
        let next = if delta > 0 {
            (current + 1).min(len - 1)
        } else {
            current.saturating_sub(1)
        };
        self.identity_index = Some(next);
    }

    pub(crate) fn identity_list_close(&mut self) {
        self.view = self.previous_view.unwrap_or(View::EnvelopeList);
        self.previous_view = None;
    }

    pub(crate) fn identity_new(&mut self) {
        let account = self.account_name.as_deref().unwrap_or("").to_string();
        self.identity_edit_state = Some(IdentityEditState::new(&account));
        self.view = View::IdentityEdit;
    }

    pub(crate) fn identity_edit_selected(&mut self) {
        let identity = self
            .identity_index
            .and_then(|i| self.identities.get(i))
            .cloned();
        if let Some(id) = identity {
            self.identity_edit_state = Some(IdentityEditState::from_identity(&id));
            self.view = View::IdentityEdit;
        }
    }

    pub(crate) fn identity_delete(&mut self) {
        let Some(idx) = self.identity_index else {
            return;
        };
        let Some(identity) = self.identities.get(idx) else {
            return;
        };
        let id = identity.id;
        if let Some(ref conn) = self.db {
            match crate::identities::delete(conn, id) {
                Ok(()) => {
                    self.set_status("Identity deleted.");
                    self.reload_identities();
                }
                Err(e) => self.set_error(&format!("delete identity: {e}")),
            }
        }
    }

    pub(crate) fn identity_set_default(&mut self) {
        let Some(idx) = self.identity_index else {
            return;
        };
        let Some(identity) = self.identities.get(idx) else {
            return;
        };
        let id = identity.id;
        let account = identity.account.clone();
        if let Some(ref conn) = self.db {
            match crate::identities::set_default(conn, &account, id) {
                Ok(()) => {
                    self.set_status("Default identity set.");
                    self.reload_identities();
                }
                Err(e) => self.set_error(&format!("set default: {e}")),
            }
        }
    }

    pub(crate) fn identity_edit_field_next(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            s.focused = s.focused.next();
        }
    }

    pub(crate) fn identity_edit_field_prev(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            s.focused = s.focused.prev();
        }
    }

    pub(crate) fn identity_edit_input(&mut self, c: char) {
        if let Some(ref mut s) = self.identity_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.push(c);
            }
            s.error = None;
        }
    }

    pub(crate) fn identity_edit_backspace(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.pop();
            }
        }
    }

    pub(crate) fn identity_edit_toggle(&mut self) {
        use crate::identity_edit::IdentityField;
        let focused = self.identity_edit_state.as_ref().map(|s| s.focused);
        match focused {
            Some(IdentityField::Save) => self.identity_edit_save(),
            Some(IdentityField::Cancel) => self.identity_edit_cancel(),
            Some(IdentityField::IsDefault) => {
                if let Some(ref mut s) = self.identity_edit_state {
                    s.toggle_default();
                }
            }
            Some(_) => {
                // Enter on a text field advances to the next field.
                if let Some(ref mut s) = self.identity_edit_state {
                    s.focused = s.focused.next();
                }
            }
            None => {}
        }
    }

    pub(crate) fn identity_edit_save(&mut self) {
        // Validate and extract data while borrowing immutably.
        let validation = self
            .identity_edit_state
            .as_ref()
            .map(|s| (s.validate(), s.identity_id, s.account.clone()));

        let Some((validation_result, identity_id, account)) = validation else {
            return;
        };

        match validation_result {
            Ok((name, display_name, email, signature, is_default)) => {
                if let Some(ref conn) = self.db {
                    let result = if let Some(id) = identity_id {
                        crate::identities::delete(conn, id).and_then(|_| {
                            crate::identities::add(
                                conn,
                                &account,
                                name.as_deref(),
                                display_name.as_deref(),
                                &email,
                                signature.as_deref(),
                                is_default,
                            )
                            .map(|_| ())
                        })
                    } else {
                        crate::identities::add(
                            conn,
                            &account,
                            name.as_deref(),
                            display_name.as_deref(),
                            &email,
                            signature.as_deref(),
                            is_default,
                        )
                        .map(|_| ())
                    };
                    match result {
                        Ok(()) => {
                            self.identity_edit_state = None;
                            self.view = View::IdentityList;
                            self.set_status("Identity saved.");
                            self.reload_identities();
                        }
                        Err(e) => {
                            if let Some(ref mut s) = self.identity_edit_state {
                                s.error = Some(format!("Save failed: {e}"));
                            }
                        }
                    }
                }
            }
            Err(msg) => {
                if let Some(ref mut s) = self.identity_edit_state {
                    s.error = Some(msg);
                }
            }
        }
    }

    pub(crate) fn identity_edit_cancel(&mut self) {
        self.identity_edit_state = None;
        self.view = View::IdentityList;
    }
}
