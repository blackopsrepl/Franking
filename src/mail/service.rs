use std::sync::Arc;

use super::account_store::{self, AccountRecord};
use super::errors::{MailError, MailResult};
use super::maildir::MaildirService;
use super::model::MessageDocument;
use super::remote::ImapSmtpService;
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
    ) -> MailResult<MessageDocument>;
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
    true
}

pub fn default_mail_service() -> Arc<dyn MailService> {
    Arc::new(RouterMailService)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RouterMailService;

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

        if record.backend_kind.eq_ignore_ascii_case("imap") {
            return Ok(Route::Remote(Box::new(ImapSmtpService::new(record))));
        }

        Err(MailError::unsupported_feature(format!(
            "unsupported backend kind: {}",
            record.backend_kind
        )))
    }

    fn merged_accounts(&self) -> MailResult<Vec<Account>> {
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
            Route::Remote(service) => service.probe_account(account),
        }
    }

    fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.list_folders(account),
            Route::Remote(service) => service.list_folders(account),
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
            Route::Remote(service) => {
                service.list_envelopes(account, folder, page, page_size, query)
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
            Route::Remote(service) => service.list_envelopes_threaded(account, folder, query),
        }
    }

    fn read_message_content(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<MessageDocument> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.read_message_content(account, folder, id),
            Route::Remote(service) => service.read_message_content(account, folder, id),
        }
    }

    fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.delete_message(account, folder, id),
            Route::Remote(service) => service.delete_message(account, folder, id),
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
            Route::Remote(service) => service.move_message(account, folder, target, id),
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
            Route::Remote(service) => service.copy_message(account, folder, target, id),
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
            Route::Remote(service) => service.flag_add(account, folder, id, flag),
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
            Route::Remote(service) => service.flag_remove(account, folder, id, flag),
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
            Route::Remote(service) => service.download_attachments(account, folder, id),
        }
    }

    fn template_write(&self, account: Option<&str>) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_write(account),
            Route::Remote(service) => service.template_write(account),
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
            Route::Remote(service) => service.template_reply(account, folder, id, all),
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
            Route::Remote(service) => service.template_forward(account, folder, id),
        }
    }

    fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.template_send(account, template),
            Route::Remote(service) => service.template_send(account, template),
        }
    }
}

enum Route {
    Maildir(MaildirService),
    Remote(Box<ImapSmtpService>),
}

#[cfg(test)]
mod tests {
    use super::sort_account_records;
    use crate::mail::account_store::AccountRecord;

    fn stored_account(name: &str, backend_kind: &str) -> AccountRecord {
        AccountRecord {
            name: name.to_string(),
            backend_kind: backend_kind.to_string(),
            provider_kind: "generic".to_string(),
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

    #[test]
    fn stored_imap_accounts_are_routable() {
        assert!(stored_account("work", "imap").is_routable());
        assert!(stored_account("test", "maildir").is_routable());
    }

    #[test]
    fn account_sort_keeps_default_remote_ahead_of_local_test_account() {
        let mut accounts = vec![
            stored_account("test", "maildir"),
            AccountRecord {
                is_default: true,
                ..stored_account("work", "imap")
            },
        ];

        sort_account_records(&mut accounts);

        assert_eq!(accounts[0].name, "work");
        assert_eq!(accounts[1].name, "test");
    }
}
