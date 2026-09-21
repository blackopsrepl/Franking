/*! Account routing and the default service constructor. */

use std::sync::Arc;

use crate::db;

use super::super::account_store::{self, AccountRecord};
use super::super::errors::{MailError, MailResult};
use super::super::maildir::MaildirService;
use super::super::remote::ImapSmtpService;
use super::super::session::SessionPool;
use super::super::types::{sort_accounts, Account};
use super::service_trait::MailService;

pub fn default_mail_service() -> Arc<dyn MailService> {
    Arc::new(RouterMailService::default())
}

#[derive(Debug, Default, Clone)]
pub struct RouterMailService {
    pub(super) pool: Arc<SessionPool>,
}

impl RouterMailService {
    pub(super) fn with_db<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection) -> MailResult<T>,
    ) -> MailResult<T> {
        let conn = db::open().map_err(|err| MailError::config_invalid(err.to_string()))?;
        account_store::seed_defaults(&conn)
            .map_err(|err| MailError::config_invalid(err.to_string()))?;
        f(&conn)
    }

    pub(super) fn choose_account(&self, account: Option<&str>) -> MailResult<AccountRecord> {
        self.with_db(|conn| {
            if let Some(name) = account {
                return account_store::get_account(conn, name)
                    .map_err(|err| MailError::config_invalid(err.to_string()))?
                    .filter(AccountRecord::is_routable)
                    .ok_or_else(|| MailError::account_not_found(name.to_string()));
            }

            let mut accounts = account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?
                .into_iter()
                .filter(AccountRecord::is_routable)
                .collect::<Vec<_>>();
            sort_account_records(&mut accounts);
            accounts
                .into_iter()
                .next()
                .ok_or_else(|| MailError::account_not_found("no configured account".to_string()))
        })
    }

    pub(super) fn route_account(&self, account: Option<&str>) -> MailResult<Route> {
        let record = self.choose_account(account)?;
        if record.is_maildir() {
            let path = record.maildir_path.ok_or_else(|| {
                MailError::config_invalid(format!(
                    "account {} is missing a maildir path",
                    record.name
                ))
            })?;
            return Ok(Route::Maildir(
                MaildirService::new(record.name, path).with_default(record.is_default),
            ));
        }

        if record.backend_kind.eq_ignore_ascii_case("imap") {
            return Ok(Route::Remote(Box::new(ImapSmtpService::new(
                record,
                self.pool.clone(),
            ))));
        }

        Err(MailError::unsupported_feature(format!(
            "unsupported backend kind: {}",
            record.backend_kind
        )))
    }

    pub(super) fn merged_accounts(&self) -> MailResult<Vec<Account>> {
        self.with_db(|conn| {
            let mut accounts = account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?
                .into_iter()
                .filter(AccountRecord::is_routable)
                .map(|record| record.to_account())
                .collect::<Vec<_>>();
            sort_accounts(&mut accounts);
            Ok(accounts)
        })
    }
}

pub(super) fn sort_account_records(accounts: &mut [AccountRecord]) {
    accounts.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then_with(|| left.is_maildir().cmp(&right.is_maildir()))
            .then_with(|| left.name.cmp(&right.name))
    });
}

/// Whether an error means the server is unreachable, so a cached result is
/// preferable to surfacing the failure.
pub(super) enum Route {
    Maildir(MaildirService),
    Remote(Box<ImapSmtpService>),
}
