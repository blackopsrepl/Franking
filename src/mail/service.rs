use std::sync::Arc;

use super::account_store::{self, AccountRecord};
use super::errors::{MailError, MailErrorKind, MailResult};
use super::maildir::MaildirService;
use super::model::MessageDocument;
use super::remote::ImapSmtpService;
use super::session::{IdleOutcome, SessionPool};
use super::store::{self, StoredMessage};
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
    /// Raw RFC 5322 bytes for a message. Backends preserve the raw-bytes
    /// boundary; parsing and caching happen one layer up.
    fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>>;
    fn read_message_content(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<MessageDocument> {
        super::mime::parse_message(self.read_message_raw(account, folder, id)?)
    }
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

    /// Block until a folder changes or the timeout elapses. Backends without
    /// push support report it as unsupported so callers can stop watching.
    fn idle_watch(
        &self,
        account: Option<&str>,
        folder: &str,
        timeout: std::time::Duration,
    ) -> MailResult<IdleOutcome> {
        let _ = (account, folder, timeout);
        Err(MailError::unsupported_feature(
            "IDLE is not supported by this backend",
        ))
    }
}

pub fn app_owned_remote_transport_available() -> bool {
    true
}

pub fn default_mail_service() -> Arc<dyn MailService> {
    Arc::new(RouterMailService::default())
}

#[derive(Debug, Default, Clone)]
pub struct RouterMailService {
    pool: Arc<SessionPool>,
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

/// Whether an error means the server is unreachable, so a cached result is
/// preferable to surfacing the failure.
fn is_offline(error: &MailError) -> bool {
    error.is_transport() || error.kind == MailErrorKind::BackendUnavailable
}

/// Serve a folder listing or search from the local store.
fn cached_envelopes(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    page: usize,
    page_size: usize,
    query: Option<&str>,
) -> MailResult<Vec<Envelope>> {
    let offset = page.saturating_sub(1) * page_size;
    let messages = match query.map(str::trim).filter(|query| !query.is_empty()) {
        Some(query) => store::search_messages(conn, Some(account), query, usize::MAX)
            .map_err(|err| MailError::config_invalid(err.to_string()))?
            .into_iter()
            .filter(|message| message.folder == folder)
            .skip(offset)
            .take(page_size)
            .collect::<Vec<_>>(),
        None => store::list_messages(conn, account, folder, page_size, offset)
            .map_err(|err| MailError::config_invalid(err.to_string()))?,
    };
    Ok(messages.iter().map(StoredMessage::to_envelope).collect())
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
        let record = self.choose_account(account)?;
        let result = match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => {
                service.list_envelopes(account, folder, page, page_size, query)
            }
            Route::Remote(service) => {
                service.list_envelopes(account, folder, page, page_size, query)
            }
        };

        match result {
            Ok(envelopes) => {
                let _ = self.with_db(|conn| {
                    for envelope in &envelopes {
                        store::upsert_envelope(
                            conn,
                            &StoredMessage::from_envelope(&record.name, folder, envelope),
                        )
                        .map_err(|err| MailError::config_invalid(err.to_string()))?;
                    }
                    Ok(())
                });
                Ok(envelopes)
            }
            Err(error) if is_offline(&error) => self.with_db(|conn| {
                cached_envelopes(conn, &record.name, folder, page, page_size, query)
            }),
            Err(error) => Err(error),
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

    fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        let record = self.choose_account(account)?;
        let result = match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => service.read_message_raw(account, folder, id),
            Route::Remote(service) => service.read_message_raw(account, folder, id),
        };

        match result {
            Ok(raw) => {
                if let Ok(document) = super::mime::parse_message(&raw) {
                    let stored = StoredMessage::from_document(
                        &record.name,
                        folder,
                        id,
                        None,
                        &[],
                        &document,
                        Some(raw.clone()),
                    );
                    let _ = self.with_db(|conn| {
                        store::upsert_message(conn, &stored)
                            .map_err(|err| MailError::config_invalid(err.to_string()))
                    });
                }
                Ok(raw)
            }
            Err(error) if is_offline(&error) => self.with_db(|conn| {
                store::get_message(conn, &record.name, folder, id)
                    .map_err(|err| MailError::config_invalid(err.to_string()))?
                    .and_then(|message| message.raw)
                    .ok_or(error)
            }),
            Err(error) => Err(error),
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

    fn idle_watch(
        &self,
        account: Option<&str>,
        folder: &str,
        timeout: std::time::Duration,
    ) -> MailResult<IdleOutcome> {
        let record = self.choose_account(account)?;
        if !record.backend_kind.eq_ignore_ascii_case("imap") {
            return Err(MailError::unsupported_feature(
                "IDLE is only available for IMAP accounts",
            ));
        }
        self.pool.idle_wait(&record, folder, timeout)
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

    #[test]
    fn offline_errors_are_classified_for_cache_fallback() {
        use super::is_offline;
        use crate::mail::MailError;

        assert!(is_offline(&MailError::transport_timeout("timed out")));
        assert!(is_offline(&MailError::connection_dropped("closed")));
        assert!(!is_offline(&MailError::imap_auth_rejected("bad password")));
        assert!(!is_offline(&MailError::config_invalid("missing host")));
    }

    #[test]
    fn cached_envelopes_serve_listings_and_search_offline() {
        use super::cached_envelopes;
        use crate::mail::store::{upsert_envelope, StoredMessage};
        use crate::mail::types::{Envelope, Sender};

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        for (uid, subject) in [("1", "Quarterly report"), ("2", "Lunch plans")] {
            let envelope = Envelope {
                id: uid.to_string(),
                flags: Vec::new(),
                subject: subject.to_string(),
                sender: Sender::Plain("alice@example.com".to_string()),
                date: "2026-04-13 09:00:00+00:00".to_string(),
            };
            upsert_envelope(
                &conn,
                &StoredMessage::from_envelope("work", "INBOX", &envelope),
            )
            .unwrap();
        }

        let listed = cached_envelopes(&conn, "work", "INBOX", 1, 10, None).unwrap();
        assert_eq!(listed.len(), 2);

        let searched = cached_envelopes(&conn, "work", "INBOX", 1, 10, Some("lunch")).unwrap();
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].id, "2");

        let other_folder = cached_envelopes(&conn, "work", "Sent", 1, 10, None).unwrap();
        assert!(other_folder.is_empty());
    }
}
