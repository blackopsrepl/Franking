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
                return account_store::get_account(conn, name)
                    .map_err(|err| MailError::config_invalid(err.to_string()))?
                    .ok_or_else(|| MailError::account_not_found(name.to_string()));
            }

            let accounts = account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?;
            account_store::preferred_account(&accounts)
                .cloned()
                .ok_or_else(|| MailError::account_not_found("no configured account".to_string()))
        })
    }

    fn route_account(&self, account: Option<&str>) -> MailResult<Route> {
        let record = self.choose_account(account)?;
        if record.backend_kind.eq_ignore_ascii_case("maildir") {
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

        Ok(Route::Legacy(record.name))
    }

    fn merged_accounts(&self) -> MailResult<Vec<Account>> {
        self.with_db(|conn| {
            let mut merged = HashMap::new();
            for account in account_store::list_accounts(conn)
                .map_err(|err| MailError::config_invalid(err.to_string()))?
            {
                merged.insert(account.name.clone(), account.to_account());
            }

            if let Ok(legacy_accounts) = self.legacy.list_accounts() {
                for account in &legacy_accounts {
                    let _ = account_store::upsert_legacy_account(conn, account);
                }
                for account in legacy_accounts {
                    merged.insert(account.name.clone(), account);
                }
            }

            let mut accounts = merged.into_values().collect::<Vec<_>>();
            sort_accounts(&mut accounts);
            Ok(accounts)
        })
    }
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
