/*! The app-facing mail service boundary. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::model::MessageDocument;
use crate::mail::session::IdleOutcome;
use crate::mail::types::{Account, Envelope, Folder};

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
    fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String>;

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

    /// Number of unseen messages in a folder.
    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        let _ = (account, folder);
        Err(MailError::unsupported_feature(
            "unread counts are not supported by this backend",
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
