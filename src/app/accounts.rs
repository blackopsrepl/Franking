/*! Account list management: set default and delete. */

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

    /// Fill the account form from discovery results.
    pub(crate) fn apply_discovered(
        &mut self,
        config: Option<crate::mail::autoconfig::DiscoveredConfig>,
    ) {
        let Some(config) = config else {
            self.set_status("No automatic settings found; enter them by hand.");
            return;
        };
        let Some(state) = self.account_edit_state.as_mut() else {
            return;
        };
        state.imap_host = config.imap_host.clone();
        state.imap_port = config.imap_port.to_string();
        state.smtp_host = config.smtp_host.clone();
        state.smtp_port = config.smtp_port.to_string();
        if state.username.trim().is_empty() {
            state.username = config.username.clone();
        }
        let source = match config.source {
            crate::mail::autoconfig::DiscoverySource::Preset => "built-in provider settings",
            crate::mail::autoconfig::DiscoverySource::MozillaAutoconfig => "Mozilla autoconfig",
            crate::mail::autoconfig::DiscoverySource::Autodiscover => "Autodiscover",
            crate::mail::autoconfig::DiscoverySource::Srv => "DNS SRV records",
        };
        self.set_status(&format!("Filled settings from {source}."));
    }

    /// Begin the OAuth browser flow for the form's values.
    pub(crate) fn start_oauth_authorization(
        &mut self,
        account: &str,
        username: &str,
        provider_kind: &str,
        client_id: &str,
        client_secret: &str,
        is_default: bool,
    ) {
        if client_id.trim().is_empty() {
            if let Some(state) = self.account_edit_state.as_mut() {
                state.error = Some("An OAuth client ID is required.".to_string());
            }
            return;
        }
        if !username.contains('@') {
            if let Some(state) = self.account_edit_state.as_mut() {
                state.error = Some("An email address is required for OAuth.".to_string());
            }
            return;
        }

        let existing = self
            .db
            .as_ref()
            .and_then(|conn| crate::mail::account_store::get_oauth_state(conn, account).ok())
            .flatten();
        let request = crate::mail::oauth::account::OAuthAccountRequest {
            account: account.to_string(),
            username: username.to_string(),
            client_id: client_id.trim().to_string(),
            client_secret: (!client_secret.trim().is_empty())
                .then(|| client_secret.trim().to_string()),
            provider_kind: provider_kind.to_string(),
            is_default,
            existing_client_secret: None,
            existing_refresh_token_ref: existing
                .as_ref()
                .map(|state| state.refresh_token_ref.clone()),
            existing_client_secret_ref: existing
                .as_ref()
                .and_then(|state| state.client_secret_ref.clone()),
        };

        self.loading = true;
        self.set_status("Waiting for authorization in the browser...");
        self.worker
            .authorize_oauth_account(account.to_string(), request);
    }

    /// Ask the worker to discover provider settings for the typed login.
    pub(crate) fn discover_account_settings(&mut self) {
        let Some(state) = self.account_edit_state.as_ref() else {
            return;
        };
        let identifier = state.username.trim().to_string();
        if identifier.is_empty() || !identifier.contains('@') {
            if let Some(state) = self.account_edit_state.as_mut() {
                state.error = Some("Enter an email address to auto-detect settings.".to_string());
            }
            return;
        }
        self.loading = true;
        self.worker.discover_provider(identifier);
    }
}
