/*! Per-account client reuse, reconnect, and IDLE. */

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::mail::account_store::AccountRecord;
use crate::mail::errors::{MailError, MailResult};

use super::client::ImapClient;
use super::client_connect::open_imap_client;
use super::credentials::{CredentialProvider, KeyringCredentials};
use super::idle::{self, IdleOutcome};
use super::transport::ReadWrite;

/// An app-owned IMAP client over any transport.
pub type CodecClient = ImapClient<Box<dyn ReadWrite>>;

#[derive(Debug)]
pub struct SessionPool {
    credentials: Arc<dyn CredentialProvider>,
    clients: Mutex<HashMap<String, CodecClient>>,
    watchers: Mutex<HashMap<String, CodecClient>>,
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
            clients: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn credentials(&self) -> &Arc<dyn CredentialProvider> {
        &self.credentials
    }

    /// Run an operation against the pooled app-owned client, reconnecting once
    /// when a transport failure discards it.
    pub fn with_client<T>(
        &self,
        account: &AccountRecord,
        mut operation: impl FnMut(&mut CodecClient) -> MailResult<T>,
    ) -> MailResult<T> {
        self.with_cached(&self.clients, account, &mut operation)
    }

    /// Wait for a change in a folder over a dedicated IDLE connection, so a
    /// long wait never blocks the operation client for the same account.
    pub fn idle_wait(
        &self,
        account: &AccountRecord,
        folder: &str,
        timeout: Duration,
    ) -> MailResult<IdleOutcome> {
        self.with_cached(&self.watchers, account, &mut |client| {
            idle::select(client, folder)?;
            idle::wait(client, timeout)
        })
    }

    pub fn invalidate(&self, account: &str) {
        if let Ok(mut clients) = self.clients.lock() {
            clients.remove(account);
        }
        if let Ok(mut watchers) = self.watchers.lock() {
            watchers.remove(account);
        }
    }

    fn with_cached<T>(
        &self,
        cache: &Mutex<HashMap<String, CodecClient>>,
        account: &AccountRecord,
        operation: &mut impl FnMut(&mut CodecClient) -> MailResult<T>,
    ) -> MailResult<T> {
        let mut attempts = 0;
        loop {
            let mut clients = cache
                .lock()
                .map_err(|_| MailError::other("session pool lock was poisoned"))?;

            if !clients.contains_key(&account.name) {
                clients.insert(
                    account.name.clone(),
                    open_imap_client(account, self.credentials.as_ref())?,
                );
            }

            let client = clients
                .get_mut(&account.name)
                .expect("client inserted immediately above");

            match operation(client) {
                Ok(value) => return Ok(value),
                Err(error) if error.is_transport() => {
                    clients.remove(&account.name);
                    drop(clients);
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
