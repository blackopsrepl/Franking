/*! OAuth authorization for a new account, off the UI thread. */

use crate::mail::oauth::account::{self, OAuthAccountRequest};

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Run the browser flow and persist the account.
    pub fn authorize_oauth_account(&self, account: String, request: OAuthAccountRequest) {
        let label = account.clone();
        self.spawn(
            move |_service| -> Result<String, anyhow::Error> {
                let authorization = account::authorize(&request)?;
                account::store(&request, &authorization)?;
                Ok(format!(
                    "Authorized {account} via OAuth; the account is ready."
                ))
            },
            move |result| {
                WorkerResult::OAuthAuthorized(
                    label,
                    result.map_err(|error| crate::mail::MailError::other(error.to_string())),
                )
            },
        );
    }
}
