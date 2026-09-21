/*! IMAP/SMTP service type and account guards. */

use std::sync::Arc;

use crate::mail::account_store::AccountRecord;
use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::SessionPool;

#[derive(Debug, Clone)]
pub struct ImapSmtpService {
    pub(super) account: AccountRecord,
    pub(super) pool: Arc<SessionPool>,
}

impl ImapSmtpService {
    pub fn new(account: AccountRecord, pool: Arc<SessionPool>) -> Self {
        Self { account, pool }
    }

    pub(super) fn ensure_requested_account(&self, account: Option<&str>) -> MailResult<()> {
        if let Some(name) = account {
            self.ensure_account(name)?;
        }
        Ok(())
    }

    pub(super) fn ensure_account(&self, account: &str) -> MailResult<()> {
        if self.account.name == account {
            Ok(())
        } else {
            Err(MailError::account_not_found(account.to_string()))
        }
    }

    pub(super) fn username(&self) -> MailResult<&str> {
        self.account
            .username
            .as_deref()
            .ok_or_else(|| MailError::config_invalid("account username is missing"))
    }

    pub(super) fn default_from_header(&self) -> Option<String> {
        self.account.username.as_ref().cloned()
    }
}
