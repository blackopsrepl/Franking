/*! Compose templates, sending, drafts, and SMTP transport. */

use std::time::Duration;

use imap::types::Flag;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Message, SmtpTransport, Transport};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::mime;
use crate::mail::oauth;
use crate::mail::session::{map_imap_error, ConnectedImapSession, Security};

use super::errors::map_smtp_error;
use super::model::ImapSmtpService;
use super::roles::{
    list_folder_attributes, pick_drafts_folder, pick_sent_folder, pick_trash_folder,
};
use super::template::{
    forward_subject, forwarded_body, parse_mailbox, parse_mailboxes, parse_template_message,
    quoted_reply_body, render_template, reply_subject,
};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

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

    pub fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let message = self.build_outgoing_message(template)?;
        let transport = self.smtp_transport()?;
        transport.send(&message).map_err(map_smtp_error)?;

        let mut status = "Message sent.".to_string();
        if let Err(error) = self.save_message_to_sent(&message.formatted()) {
            status = format!("Message sent, but saving to Sent failed: {error}");
        }
        Ok(status)
    }

    /// Persist a compose template to the account's Drafts mailbox.
    pub fn save_draft(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_requested_account(account)?;
        let message = self.build_outgoing_message(template)?;
        let Some(folder) = self.drafts_folder_name()? else {
            return Ok("Draft saved locally only (no Drafts mailbox found).".to_string());
        };
        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => session
                    .append_with_flags(&folder, message.formatted(), &[Flag::Draft])
                    .map_err(map_imap_error),
                ConnectedImapSession::Tls(session) => session
                    .append_with_flags(&folder, message.formatted(), &[Flag::Draft])
                    .map_err(map_imap_error),
            })?;
        Ok("Draft saved.".to_string())
    }

    /// Count unseen messages in a folder using server-side SEARCH.
    fn build_outgoing_message(&self, template: &str) -> MailResult<Message> {
        let draft = parse_template_message(template);
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

        builder
            .body(draft.body)
            .map_err(|err| MailError::invalid_input(err.to_string()))
    }

    /// Append a sent message to the account's Sent mailbox, discovered via the
    /// RFC 6154 `\Sent` attribute with a name fallback.
    pub fn save_message_to_sent(&self, raw: &[u8]) -> MailResult<()> {
        let Some(folder) = self.sent_folder_name()? else {
            return Ok(());
        };
        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => {
                    session.append(&folder, raw).map_err(map_imap_error)
                }
                ConnectedImapSession::Tls(session) => {
                    session.append(&folder, raw).map_err(map_imap_error)
                }
            })
    }

    pub(super) fn drafts_folder_name(&self) -> MailResult<Option<String>> {
        let folders = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => list_folder_attributes(session),
                ConnectedImapSession::Tls(session) => list_folder_attributes(session),
            })?;
        Ok(pick_drafts_folder(&folders))
    }

    pub(super) fn sent_folder_name(&self) -> MailResult<Option<String>> {
        let folders = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => list_folder_attributes(session),
                ConnectedImapSession::Tls(session) => list_folder_attributes(session),
            })?;
        Ok(pick_sent_folder(&folders))
    }

    pub(super) fn trash_folder_name(&self) -> MailResult<Option<String>> {
        let folders = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => list_folder_attributes(session),
                ConnectedImapSession::Tls(session) => list_folder_attributes(session),
            })?;
        Ok(pick_trash_folder(&folders))
    }

    pub(super) fn smtp_transport(&self) -> MailResult<SmtpTransport> {
        let host = self
            .account
            .smtp_host
            .as_deref()
            .ok_or_else(|| MailError::config_invalid("SMTP host is missing"))?;
        let port = self
            .account
            .smtp_port
            .ok_or_else(|| MailError::config_invalid("SMTP port is missing"))?;
        let security = Security::normalize(self.account.smtp_security.as_deref(), "tls");
        let username = self.username()?.to_string();

        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port)
            .timeout(Some(NETWORK_TIMEOUT));

        let tls = match security {
            Security::Tls => Tls::Wrapper(
                TlsParameters::new(host.to_string())
                    .map_err(|err| MailError::tls_failure(err.to_string()))?,
            ),
            Security::StartTls => Tls::Required(
                TlsParameters::new(host.to_string())
                    .map_err(|err| MailError::tls_failure(err.to_string()))?,
            ),
            Security::Plain => Tls::None,
        };
        builder = builder.tls(tls);

        match self.account.auth_mode.as_deref().unwrap_or("password") {
            "password" | "app_password" => {
                let secret_id = self
                    .account
                    .keyring_smtp_secret_id
                    .as_deref()
                    .ok_or_else(|| MailError::config_invalid("SMTP secret reference is missing"))?;
                let secret = self.pool.credentials().lookup(secret_id, &username)?;
                builder = builder.credentials(Credentials::new(username, secret));
            }
            "oauth2" => {
                let access_token = oauth::ensure_access_token(&self.account.name, &username)?;
                builder = builder
                    .credentials(Credentials::new(username, access_token))
                    .authentication(vec![Mechanism::Xoauth2]);
            }
            other => {
                return Err(MailError::unsupported_feature(format!(
                    "unsupported auth mode: {other}"
                )));
            }
        }

        Ok(builder.build())
    }
}
