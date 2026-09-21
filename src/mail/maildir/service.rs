/*! `MailService` implementation for the maildir backend. */

use std::fs;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::mime;
use crate::mail::service::MailService;
use crate::mail::types::{Account, Envelope, Folder, FolderRole};

use super::flags::*;
use super::fs_ops::*;
use super::model::MaildirService;
use super::template::*;

impl MailService for MaildirService {
    fn list_accounts(&self) -> MailResult<Vec<Account>> {
        self.ensure_ready()?;
        Ok(vec![Account {
            name: self.account_name.clone(),
            backend: "maildir".to_string(),
            default: self.is_default,
        }])
    }

    fn probe_account(&self, account: &str) -> MailResult<()> {
        if account != self.account_name {
            return Err(MailError::account_not_found(account.to_string()));
        }
        self.ensure_ready()
    }

    fn list_folders(&self, _account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.ensure_ready()?;
        Ok(vec![
            Folder {
                name: "INBOX".to_string(),
                desc: Some("Incoming messages".to_string()),
                role: FolderRole::Inbox,
            },
            Folder {
                name: "Sent".to_string(),
                desc: Some("Sent messages".to_string()),
                role: FolderRole::Sent,
            },
            Folder {
                name: "Drafts".to_string(),
                desc: Some("Draft messages".to_string()),
                role: FolderRole::Drafts,
            },
            Folder {
                name: "Trash".to_string(),
                desc: Some("Deleted messages".to_string()),
                role: FolderRole::Trash,
            },
        ])
    }

    fn list_envelopes(
        &self,
        _account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let mut entries = list_message_entries(&dir)?;
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.sort_key));

        let filtered: Vec<Envelope> = entries
            .into_iter()
            .filter(|entry| matches_query(entry, query))
            .map(|entry| entry.envelope)
            .collect();

        let start = page.saturating_sub(1) * page_size;
        Ok(filtered
            .into_iter()
            .skip(start)
            .take(page_size)
            .collect::<Vec<_>>())
    }

    fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes(account, folder, 1, usize::MAX, query)
    }

    fn read_message_raw(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        let raw =
            fs::read(&path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        mark_seen(&path)?;
        Ok(raw)
    }

    fn delete_message(&self, _account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        self.ensure_ready()?;
        if folder.eq_ignore_ascii_case("Trash") {
            let dir = self.folder_path(folder)?;
            let path = find_message_path(&dir, id)?;
            fs::remove_file(path)
                .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            return Ok(());
        }

        self.move_message(None, folder, "Trash", id)
    }

    fn move_message(
        &self,
        _account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let source_dir = self.folder_path(folder)?;
        let source = find_message_path(&source_dir, id)?;
        let target_dir = self.folder_path(target)?;
        let flags = parse_flag_codes(&source);
        let destination = next_message_path(&target_dir, &flags);
        fs::rename(&source, &destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok(())
    }

    fn copy_message(
        &self,
        _account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let source_dir = self.folder_path(folder)?;
        let source = find_message_path(&source_dir, id)?;
        let target_dir = self.folder_path(target)?;
        let flags = parse_flag_codes(&source);
        let destination = next_message_path(&target_dir, &flags);
        fs::copy(&source, &destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok(())
    }

    fn flag_add(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        update_flag(&path, flag, true)
    }

    fn flag_remove(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        update_flag(&path, flag, false)
    }

    fn download_attachments(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_ready()?;
        let raw = self.read_message_raw(None, folder, id)?;
        let document = mime::parse_message(&raw)?;
        crate::mail::attachments::save_to_downloads(attachment_payloads(&document))
    }

    fn template_write(&self, _account: Option<&str>) -> MailResult<String> {
        self.ensure_ready()?;
        Ok("\n".to_string())
    }

    fn template_reply(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let to = original
            .header_value("Reply-To")
            .map(str::to_string)
            .or_else(|| original.header_value("From").map(str::to_string))
            .unwrap_or_default();
        let cc = if all {
            original.header_value("Cc").unwrap_or_default().to_string()
        } else {
            String::new()
        };
        let subject = reply_subject(original.header_value("Subject").map(str::to_string));
        let body = quoted_reply_body(&original);

        let mut headers: Vec<(&str, String)> = vec![("To", to), ("Cc", cc), ("Subject", subject)];
        headers.extend(original.thread.reply_headers());
        Ok(render_template(&headers, &body))
    }

    fn template_forward(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let subject = forward_subject(original.header_value("Subject").map(str::to_string));
        let body = forwarded_body(&original);

        Ok(render_template(&[("Subject", subject)], &body))
    }

    fn template_send(&self, _account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_ready()?;
        let raw = render_outgoing(&parse_template_message(template))?;
        let sent_dir = self.folder_path("Sent")?;
        let destination = next_message_path(&sent_dir, &['S']);
        fs::write(&destination, raw)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok("Message sent.".to_string())
    }

    fn save_draft(&self, _account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_ready()?;
        let raw = render_outgoing(&parse_template_message(template))?;
        let drafts_dir = self.folder_path("Drafts")?;
        let destination = next_message_path(&drafts_dir, &['D']);
        fs::write(&destination, raw)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok("Draft saved.".to_string())
    }

    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        self.list_envelopes(account, folder, 1, usize::MAX, Some("not flag seen"))
            .map(|envelopes| envelopes.len())
    }
}
