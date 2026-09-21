/*! The app-facing mail service boundary. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::model::MessageDocument;
use crate::mail::session::IdleOutcome;
use crate::mail::sieve::SieveScript;
use crate::mail::types::{Account, Envelope, Folder};

use super::service_options::SendOptions;

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
        crate::mail::mime::parse_message(self.read_message_raw(account, folder, id)?)
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
    fn template_send(
        &self,
        account: Option<&str>,
        template: &str,
        options: &SendOptions,
    ) -> MailResult<String>;

    /// Fetch every envelope in a folder so it can be cached for offline use.
    fn sync_folder(&self, account: Option<&str>, folder: &str) -> MailResult<Vec<Envelope>> {
        let _ = (account, folder);
        Ok(Vec::new())
    }

    /// List a folder page in a requested order.
    ///
    /// Backends with server-side SORT order the whole folder before paging, so
    /// the ordering is meaningful beyond the page. The default orders the
    /// fetched page locally, which is all a backend or server without SORT can
    /// offer.
    fn list_envelopes_sorted(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
        order: crate::mail::sort::SortOrder,
    ) -> MailResult<Vec<Envelope>> {
        let mut envelopes = self.list_envelopes(account, folder, page, page_size, query)?;
        order.apply(&mut envelopes);
        Ok(envelopes)
    }

    /// Report folder changes since an anchor, when the backend can.
    ///
    /// Backends that cannot report deltas return `full_resync`.
    fn sync_folder_delta(
        &self,
        account: Option<&str>,
        folder: &str,
        anchor: Option<crate::mail::remote::next::SyncAnchor>,
    ) -> MailResult<crate::mail::types::FolderDelta> {
        let _ = (account, folder, anchor);
        Ok(crate::mail::types::FolderDelta {
            full_resync: true,
            ..crate::mail::types::FolderDelta::default()
        })
    }

    /// Build a resume template from a stored draft message.
    fn draft_template(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<String> {
        let _ = (account, folder, id);
        Err(MailError::unsupported_feature(
            "resuming drafts is not supported by this backend",
        ))
    }

    /// Persist a compose template as a draft in the account's Drafts mailbox.
    fn save_draft(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        let _ = (account, template);
        Err(MailError::unsupported_feature(
            "saving drafts is not supported by this backend",
        ))
    }

    /// Create a new mailbox.
    fn create_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "creating folders is not supported by this backend",
        ))
    }

    /// Rename a mailbox.
    fn rename_folder(&self, account: Option<&str>, from: &str, to: &str) -> MailResult<()> {
        let _ = (account, from, to);
        Err(MailError::unsupported_feature(
            "renaming folders is not supported by this backend",
        ))
    }

    /// Delete a mailbox.
    fn delete_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "deleting folders is not supported by this backend",
        ))
    }

    /// Folder listing that also reports subscription state when the backend
    /// knows it. The default is the plain listing, so no backend is required to
    /// track subscriptions.
    fn list_folders_detailed(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.list_folders(account)
    }

    /// Subscribe to a mailbox.
    fn subscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "subscribing to folders is not supported by this backend",
        ))
    }

    /// Unsubscribe from a mailbox.
    fn unsubscribe_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "subscriptions are not supported by this backend",
        ))
    }

    /// List server-side Sieve filter scripts.
    fn sieve_scripts(&self, account: Option<&str>) -> MailResult<Vec<SieveScript>> {
        let _ = account;
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Fetch one Sieve script's source.
    fn sieve_script(&self, account: Option<&str>, name: &str) -> MailResult<String> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Create or replace a Sieve script.
    fn sieve_save_script(&self, account: Option<&str>, name: &str, body: &str) -> MailResult<()> {
        let _ = (account, name, body);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Activate a Sieve script, or deactivate all when `name` is `None`.
    fn sieve_set_active(&self, account: Option<&str>, name: Option<&str>) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Delete a Sieve script.
    fn sieve_delete_script(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Permanently remove every message in a folder.
    fn empty_folder(&self, account: Option<&str>, folder: &str) -> MailResult<String> {
        let _ = (account, folder);
        Err(MailError::unsupported_feature(
            "emptying folders is not supported by this backend",
        ))
    }

    /// Number of unseen messages in a folder.
    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        let _ = (account, folder);
        Err(MailError::unsupported_feature(
            "unread counts are not supported by this backend",
        ))
    }

    /// Mark every message in a folder as seen.
    fn mark_folder_seen(&self, account: Option<&str>, folder: &str) -> MailResult<()> {
        let _ = (account, folder);
        Err(MailError::unsupported_feature(
            "marking a folder read is not supported by this backend",
        ))
    }

    /// UIDVALIDITY and UIDNEXT for a folder, when the backend can report them.
    fn folder_sync_cursor(
        &self,
        account: Option<&str>,
        folder: &str,
    ) -> MailResult<(Option<u32>, Option<u32>)> {
        let _ = (account, folder);
        Ok((None, None))
    }

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
