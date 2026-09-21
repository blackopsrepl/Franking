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
}
