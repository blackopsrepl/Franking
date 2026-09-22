/*! Compose templates, sending, drafts, and SMTP transport. */

use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::{Message, Transport};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::mime;
use crate::mail::service::SendOptions;

use crate::mail::types::{Folder, FolderRole};

use super::errors::map_smtp_error;
use super::model::ImapSmtpService;
use super::next;
use super::template::{
    forward_subject, forwarded_body, parse_mailbox, parse_mailboxes, parse_template_message,
    quoted_reply_body, render_template, reply_subject,
};

impl ImapSmtpService {
    pub fn template_write(&self, account: Option<&str>) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        Ok("\n".to_string())
    }

    pub fn template_reply(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let original = mime::parse_message(self.read_message_raw(account, folder, id)?)?;
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

    pub fn template_forward(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let original = mime::parse_message(self.read_message_raw(account, folder, id)?)?;
        let subject = forward_subject(original.header_value("Subject").map(str::to_string));
        let body = forwarded_body(&original);
        Ok(render_template(&[("Subject", subject)], &body))
    }

    pub fn template_send(
        &self,
        account: Option<&str>,
        template: &str,
        options: &SendOptions,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let message = self.build_outgoing_message(template)?;
        let transport = self.smtp_transport()?;

        let raw = match (options.is_pgp(), options.is_smime()) {
            (true, false) => self.wrap_pgp(&message.formatted(), options)?,
            (false, true) => self.wrap_smime(&message.formatted(), options)?,
            (false, false) => message.formatted(),
            (true, true) => {
                return Err(MailError::invalid_input(
                    "choose either PGP/MIME or S/MIME for one message, not both",
                ))
            }
        };

        transport
            .send_raw(message.envelope(), &raw)
            .map_err(map_smtp_error)?;

        let mut status = "Message sent.".to_string();
        let sent_folder = match options.sent_folder.as_deref() {
            Some(folder) => Some(folder.to_string()),
            None => self.sent_folder_name()?,
        };
        if let Err(error) = self.save_message_to_folder(sent_folder.as_deref(), &raw) {
            status = format!("Message sent, but saving to Sent failed: {error}");
        }
        Ok(status)
    }

    /// Wrap an outgoing message as PGP/MIME per `options`.
    fn wrap_pgp(&self, raw: &[u8], options: &SendOptions) -> MailResult<Vec<u8>> {
        let keys_dir = options
            .keys_dir
            .clone()
            .unwrap_or_else(crate::mail::pgp::default_keys_dir);
        let keyring = crate::mail::pgp::Keyring::load(&keys_dir);
        crate::mail::pgp_mime::wrap(raw, options, &keyring)
            .map_err(|err| MailError::invalid_input(err.to_string()))
    }

    /// Wrap an outgoing message as S/MIME per `options`.
    fn wrap_smime(&self, raw: &[u8], options: &SendOptions) -> MailResult<Vec<u8>> {
        let keys_dir = options
            .keys_dir
            .clone()
            .unwrap_or_else(crate::mail::pgp::default_keys_dir);
        let keyring = crate::mail::smime::SmimeKeyring::load(&keys_dir);
        crate::mail::smime_mime::wrap(raw, options, &keyring)
            .map_err(|err| MailError::invalid_input(err.to_string()))
    }

    /// Encrypt a draft to its own sender, when that key is available.
    ///
    /// A draft is a private note until it is sent, so it is encrypted to the
    /// sender rather than to the recipients. Without a key for the sender the
    /// draft is stored as it is, and the status line says so.
    fn encrypt_draft(&self, raw: &[u8], template: &str) -> MailResult<Vec<u8>> {
        let sender = template
            .lines()
            .find_map(|line| line.strip_prefix("From:"))
            .map(str::trim)
            .unwrap_or_default();
        if sender.is_empty() {
            return Ok(raw.to_vec());
        }
        let keys_dir = crate::mail::pgp::default_keys_dir();
        let keyring = crate::mail::pgp::Keyring::load(&keys_dir);
        crate::mail::pgp_mime::encrypt_for_emails(raw, &[sender.to_string()], &keyring)
            .map_err(|error| MailError::invalid_input(error.to_string()))
    }

    /// Persist a compose template to the account's Drafts mailbox.
    pub fn save_draft(
        &self,
        account: Option<&str>,
        template: &str,
        options: &SendOptions,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let message = self.build_outgoing_message(template)?;
        let Some(folder) = self.drafts_folder_name()? else {
            return Ok("Draft saved locally only (no Drafts mailbox found).".to_string());
        };
        let draft = next::flag_of("draft")?;
        let formatted = message.formatted();
        let stored = if options.encrypt_draft {
            self.encrypt_draft(&formatted, template)?
        } else {
            formatted
        };
        self.pool.with_client(&self.account, |client| {
            next::append(client, &folder, vec![draft.clone()], &stored)?;
            Ok(())
        })?;
        Ok("Draft saved.".to_string())
    }

    /// Count unseen messages in a folder using server-side SEARCH.
    fn build_outgoing_message(&self, template: &str) -> MailResult<Message> {
        let draft = parse_template_message(template);
        for (name, value) in &draft.headers {
            if crate::mail::draft::has_header_injection(value) {
                return Err(MailError::invalid_input(format!(
                    "header {name} contains a line break"
                )));
            }
        }
        let from = draft
            .header("from")
            .map(str::to_string)
            .or_else(|| self.default_from_header())
            .ok_or_else(|| MailError::invalid_input("a From address is required to send mail"))?;
        let to = draft
            .header("to")
            .ok_or_else(|| MailError::invalid_input("a To address is required to send mail"))?;

        let mut builder = Message::builder()
            .from(parse_mailbox(&from)?)
            .subject(draft.header("subject").unwrap_or_default())
            .header(ContentType::TEXT_PLAIN);

        for mailbox in parse_mailboxes(to)? {
            builder = builder.to(mailbox);
        }
        if let Some(value) = draft.header("cc") {
            for mailbox in parse_mailboxes(value)? {
                builder = builder.cc(mailbox);
            }
        }
        if let Some(value) = draft.header("bcc") {
            for mailbox in parse_mailboxes(value)? {
                builder = builder.bcc(mailbox);
            }
        }
        if let Some(value) = draft.header("in-reply-to") {
            builder = builder.in_reply_to(value.to_string());
        }
        if let Some(value) = draft.header("references") {
            builder = builder.references(value.to_string());
        }

        let attachments = draft
            .headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("attachment"))
            .map(|(_, value)| value.clone())
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>();

        if attachments.is_empty() {
            return builder
                .body(draft.body)
                .map_err(|err| MailError::invalid_input(err.to_string()));
        }

        let text_part = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(draft.body);
        let mut multipart = MultiPart::mixed().singlepart(text_part);
        for path in attachments {
            let bytes = std::fs::read(&path).map_err(|err| {
                MailError::invalid_input(format!("cannot read attachment {path}: {err}"))
            })?;
            let file_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("attachment")
                .to_string();
            let content_type = ContentType::parse("application/octet-stream")
                .map_err(|err| MailError::invalid_input(err.to_string()))?;
            multipart = multipart.singlepart(Attachment::new(file_name).body(bytes, content_type));
        }

        builder
            .multipart(multipart)
            .map_err(|err| MailError::invalid_input(err.to_string()))
    }

    /// Append a sent message to the account's Sent mailbox, discovered via the
    /// RFC 6154 `\Sent` attribute with a name fallback.
    /// Append a sent message to the account's Sent mailbox.
    pub fn save_message_to_sent(&self, raw: &[u8]) -> MailResult<()> {
        let folder = self.sent_folder_name()?;
        self.save_message_to_folder(folder.as_deref(), raw)
    }

    /// Append a sent message to `folder`, appending nothing when it is `None`.
    fn save_message_to_folder(&self, folder: Option<&str>, raw: &[u8]) -> MailResult<()> {
        let Some(folder) = folder else {
            return Ok(());
        };
        self.pool.with_client(&self.account, |client| {
            next::append(client, folder, vec![], raw).map(|_| ())
        })
    }

    pub(super) fn drafts_folder_name(&self) -> MailResult<Option<String>> {
        self.role_folder_name(FolderRole::Drafts)
    }

    pub(super) fn sent_folder_name(&self) -> MailResult<Option<String>> {
        self.role_folder_name(FolderRole::Sent)
    }

    pub(super) fn trash_folder_name(&self) -> MailResult<Option<String>> {
        self.role_folder_name(FolderRole::Trash)
    }

    fn role_folder_name(&self, role: FolderRole) -> MailResult<Option<String>> {
        Ok(pick_role_folder(&self.list_folders(None)?, role))
    }
}

/// Choose a role's mailbox, preferring the RFC 6154 attribute over localized or
/// historical names, which is how servers without SPECIAL-USE are handled.
pub(super) fn pick_role_folder(folders: &[Folder], role: FolderRole) -> Option<String> {
    folders
        .iter()
        .find(|folder| folder.role == role)
        .or_else(|| {
            folders
                .iter()
                .find(|folder| FolderRole::from_name(&folder.name) == role)
        })
        .map(|folder| folder.name.clone())
}
