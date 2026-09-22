/*! Mailbox lifecycle operations (create, rename, delete). */

use crate::mail::errors::MailResult;

use super::model::ImapSmtpService;
use super::next::{self, FlagChange};

impl ImapSmtpService {
    /// Create a new mailbox.
    pub fn create_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool
            .with_client(&self.account, |client| next::create_folder(client, name))
    }

    /// Rename a mailbox.
    pub fn rename_folder(&self, account: Option<&str>, from: &str, to: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool.with_client(&self.account, |client| {
            next::rename_folder(client, from, to)
        })
    }

    /// Permanently remove every message in a mailbox.
    pub fn empty_folder(&self, account: Option<&str>, name: &str) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let deleted = next::flag_of("deleted")?;
        self.pool.with_client(&self.account, |client| {
            next::select(client, name)?;
            next::store_flags(client, "1:*", FlagChange::Add, vec![deleted.clone()], true)?;
            next::expunge(client)?;
            Ok(format!("Emptied {name}."))
        })
    }

    /// Delete a mailbox.
    pub fn delete_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool
            .with_client(&self.account, |client| next::delete_folder(client, name))
    }

    /// The mailboxes the account is subscribed to.
    pub fn list_subscribed_folders(&self, account: Option<&str>) -> MailResult<Vec<String>> {
        self.ensure_requested_account(account)?;
        let folders = self
            .pool
            .with_client(&self.account, next::list_subscribed)?;
        Ok(folders.into_iter().map(|folder| folder.name).collect())
    }

    /// Subscribe to a mailbox.
    pub fn subscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool
            .with_client(&self.account, |client| next::subscribe(client, name))
    }

    /// Unsubscribe from a mailbox.
    pub fn unsubscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool
            .with_client(&self.account, |client| next::unsubscribe(client, name))
    }
}
