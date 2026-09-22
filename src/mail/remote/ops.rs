/*! Probe, flag, and mutation operations. */

use crate::mail::errors::{MailError, MailResult};

use super::errors::map_smtp_error;
use super::model::ImapSmtpService;
use super::next::{self, FlagChange};
use super::template::extract_attachments;

impl ImapSmtpService {
    /// Mark every message in a folder as seen.
    pub fn mark_folder_seen(&self, account: Option<&str>, folder: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        let seen = next::flag_of("seen")?;
        self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::store_flags(client, "1:*", FlagChange::Add, vec![seen.clone()], true)
        })
    }

    pub fn probe_account(&self, account: &str) -> MailResult<()> {
        self.ensure_account(account)?;
        self.probe_imap()?;
        self.probe_smtp()?;
        Ok(())
    }

    fn probe_imap(&self) -> MailResult<()> {
        self.pool.with_client(&self.account, |client| {
            next::list_folders(client).map(|_| ())
        })
    }

    fn probe_smtp(&self) -> MailResult<()> {
        let transport = self.smtp_transport()?;
        let ok = transport.test_connection().map_err(map_smtp_error)?;
        if ok {
            Ok(())
        } else {
            Err(MailError::connection_dropped(
                "SMTP server closed the connection during NOOP",
            ))
        }
    }

    fn set_flag(&self, folder: &str, id: &str, flag: &str, add: bool) -> MailResult<()> {
        let flag = next::flag_of(flag)?;
        let change = if add {
            FlagChange::Add
        } else {
            FlagChange::Remove
        };
        self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::store_flags(client, id, change, vec![flag.clone()], true)
        })
    }

    pub fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        let trash = self
            .trash_folder_name()?
            .unwrap_or_else(|| "Trash".to_string());

        if folder.eq_ignore_ascii_case(&trash) {
            let deleted = next::flag_of("deleted")?;
            return self.pool.with_client(&self.account, |client| {
                next::select(client, folder)?;
                next::store_flags(client, id, FlagChange::Add, vec![deleted.clone()], true)?;
                next::expunge_uids(client, id)
            });
        }

        self.move_message(account, folder, &trash, id)
    }

    pub fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        let deleted = next::flag_of("deleted")?;

        self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            // UID MOVE when the server has it, COPY + STORE + EXPUNGE otherwise.
            if next::move_messages(client, id, target).is_ok() {
                return Ok(());
            }
            next::copy(client, id, target)?;
            next::store_flags(client, id, FlagChange::Add, vec![deleted.clone()], true)?;
            next::expunge_uids(client, id)
        })
    }

    pub fn copy_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::copy(client, id, target).map(|_| ())
        })
    }

    pub fn flag_add(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.set_flag(folder, id, flag, true)
    }

    pub fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.set_flag(folder, id, flag, false)
    }

    /// Write every attachment into one archive, alongside the single-file save.
    pub fn download_attachments_zip(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let uid = id
            .parse::<u32>()
            .map_err(|_| MailError::invalid_input(format!("invalid message id {id}")))?;
        let raw = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::read_message_raw(client, uid)
        })?;
        crate::mail::service::attachment_archive::archive_from_raw(raw)
    }

    pub fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let uid = id
            .parse::<u32>()
            .map_err(|_| MailError::invalid_input(format!("invalid message id {id}")))?;

        let raw = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::read_message_raw(client, uid)
        })?;
        let attachments = extract_attachments(&raw)?;
        crate::mail::attachments::save_to_downloads(attachments)
    }
}
