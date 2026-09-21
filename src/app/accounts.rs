/*! Account list management: set default and delete. */

use crate::account_edit::{AccountEditState, AccountField};
use crate::keys::View;
use crate::mail::account_store::AccountConfig;

use super::model::App;

impl App {
    /// Make the highlighted account the default.
    pub(crate) fn set_default_account(&mut self) {
        let Some(name) = self
            .accounts
            .get(self.account_index)
            .map(|a| a.name.clone())
        else {
            return;
        };
        let Some(ref conn) = self.db else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match crate::mail::account_store::set_default_account(conn, &name) {
            Ok(()) => {
                self.load_accounts();
                self.set_status(&format!("{name} is now the default account."));
            }
            Err(error) => self.set_error(&format!("Could not set default: {error}")),
        }
    }

    /// Delete the highlighted account; a second press confirms.
    pub(crate) fn delete_account(&mut self) {
        let Some(name) = self
            .accounts
            .get(self.account_index)
            .map(|a| a.name.clone())
        else {
            return;
        };
        if self.pending_delete_account.as_deref() != Some(name.as_str()) {
            self.pending_delete_account = Some(name.clone());
            self.set_status(&format!("Press d again to delete account {name}."));
            return;
        }
        self.pending_delete_account = None;
        let Some(ref conn) = self.db else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match crate::mail::account_store::delete_account(conn, &name) {
            Ok(()) => {
                if self.account_name.as_deref() == Some(name.as_str()) {
                    self.account_name = None;
                    self.current_folder = "INBOX".to_string();
                }
                self.load_accounts();
                self.set_status(&format!("Deleted account {name}."));
            }
            Err(error) => self.set_error(&format!("Could not delete account: {error}")),
        }
    }

    /// Open the form for a new IMAP/SMTP account.
    pub(crate) fn open_account_new(&mut self) {
        self.account_edit_state = Some(AccountEditState::new());
        self.view = View::AccountEdit;
    }

    /// Open the form for the highlighted account.
    pub(crate) fn open_account_edit(&mut self) {
        let Some(name) = self
            .accounts
            .get(self.account_index)
            .map(|a| a.name.clone())
        else {
            return;
        };
        let Some(ref conn) = self.db else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match crate::mail::account_store::get_account(conn, &name) {
            Ok(Some(record)) => {
                self.account_edit_state = Some(AccountEditState::from_record(&record));
                self.view = View::AccountEdit;
            }
            Ok(None) => self.set_error(&format!("Account {name} was not found.")),
            Err(error) => self.set_error(&format!("Could not load account: {error}")),
        }
    }

    pub(crate) fn cancel_account_form(&mut self) {
        self.account_edit_state = None;
        self.view = View::AccountList;
    }

    /// Move focus to the next (or previous) form field.
    pub(crate) fn account_form_step(&mut self, delta: i32) {
        if let Some(ref mut state) = self.account_edit_state {
            state.focused = state.focused.step(delta);
        }
    }

    pub(crate) fn account_form_input(&mut self, c: char) {
        if let Some(state) = self.account_edit_state.as_mut() {
            if let Some(field) = state.focused_field_mut() {
                field.push(c);
            }
        }
    }

    pub(crate) fn account_form_backspace(&mut self) {
        if let Some(state) = self.account_edit_state.as_mut() {
            if let Some(field) = state.focused_field_mut() {
                field.pop();
            }
        }
    }

    pub(crate) fn account_form_toggle_default(&mut self) {
        if let Some(state) = self.account_edit_state.as_mut() {
            if state.focused == AccountField::Default {
                state.toggle_default();
            }
        }
    }

    /// Validate and persist the form, storing the password when one was typed.
    pub(crate) fn account_form_save(&mut self) {
        let Some(state) = self.account_edit_state.as_ref() else {
            return;
        };
        let (name, username, imap_host, imap_port, smtp_host, smtp_port) = match state.validate() {
            Ok(values) => values,
            Err(message) => {
                if let Some(state) = self.account_edit_state.as_mut() {
                    state.error = Some(message);
                }
                return;
            }
        };
        let password = state.password.clone();
        let is_default = state.is_default;
        let editing = state.editing;

        let imap_secret_id = crate::setup::secret_service_id(&name, "imap");
        let smtp_secret_id = crate::setup::secret_service_id(&name, "smtp");

        if !password.is_empty() {
            for (protocol, secret_id) in [("IMAP", &imap_secret_id), ("SMTP", &smtp_secret_id)] {
                let label = format!("{name} {protocol} password");
                if let Err(error) =
                    crate::setup::store_secret(&label, secret_id, &username, &password)
                {
                    if let Some(state) = self.account_edit_state.as_mut() {
                        state.error = Some(format!("Could not store the password: {error}"));
                    }
                    return;
                }
            }
        } else if !editing {
            if let Some(state) = self.account_edit_state.as_mut() {
                state.error = Some("A password is required for a new account.".to_string());
            }
            return;
        }

        let config = AccountConfig {
            name: name.clone(),
            backend_kind: "imap".to_string(),
            provider_kind: "generic".to_string(),
            enabled: true,
            is_default,
            maildir_path: None,
            imap_host: Some(imap_host),
            imap_port: Some(imap_port),
            imap_security: Some("tls".to_string()),
            smtp_host: Some(smtp_host),
            smtp_port: Some(smtp_port),
            smtp_security: Some("tls".to_string()),
            auth_mode: Some("password".to_string()),
            username: Some(username),
            keyring_imap_secret_id: Some(imap_secret_id),
            keyring_smtp_secret_id: Some(smtp_secret_id),
        };

        let Some(ref conn) = self.db else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match crate::mail::account_store::upsert_account(conn, &config) {
            Ok(()) => {
                self.account_edit_state = None;
                self.view = View::AccountList;
                self.set_status(&format!("Saved account {name}."));
                self.load_accounts();
            }
            Err(error) => {
                if let Some(state) = self.account_edit_state.as_mut() {
                    state.error = Some(format!("Could not save the account: {error}"));
                }
            }
        }
    }
}
