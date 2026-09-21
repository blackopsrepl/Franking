/*! `MailService` implementation delegating to the routed backend. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::IdleOutcome;
use crate::mail::store;
use crate::mail::types::{Account, Envelope, Folder};

use super::cache::{
    cache_envelopes, cache_message, cached_envelopes, is_offline, record_sync_cursor,
};
use super::router::{Route, RouterMailService};
use super::service_trait::MailService;

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
                    cache_envelopes(conn, &record.name, folder, &envelopes)
                        .map_err(|err| MailError::config_invalid(err.to_string()))
                });
                if let Ok((uid_validity, uid_next)) =
                    self.folder_sync_cursor(Some(&record.name), folder)
                {
                    let _ = self.with_db(|conn| {
                        record_sync_cursor(conn, &record.name, folder, uid_validity, uid_next)
                            .map_err(|err| MailError::config_invalid(err.to_string()))
                    });
                }
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
                if let Ok(document) = crate::mail::mime::parse_message(&raw) {
                    let _ = self.with_db(|conn| {
                        cache_message(conn, &record.name, folder, id, &raw, &document)
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
        let record = self.choose_account(account)?;
        match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => service.delete_message(account, folder, id),
            Route::Remote(service) => service.delete_message(account, folder, id),
        }?;
        let _ = self.with_db(|conn| {
            store::delete_message(conn, &record.name, folder, id)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
    }

    fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        let record = self.choose_account(account)?;
        match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => service.move_message(account, folder, target, id),
            Route::Remote(service) => service.move_message(account, folder, target, id),
        }?;
        let _ = self.with_db(|conn| {
            store::move_message(conn, &record.name, folder, id, target)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
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
        let record = self.choose_account(account)?;
        match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => service.flag_add(account, folder, id, flag),
            Route::Remote(service) => service.flag_add(account, folder, id, flag),
        }?;
        let _ = self.with_db(|conn| {
            store::set_flag(conn, &record.name, folder, id, flag, true)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
    }

    fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        let record = self.choose_account(account)?;
        match self.route_account(Some(&record.name))? {
            Route::Maildir(service) => service.flag_remove(account, folder, id, flag),
            Route::Remote(service) => service.flag_remove(account, folder, id, flag),
        }?;
        let _ = self.with_db(|conn| {
            store::set_flag(conn, &record.name, folder, id, flag, false)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
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

    fn save_draft(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.save_draft(account, template),
            Route::Remote(service) => service.save_draft(account, template),
        }
    }

    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.folder_unread(account, folder),
            Route::Remote(service) => service.folder_unread(account, folder),
        }
    }

    fn folder_sync_cursor(
        &self,
        account: Option<&str>,
        folder: &str,
    ) -> MailResult<(Option<u32>, Option<u32>)> {
        match self.route_account(account)? {
            Route::Maildir(service) => service.folder_sync_cursor(account, folder),
            Route::Remote(service) => service.folder_sync_cursor(account, folder),
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
