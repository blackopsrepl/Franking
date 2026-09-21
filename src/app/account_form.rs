/*! Account add/edit form commands. */

use crate::account_edit::{AccountEditState, AccountField};
use crate::keys::View;
use crate::mail::account_store::AccountConfig;

use super::model::App;

impl App {
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
            match state.focused {
                AccountField::Default => state.toggle_default(),
                AccountField::Auth => state.cycle_auth_mode(),
                _ => {}
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
        let auth_mode = state.auth_mode;
        let client_id = state.client_id.clone();
        let client_secret = state.client_secret.clone();

        if let Some(provider_kind) = auth_mode.provider_kind() {
            self.start_oauth_authorization(
                &name,
                &username,
                provider_kind,
                &client_id,
                &client_secret,
                is_default,
            );
            return;
        }

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

    /// Finish an OAuth authorization attempt.
    pub(crate) fn handle_oauth_authorized(
        &mut self,
        result: Result<String, crate::mail::MailError>,
    ) {
        self.loading = false;
        match result {
            Ok(message) => {
                self.account_edit_state = None;
                self.view = View::AccountList;
                self.set_status(&message);
                self.load_accounts();
            }
            Err(error) => self.set_error(&format!("OAuth: {error}")),
        }
    }
}
