/*! Per-account session reuse and reconnect. */

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::mail::account_store::AccountRecord;
use crate::mail::errors::{MailError, MailResult};

use super::connection::{
    idle_wait, select_mailbox, ConnectedImapSession, IdleOutcome, ImapSession,
};
use super::credentials::{CredentialProvider, KeyringCredentials};

#[derive(Debug)]
pub struct SessionPool {
    credentials: Arc<dyn CredentialProvider>,
    sessions: Mutex<HashMap<String, ImapSession>>,
    watchers: Mutex<HashMap<String, ImapSession>>,
}

impl Default for SessionPool {
    fn default() -> Self {
        Self::with_credentials(Arc::new(KeyringCredentials))
    }
}

impl SessionPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_credentials(credentials: Arc<dyn CredentialProvider>) -> Self {
        Self {
            credentials,
            sessions: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn credentials(&self) -> &Arc<dyn CredentialProvider> {
        &self.credentials
    }

    pub fn with_connection<T>(
        &self,
        account: &AccountRecord,
        mut operation: impl FnMut(&mut ConnectedImapSession) -> MailResult<T>,
    ) -> MailResult<T> {
        let mut attempts = 0;
        loop {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| MailError::other("session pool lock was poisoned"))?;

            if !sessions.contains_key(&account.name) {
                sessions.insert(
                    account.name.clone(),
                    ImapSession::connect(account, self.credentials.as_ref())?,
                );
            }

            let session = sessions
                .get_mut(&account.name)
                .expect("session inserted immediately above");

            match operation(session.connection()) {
                Ok(value) => return Ok(value),
                Err(error) if error.is_transport() => {
                    sessions.remove(&account.name);
                    drop(sessions);
                    if attempts == 0 {
                        attempts += 1;
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub fn idle_wait(
        &self,
        account: &AccountRecord,
        folder: &str,
        timeout: Duration,
    ) -> MailResult<IdleOutcome> {
        // A dedicated connection is used for IDLE so a long wait never blocks
        // the operation sessions for the same account.
        let mut watchers = self
            .watchers
            .lock()
            .map_err(|_| MailError::other("session pool lock was poisoned"))?;

        if !watchers.contains_key(&account.name) {
            watchers.insert(
                account.name.clone(),
                ImapSession::connect(account, self.credentials.as_ref())?,
            );
        }
        let session = watchers
            .get_mut(&account.name)
            .expect("session inserted immediately above");

        let result = (|| {
            select_mailbox(session.connection(), folder)?;
            idle_wait(session.connection(), timeout)
        })();

        if let Err(error) = &result {
            if error.is_transport() {
                watchers.remove(&account.name);
            }
        }

        result
    }

    pub fn invalidate(&self, account: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(account);
        }
    }
}

impl SessionPool {
    /// Run an operation against the pooled session, exposing capabilities.
    pub fn with_session<T>(
        &self,
        account: &AccountRecord,
        mut operation: impl FnMut(&mut ImapSession) -> MailResult<T>,
    ) -> MailResult<T> {
        let mut attempts = 0;
        loop {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| MailError::other("session pool lock was poisoned"))?;

            if !sessions.contains_key(&account.name) {
                sessions.insert(
                    account.name.clone(),
                    ImapSession::connect(account, self.credentials.as_ref())?,
                );
            }

            let session = sessions
                .get_mut(&account.name)
                .expect("session inserted immediately above");

            match operation(session) {
                Ok(value) => return Ok(value),
                Err(error) if error.is_transport() => {
                    sessions.remove(&account.name);
                    drop(sessions);
                    if attempts == 0 {
                        attempts += 1;
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }
    }
}
