use std::collections::HashMap;
use std::sync::Arc;

use super::account_store::{self, AccountRecord};
use super::errors::{MailError, MailResult};
use super::himalaya::HimalayaService;
use super::maildir::MaildirService;
use super::message::MessageContent;
use super::types::{sort_accounts, Account, Envelope, Folder};
use crate::db;

pub trait MailService: Send + Sync {
    fn list_accounts(&self) -> MailResult<Vec<Account>>;
    fn probe_account(&self, account: &str) -> MailResult<()>;
    fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>>;
    fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>>;
    fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>>;
    fn read_message_content(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<MessageContent>;
    fn read_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<String> {
        self.read_message_content(account, folder, id)
            .map(|message| message.render_for_legacy_view(78))
    }
    fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()>;
    fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()>;
    fn copy_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()>;
    fn flag_add(&self, account: Option<&str>, folder: &str, id: &str, flag: &str)
        -> MailResult<()>;
    fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()>;
    fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String>;
    fn template_write(&self, account: Option<&str>) -> MailResult<String>;
    fn template_reply(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String>;
    fn template_forward(&self, account: Option<&str>, folder: &str, id: &str)
        -> MailResult<String>;
    fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String>;
}

pub fn app_owned_remote_transport_available() -> bool {
    false
}

pub fn default_mail_service() -> Arc<dyn MailService> {
    Arc::new(RouterMailService::default())
}

#[derive(Debug, Default, Clone)]
pub struct RouterMailService {
    legacy: HimalayaService,
}

impl RouterMailService {
    fn with_db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> MailResult<T>) -> MailResult<T> {
        let conn = db::open().map_err(|err| MailError::config_invalid(err.to_string()))?;
        account_store::seed_defaults(&conn)
            .map_err(|err| MailError::config_invalid(err.to_string()))?;
        f(&conn)
    }

    fn choose_account(&self, account: Option<&str>) -> MailResult<AccountRecord> {
        self.with_db(|conn| {
            if let Some(name) = account {
                let stored_record = account_store::get_account(conn, name)
                    .map_err(|err| MailError::config_invalid(err.to_string()))?;
                let legacy_accounts = match self.legacy.list_accounts() {
                    Ok(accounts) => accounts,
                    Err(_) if stored_record.is_some() => Vec::new(),
                    Err(error) => return Err(error),
                };

                return resolve_named_account(stored_record, &legacy_accounts, name)
                    .ok_or_else(|| MailError::account_not_found(name.to_string()));
            }

            let stored_accounts = account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?;
            let legacy_accounts = self.legacy.list_accounts().unwrap_or_default();
            let mut accounts = merge_visible_records(stored_accounts, legacy_accounts);
            sort_account_records(&mut accounts);

            accounts
                .into_iter()
                .next()
                .ok_or_else(|| MailError::account_not_found("no configured account".to_string()))
        })
    }

    fn route_account(&self, account: Option<&str>) -> MailResult<Route> {
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

        if record.is_legacy() {
            return Ok(Route::Legacy(record.name));
        }

        Err(MailError::unsupported_feature(format!(
            "account {} is stored as an app-owned remote account, but native IMAP/SMTP transport is not available in this build",
            record.name
        )))
    }

    fn merged_accounts(&self) -> MailResult<Vec<Account>> {
        self.with_db(|conn| {
            let stored_accounts = account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?;
            let legacy_accounts = self.legacy.list_accounts().unwrap_or_default();
            Ok(merge_visible_accounts(stored_accounts, legacy_accounts))
        })
    }
}

fn resolve_named_account(
    stored_record: Option<AccountRecord>,
    legacy_accounts: &[Account],
    name: &str,
) -> Option<AccountRecord> {
    match stored_record {
        Some(record) if record.is_routable() => Some(record),
        Some(record) => legacy_accounts
            .iter()
            .find(|account| account.name == name)
            .map(AccountRecord::from_legacy_account)
            .or(Some(record)),
        None => legacy_accounts
            .iter()
            .find(|account| account.name == name)
            .map(AccountRecord::from_legacy_account),
    }
}

fn merge_visible_accounts(
    stored_accounts: Vec<AccountRecord>,
    legacy_accounts: Vec<Account>,
) -> Vec<Account> {
    let mut accounts = merge_visible_records(stored_accounts, legacy_accounts)
        .into_iter()
        .map(|record| record.to_account())
        .collect::<Vec<_>>();
    sort_accounts(&mut accounts);
    accounts
}

fn merge_visible_records(
    stored_accounts: Vec<AccountRecord>,
    legacy_accounts: Vec<Account>,
) -> Vec<AccountRecord> {
    let mut merged = HashMap::new();
    for account in legacy_accounts {
        merged.insert(
            account.name.clone(),
            AccountRecord::from_legacy_account(&account),
        );
    }
    for account in stored_accounts
        .into_iter()
        .filter(AccountRecord::is_routable)
    {
        merged.insert(account.name.clone(), account);
    }
    merged.into_values().collect()
}

fn sort_account_records(accounts: &mut [AccountRecord]) {
    accounts.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then_with(|| left.is_maildir().cmp(&right.is_maildir()))
            .then_with(|| left.name.cmp(&right.name))
    });
}

