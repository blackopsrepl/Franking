/*! `MailService` implementation delegating to the routed backend. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::IdleOutcome;
use crate::mail::store;
use crate::mail::types::{Account, Envelope, Folder};

use super::router::{route, RouterMailService};
use super::service_options::SendOptions;
use super::service_trait::MailService;

impl MailService for RouterMailService {
    fn list_accounts(&self) -> MailResult<Vec<Account>> {
        self.merged_accounts()
    }

    fn probe_account(&self, account: &str) -> MailResult<()> {
        route!(self, Some(account), service => service.probe_account(account))
    }

    fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        route!(self, account, service => service.list_folders(account))
    }

    fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes_page(account, folder, page, page_size, query, None)
    }

    fn list_envelopes_sorted(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
        order: crate::mail::sort::SortOrder,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes_page(account, folder, page, page_size, query, Some(order))
    }

    fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        route!(self, account, service => service.list_envelopes_threaded(account, folder, query))
    }

    fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        super::read::read_message_raw(self, account, folder, id)
    }

    fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        let record = self.choose_account(account)?;
        route!(self, Some(&record.name), service => service.delete_message(account, folder, id))?;
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
        route!(self, Some(&record.name), service => service.move_message(account, folder, target, id))?;
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
        route!(self, account, service => service.copy_message(account, folder, target, id))
    }

    fn flag_add(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        let record = self.choose_account(account)?;
        route!(self, Some(&record.name), service => service.flag_add(account, folder, id, flag))?;
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
        route!(self, Some(&record.name), service => service.flag_remove(account, folder, id, flag))?;
        let _ = self.with_db(|conn| {
            store::set_flag(conn, &record.name, folder, id, flag, false)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
    }

    fn download_attachments_zip(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        route!(self, account, service => service.download_attachments_zip(account, folder, id))
    }

    fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        route!(self, account, service => service.download_attachments(account, folder, id))
    }

    fn template_write(&self, account: Option<&str>) -> MailResult<String> {
        route!(self, account, service => service.template_write(account))
    }

    fn template_reply(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        route!(self, account, service => service.template_reply(account, folder, id, all))
    }

    fn template_forward(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        route!(self, account, service => service.template_forward(account, folder, id))
    }

    fn template_send(
        &self,
        account: Option<&str>,
        template: &str,
        options: &SendOptions,
    ) -> MailResult<String> {
        route!(self, account, service => service.template_send(account, template, options))
    }

    fn save_draft(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        route!(self, account, service => service.save_draft(account, template))
    }

    fn sync_folder(&self, account: Option<&str>, folder: &str) -> MailResult<Vec<Envelope>> {
        self.sync_folder_cached(account, folder)
    }

    fn draft_template(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<String> {
        route!(self, account, service => service.draft_template(account, folder, id))
    }

    fn create_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        route!(self, account, service => service.create_folder(account, name))
    }

    fn rename_folder(&self, account: Option<&str>, from: &str, to: &str) -> MailResult<()> {
        route!(self, account, service => service.rename_folder(account, from, to))
    }

    fn delete_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        route!(self, account, service => service.delete_folder(account, name))
    }

    fn list_folders_detailed(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        route!(self, account, service => service.list_folders_detailed(account))
    }

    fn subscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        route!(self, account, service => service.subscribe_folder(account, name))
    }

    fn unsubscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        route!(self, account, service => service.unsubscribe_folder(account, name))
    }

    fn sieve_scripts(
        &self,
        account: Option<&str>,
    ) -> MailResult<Vec<crate::mail::sieve::SieveScript>> {
        super::router_sieve::scripts(self, account)
    }

    fn sieve_script(&self, account: Option<&str>, name: &str) -> MailResult<String> {
        super::router_sieve::script(self, account, name)
    }

    fn sieve_save_script(&self, account: Option<&str>, name: &str, body: &str) -> MailResult<()> {
        super::router_sieve::save_script(self, account, name, body)
    }

    fn sieve_set_active(&self, account: Option<&str>, name: Option<&str>) -> MailResult<()> {
        super::router_sieve::set_active(self, account, name)
    }

    fn sieve_delete_script(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        super::router_sieve::delete_script(self, account, name)
    }

    fn empty_folder(&self, account: Option<&str>, folder: &str) -> MailResult<String> {
        route!(self, account, service => service.empty_folder(account, folder))
    }

    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        route!(self, account, service => service.folder_unread(account, folder))
    }

    fn mark_folder_seen(&self, account: Option<&str>, folder: &str) -> MailResult<()> {
        let record = self.choose_account(account)?;
        route!(self, Some(&record.name), service => service.mark_folder_seen(account, folder))?;
        let _ = self.with_db(|conn| {
            store::mark_folder_seen(conn, &record.name, folder)
                .map_err(|err| MailError::config_invalid(err.to_string()))
        });
        Ok(())
    }

    fn folder_sync_cursor(
        &self,
        account: Option<&str>,
        folder: &str,
    ) -> MailResult<(Option<u32>, Option<u32>)> {
        route!(self, account, service => service.folder_sync_cursor(account, folder))
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