impl MailService for RouterMailService {
    fn list_accounts(&self) -> MailResult<Vec<Account>> {
        self.merged_accounts()
    }

    fn probe_account(&self, account: &str) -> MailResult<()> {
        match self.route_account(Some(account))? {
            Route::Maildir(service) => service.probe_account(account),
            Route::Legacy(name) => self.legacy.probe_account(&name),
        }
    }

    fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.list_folders(account),
            Route::Legacy(name) => self.legacy.list_folders(Some(&name)),
        }
    }

    fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        match self.route_account(account)? {
            Route::Maildir(service) => {
                service.list_envelopes(account, folder, page, page_size, query)
            }
            Route::Legacy(name) => {
                self.legacy
                    .list_envelopes(Some(&name), folder, page, page_size, query)
            }
        }
    }

    fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.list_envelopes_threaded(account, folder, query),
            Route::Legacy(name) => self
                .legacy
                .list_envelopes_threaded(Some(&name), folder, query),
        }
    }

    fn read_message_content(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<MessageContent> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.read_message_content(account, folder, id),
            Route::Legacy(name) => self.legacy.read_message_content(Some(&name), folder, id),
        }
    }

    fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.delete_message(account, folder, id),
            Route::Legacy(name) => self.legacy.delete_message(Some(&name), folder, id),
        }
    }

    fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.move_message(account, folder, target, id),
            Route::Legacy(name) => self.legacy.move_message(Some(&name), folder, target, id),
        }
    }

    fn copy_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.copy_message(account, folder, target, id),
            Route::Legacy(name) => self.legacy.copy_message(Some(&name), folder, target, id),
        }
    }

    fn flag_add(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.flag_add(account, folder, id, flag),
            Route::Legacy(name) => self.legacy.flag_add(Some(&name), folder, id, flag),
        }
    }

    fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.flag_remove(account, folder, id, flag),
            Route::Legacy(name) => self.legacy.flag_remove(Some(&name), folder, id, flag),
        }
    }

    fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.download_attachments(account, folder, id),
            Route::Legacy(name) => self.legacy.download_attachments(Some(&name), folder, id),
        }
    }

    fn template_write(&self, account: Option<&str>) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_write(account),
            Route::Legacy(name) => self.legacy.template_write(Some(&name)),
        }
    }

    fn template_reply(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_reply(account, folder, id, all),
            Route::Legacy(name) => self.legacy.template_reply(Some(&name), folder, id, all),
        }
    }

    fn template_forward(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_forward(account, folder, id),
            Route::Legacy(name) => self.legacy.template_forward(Some(&name), folder, id),
        }
    }

    fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_send(account, template),
            Route::Legacy(name) => self.legacy.template_send(Some(&name), template),
        }
    }
}

enum Route {
    Maildir(MaildirService),
    Legacy(String),
}

#[cfg(test)]
mod tests {
    use super::{merge_visible_accounts, resolve_named_account};
    use crate::mail::account_store::AccountRecord;
    use crate::mail::types::Account;

    fn stored_account(name: &str, backend_kind: &str, provider_kind: &str) -> AccountRecord {
        AccountRecord {
            name: name.to_string(),
            backend_kind: backend_kind.to_string(),
            provider_kind: provider_kind.to_string(),
            enabled: true,
            is_default: false,
            maildir_path: None,
            imap_host: Some("imap.example.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: Some(465),
            smtp_security: Some("tls".to_string()),
            auth_mode: Some("password".to_string()),
            username: Some("alice@example.com".to_string()),
            keyring_imap_secret_id: Some("solverforge-mail/work/imap".to_string()),
            keyring_smtp_secret_id: Some("solverforge-mail/work/smtp".to_string()),
        }
    }

    fn legacy_account(name: &str) -> Account {
        Account {
            name: name.to_string(),
            backend: "imap".to_string(),
            default: false,
        }
    }

    #[test]
    fn merged_accounts_keep_working_legacy_entry_when_stored_remote_is_unroutable() {
        let accounts = merge_visible_accounts(
            vec![stored_account("work", "imap", "generic")],
            vec![legacy_account("work")],
        );

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "work");
        assert_eq!(accounts[0].backend, "imap");
    }

    #[test]
    fn resolve_named_account_falls_back_to_legacy_when_stored_remote_is_unroutable() {
        let resolved = resolve_named_account(
            Some(stored_account("work", "imap", "generic")),
            &[legacy_account("work")],
            "work",
        )
        .expect("account should resolve");

        assert!(resolved.is_legacy());
    }

    #[test]
    fn resolve_named_account_keeps_stored_maildir_when_it_is_routable() {
        let mut stored = stored_account("test", "maildir", "custom");
        stored.maildir_path = Some("/tmp/test-maildir".into());

        let resolved =
            resolve_named_account(Some(stored.clone()), &[legacy_account("test")], "test")
                .expect("account should resolve");

        assert_eq!(resolved, stored);
    }
}
