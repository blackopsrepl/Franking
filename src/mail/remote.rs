use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use imap::types::Flag;
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::authentication::Mechanism;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::Error as SmtpError;
use lettre::{Message, SmtpTransport, Transport};
use mail_parser::{MessageParser, MimeHeaders};

use super::account_store::AccountRecord;
use super::errors::{MailError, MailResult};
use super::mime;
use super::model::MessageDocument;
use super::oauth;
use super::session::{
    looks_like_auth_failure, map_imap_error, ConnectedImapSession, Security, SessionPool,
};
use super::types::{Envelope, Folder, Sender};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct ImapSmtpService {
    account: AccountRecord,
    pool: Arc<SessionPool>,
}

impl ImapSmtpService {
    pub fn new(account: AccountRecord, pool: Arc<SessionPool>) -> Self {
        Self { account, pool }
    }

    pub fn probe_account(&self, account: &str) -> MailResult<()> {
        self.ensure_account(account)?;
        self.probe_imap()?;
        self.probe_smtp()?;
        Ok(())
    }

    pub fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(session: &mut imap::Session<S>) -> MailResult<Vec<Folder>> {
            let names = session
                .list(None, Some("*"))
                .map_err(map_imap_error)?
                .into_iter()
                .filter(|name| {
                    !name
                        .attributes()
                        .iter()
                        .any(|attr| matches!(attr, imap::types::NameAttribute::NoSelect))
                })
                .map(|name| Folder {
                    name: name.name().to_string(),
                    desc: folder_description(name.name()),
                })
                .collect::<Vec<_>>();
            Ok(names)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session),
                ConnectedImapSession::Tls(session) => exec(session),
            })
    }

    pub fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            page: usize,
            page_size: usize,
            query: Option<&str>,
        ) -> MailResult<Vec<Envelope>> {
            session.select(folder).map_err(map_imap_error)?;

            let criteria = search_criteria(query);
            let mut uids = session
                .uid_search(&criteria)
                .map_err(map_imap_error)?
                .into_iter()
                .collect::<Vec<_>>();
            if uids.is_empty() {
                return Ok(Vec::new());
            }
            uids.sort_unstable_by(|left, right| right.cmp(left));

            let start = page.saturating_sub(1) * page_size;
            let page_uids = uids
                .into_iter()
                .skip(start)
                .take(page_size)
                .collect::<Vec<_>>();
            fetch_envelope_metadata(session, &page_uids)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => {
                    exec(session, folder, page, page_size, query)
                }
                ConnectedImapSession::Tls(session) => exec(session, folder, page, page_size, query),
            })
    }

    pub fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes(account, folder, 1, usize::MAX, query)
    }

    pub fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
        ) -> MailResult<Vec<u8>> {
            session.select(folder).map_err(map_imap_error)?;
            let fetches = session.uid_fetch(id, "RFC822").map_err(map_imap_error)?;
            fetches
                .iter()
                .find_map(|fetch| fetch.body())
                .map(<[u8]>::to_vec)
                .ok_or_else(|| MailError::other("message body was not returned by the IMAP server"))
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, id),
            })
    }

    pub fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        let trash = self
            .trash_folder_name()?
            .unwrap_or_else(|| "Trash".to_string());

        if folder.eq_ignore_ascii_case(&trash) {
            fn exec<S: Read + Write>(
                session: &mut imap::Session<S>,
                folder: &str,
                id: &str,
            ) -> MailResult<()> {
                session.select(folder).map_err(map_imap_error)?;
                session
                    .uid_store(id, "+FLAGS.SILENT (\\Deleted)")
                    .map_err(map_imap_error)?;
                session.uid_expunge(id).map_err(map_imap_error)?;
                Ok(())
            }

            return self
                .pool
                .with_connection(&self.account, |connection| match connection {
                    ConnectedImapSession::Plain(session) => exec(session, folder, id),
                    ConnectedImapSession::Tls(session) => exec(session, folder, id),
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

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            target: &str,
            id: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            match session.mv(id, target) {
                Ok(()) => Ok(()),
                Err(_) => {
                    session.uid_copy(id, target).map_err(map_imap_error)?;
                    session
                        .uid_store(id, "+FLAGS.SILENT (\\Deleted)")
                        .map_err(map_imap_error)?;
                    session.uid_expunge(id).map_err(map_imap_error)?;
                    Ok(())
                }
            }
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, target, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, target, id),
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

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            target: &str,
            id: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            session.uid_copy(id, target).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, target, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, target, id),
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

    pub fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
        ) -> MailResult<Vec<(String, Vec<u8>)>> {
            session.select(folder).map_err(map_imap_error)?;
            let fetches = session
                .uid_fetch(id, "BODY.PEEK[]")
                .map_err(map_imap_error)?;
            let raw = fetches
                .iter()
                .find_map(|fetch| fetch.body())
                .ok_or_else(|| {
                    MailError::other("message body was not returned by the IMAP server")
                })?;
            extract_attachments(raw)
        }

        let attachments =
            self.pool
                .with_connection(&self.account, |connection| match connection {
                    ConnectedImapSession::Plain(session) => exec(session, folder, id),
                    ConnectedImapSession::Tls(session) => exec(session, folder, id),
                })?;

        if attachments.is_empty() {
            return Err(MailError::unsupported_feature(
                "this message does not include any downloadable attachments",
            ));
        }

        let base = dirs::download_dir()
            .or_else(dirs::data_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("solverforge-mail");
        fs::create_dir_all(&base).map_err(|err| MailError::io(err.to_string()))?;

        let mut saved = Vec::new();
        for (index, (name, bytes)) in attachments.into_iter().enumerate() {
            let file_name = ensure_unique_attachment_name(&base, index, &name);
            let path = base.join(&file_name);
            fs::write(&path, bytes).map_err(|err| MailError::io(err.to_string()))?;
            saved.push(path.display().to_string());
        }

        Ok(saved.join(", "))
    }

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

        let message = builder
            .body(draft.body)
            .map_err(|err| MailError::invalid_input(err.to_string()))?;
        let transport = self.smtp_transport()?;
        transport.send(&message).map_err(map_smtp_error)?;

        let mut status = "Message sent.".to_string();
        if let Err(error) = self.save_message_to_sent(&message.formatted()) {
            status = format!("Message sent, but saving to Sent failed: {error}");
        }
        Ok(status)
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

    fn sent_folder_name(&self) -> MailResult<Option<String>> {
        let folders = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => list_folder_attributes(session),
                ConnectedImapSession::Tls(session) => list_folder_attributes(session),
            })?;
        Ok(pick_sent_folder(&folders))
    }

    fn trash_folder_name(&self) -> MailResult<Option<String>> {
        let folders = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => list_folder_attributes(session),
                ConnectedImapSession::Tls(session) => list_folder_attributes(session),
            })?;
        Ok(pick_trash_folder(&folders))
    }

    fn ensure_requested_account(&self, account: Option<&str>) -> MailResult<()> {
        if let Some(name) = account {
            self.ensure_account(name)?;
        }
        Ok(())
    }

    fn ensure_account(&self, account: &str) -> MailResult<()> {
        if self.account.name == account {
            Ok(())
        } else {
            Err(MailError::account_not_found(account.to_string()))
        }
    }

    fn probe_imap(&self) -> MailResult<()> {
        fn exec<S: Read + Write>(session: &mut imap::Session<S>) -> MailResult<()> {
            session.list(None, Some("*")).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session),
                ConnectedImapSession::Tls(session) => exec(session),
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
        let op = if add {
            "+FLAGS.SILENT"
        } else {
            "-FLAGS.SILENT"
        };
        let mapped = imap_flag(flag);
        let command = format!("{op} ({mapped})");

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
            command: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            session.uid_store(id, command).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, id, &command),
                ConnectedImapSession::Tls(session) => exec(session, folder, id, &command),
            })
    }

    fn smtp_transport(&self) -> MailResult<SmtpTransport> {
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

    fn username(&self) -> MailResult<&str> {
        self.account
            .username
            .as_deref()
            .ok_or_else(|| MailError::config_invalid("account username is missing"))
    }

    fn default_from_header(&self) -> Option<String> {
        self.account.username.as_ref().cloned()
    }
}

#[derive(Debug, Clone)]
struct TemplateMessage {
    headers: Vec<(String, String)>,
    body: String,
}

impl TemplateMessage {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.trim().is_empty())
    }
}

fn list_folder_attributes<S: Read + Write>(
    session: &mut imap::Session<S>,
) -> MailResult<Vec<(String, Vec<String>)>> {
    let names = session.list(None, Some("*")).map_err(map_imap_error)?;
    Ok(names
        .iter()
        .map(|name| {
            (
                name.name().to_string(),
                name.attributes()
                    .iter()
                    .map(attribute_name)
                    .collect::<Vec<_>>(),
            )
        })
        .collect())
}

fn attribute_name(attribute: &imap::types::NameAttribute<'_>) -> String {
    match attribute {
        imap::types::NameAttribute::NoInferiors => "\\NoInferiors".to_string(),
        imap::types::NameAttribute::NoSelect => "\\Noselect".to_string(),
        imap::types::NameAttribute::Marked => "\\Marked".to_string(),
        imap::types::NameAttribute::Unmarked => "\\Unmarked".to_string(),
        imap::types::NameAttribute::Custom(value) => value.to_string(),
    }
}

/// Choose the Sent mailbox from LIST results, preferring the RFC 6154
/// `\Sent` attribute over localized or historical names.
fn pick_sent_folder(folders: &[(String, Vec<String>)]) -> Option<String> {
    folders
        .iter()
        .find(|(_, attributes)| {
            attributes
                .iter()
                .any(|attribute| attribute.eq_ignore_ascii_case("\\Sent"))
        })
        .or_else(|| {
            folders.iter().find(|(name, _)| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "sent" | "sent items" | "sent messages" | "inbox.sent"
                )
            })
        })
        .map(|(name, _)| name.clone())
}

/// Choose the Trash mailbox from LIST results, preferring the RFC 6154
/// `\Trash` attribute over localized or historical names.
fn pick_trash_folder(folders: &[(String, Vec<String>)]) -> Option<String> {
    folders
        .iter()
        .find(|(_, attributes)| {
            attributes
                .iter()
                .any(|attribute| attribute.eq_ignore_ascii_case("\\Trash"))
        })
        .or_else(|| {
            folders.iter().find(|(name, _)| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "trash" | "deleted" | "deleted items" | "bin" | "inbox.trash"
                )
            })
        })
        .map(|(name, _)| name.clone())
}

fn fetch_envelope_metadata<S: Read + Write>(
    session: &mut imap::Session<S>,
    uids: &[u32],
) -> MailResult<Vec<Envelope>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    let query = uid_set(uids);
    let mut envelopes = session
        .uid_fetch(query, "(UID FLAGS INTERNALDATE ENVELOPE)")
        .map_err(map_imap_error)?
        .iter()
        .map(fetch_to_envelope)
        .collect::<Vec<_>>();
    envelopes.sort_by(|left, right| {
        right
            .id
            .parse::<u32>()
            .unwrap_or_default()
            .cmp(&left.id.parse::<u32>().unwrap_or_default())
    });
    Ok(envelopes)
}

fn fetch_to_envelope(fetch: &imap::types::Fetch) -> Envelope {
    let subject = fetch
        .envelope()
        .and_then(|envelope| envelope.subject)
        .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
        .unwrap_or_default();
    let sender = fetch
        .envelope()
        .and_then(|envelope| envelope.from.as_ref())
        .map(|addresses| sender_from_addresses(addresses))
        .unwrap_or(Sender::Unknown);
    let date = fetch
        .internal_date()
        .map(|date| date.format("%Y-%m-%d %H:%M:%S%:z").to_string())
        .or_else(|| {
            fetch
                .envelope()
                .and_then(|envelope| envelope.date)
                .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
        })
        .unwrap_or_default();

    Envelope {
        id: fetch.uid.unwrap_or(fetch.message).to_string(),
        flags: fetch.flags().iter().map(imap_flag_name).collect(),
        subject,
        sender,
        date,
    }
}

fn sender_from_addresses(addresses: &[imap_proto::types::Address<'_>]) -> Sender {
    let Some(address) = addresses.first() else {
        return Sender::Unknown;
    };

    let name = address
        .name
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let mailbox = address
        .mailbox
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let host = address
        .host
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let addr = match (mailbox, host) {
        (Some(local), Some(domain)) => Some(format!("{local}@{domain}")),
        (Some(local), None) => Some(local),
        _ => None,
    };

    if name.is_some() || addr.is_some() {
        Sender::Structured { name, addr }
    } else {
        Sender::Unknown
    }
}

/// Translate the app's search grammar into an RFC 3501 SEARCH expression so
/// filtering happens on the server instead of over a fully fetched mailbox.
fn search_criteria(query: Option<&str>) -> String {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return "ALL".to_string();
    };

    let mut criteria = Vec::new();
    for term in query.split(" and ") {
        let term = term.trim();
        match term.to_ascii_lowercase().as_str() {
            "flag seen" => criteria.push("SEEN".to_string()),
            "not flag seen" => criteria.push("UNSEEN".to_string()),
            "flag flagged" => criteria.push("FLAGGED".to_string()),
            "not flag flagged" => criteria.push("UNFLAGGED".to_string()),
            other => {
                if let Some(value) = other.strip_prefix("subject ") {
                    criteria.push(format!("SUBJECT {}", imap_quote(value.trim())));
                } else if let Some(value) = other.strip_prefix("from ") {
                    criteria.push(format!("FROM {}", imap_quote(value.trim())));
                } else {
                    criteria.push(format!("TEXT {}", imap_quote(other)));
                }
            }
        }
    }

    if criteria.is_empty() {
        "ALL".to_string()
    } else {
        criteria.join(" ")
    }
}

fn imap_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn folder_description(name: &str) -> Option<String> {
    match name.to_ascii_lowercase().as_str() {
        "inbox" => Some("Incoming messages".to_string()),
        "sent" | "sent items" => Some("Sent messages".to_string()),
        "drafts" => Some("Draft messages".to_string()),
        "trash" | "deleted" | "bin" => Some("Deleted messages".to_string()),
        _ => None,
    }
}

fn uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn imap_flag(flag: &str) -> &str {
    if flag.eq_ignore_ascii_case("seen") {
        "\\Seen"
    } else if flag.eq_ignore_ascii_case("flagged") {
        "\\Flagged"
    } else if flag.eq_ignore_ascii_case("answered") {
        "\\Answered"
    } else if flag.eq_ignore_ascii_case("deleted") {
        "\\Deleted"
    } else {
        flag
    }
}

fn imap_flag_name(flag: &Flag<'_>) -> String {
    match flag {
        Flag::Seen => "Seen".to_string(),
        Flag::Flagged => "Flagged".to_string(),
        Flag::Answered => "Answered".to_string(),
        Flag::Deleted => "Deleted".to_string(),
        Flag::Draft => "Draft".to_string(),
        Flag::Recent => "Recent".to_string(),
        Flag::MayCreate => "MayCreate".to_string(),
        Flag::Custom(value) => value.to_string(),
    }
}

fn map_smtp_error(error: SmtpError) -> MailError {
    if error.is_tls() {
        MailError::tls_failure(error.to_string())
    } else if error.is_timeout() {
        MailError::transport_timeout(error.to_string())
    } else if looks_like_auth_failure(&error.to_string()) {
        MailError::smtp_auth_rejected(error.to_string())
    } else if error.is_transport_shutdown() {
        MailError::connection_dropped(error.to_string())
    } else {
        MailError::other(error.to_string())
    }
}

fn extract_attachments(raw: &[u8]) -> MailResult<Vec<(String, Vec<u8>)>> {
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    let message = parser
        .parse(raw)
        .ok_or_else(|| MailError::other("failed to parse message attachments"))?;

    Ok(message
        .attachments()
        .enumerate()
        .map(|(index, part)| {
            let name = part
                .attachment_name()
                .map(str::to_string)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("attachment-{}", index + 1));
            (name, part.contents().to_vec())
        })
        .collect())
}

fn ensure_unique_attachment_name(base: &std::path::Path, index: usize, requested: &str) -> String {
    let sanitized = sanitize_file_name(requested);
    let candidate = if sanitized.is_empty() {
        format!("attachment-{}", index + 1)
    } else {
        sanitized
    };

    if !base.join(&candidate).exists() {
        return candidate;
    }

    let (stem, ext) = candidate
        .rsplit_once('.')
        .map(|(stem, ext)| (stem.to_string(), Some(ext.to_string())))
        .unwrap_or_else(|| (candidate.clone(), None));

    for suffix in 2..1000 {
        let attempt = match ext.as_deref() {
            Some(ext) => format!("{stem}-{suffix}.{ext}"),
            None => format!("{stem}-{suffix}"),
        };
        if !base.join(&attempt).exists() {
            return attempt;
        }
    }

    candidate
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_template_message(raw: &str) -> TemplateMessage {
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in raw.lines() {
        if in_body {
            body_lines.push(line);
            continue;
        }

        if line.is_empty() {
            in_body = true;
            continue;
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(key) = current_key.as_ref() {
                if let Some((_, value)) = headers
                    .iter_mut()
                    .find(|(name, _)| name.eq_ignore_ascii_case(key))
                {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(line.trim());
                }
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            headers.push((key.clone(), value.trim().to_string()));
            current_key = Some(key);
        }
    }

    TemplateMessage {
        headers,
        body: body_lines.join("\n"),
    }
}

fn render_template(headers: &[(&str, String)], body: &str) -> String {
    let mut out = String::new();
    for (name, value) in headers {
        if !value.trim().is_empty() {
            out.push_str(&format!("{name}: {}\n", value.trim()));
        }
    }
    out.push('\n');
    out.push_str(body);
    out
}

fn reply_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject
    } else if subject.is_empty() {
        "Re:".to_string()
    } else {
        format!("Re: {subject}")
    }
}

fn forward_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("fwd:") {
        subject
    } else if subject.is_empty() {
        "Fwd:".to_string()
    } else {
        format!("Fwd: {subject}")
    }
}

fn quoted_reply_body(message: &MessageDocument) -> String {
    let from = message.header_value("From").unwrap_or_default();
    let date = message.header_value("Date").unwrap_or_default();
    let intro = match (!date.is_empty(), !from.is_empty()) {
        (true, true) => format!("On {date}, {from} wrote:\n"),
        (false, true) => format!("{from} wrote:\n"),
        _ => "Previous message:\n".to_string(),
    };
    let rendered = message.render(78);
    let quoted = rendered
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let quoted = if rendered.is_empty() {
        String::new()
    } else {
        quoted
    };
    format!("\n{intro}{quoted}")
}

fn forwarded_body(message: &MessageDocument) -> String {
    let mut lines = vec!["---------- Forwarded message ----------".to_string()];
    for header in ["From", "Date", "Subject", "To", "Cc"] {
        if let Some(value) = message.header_value(header) {
            lines.push(format!("{header}: {value}"));
        }
    }
    lines.push(String::new());
    lines.push(message.render(78));
    lines.join("\n")
}

fn parse_mailboxes(value: &str) -> MailResult<Vec<Mailbox>> {
    let raw = format!("To: {value}\r\n\r\n");
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    let message = parser
        .parse(raw.as_bytes())
        .ok_or_else(|| MailError::invalid_input("failed to parse email address list"))?;
    let addresses = message
        .to()
        .ok_or_else(|| MailError::invalid_input("failed to parse email address list"))?;

    let mut mailboxes = Vec::new();
    for addr in addresses.iter() {
        let address = addr
            .address
            .as_deref()
            .ok_or_else(|| MailError::invalid_input("recipient address is missing"))?;
        let mailbox = match addr.name.as_deref().filter(|value| !value.is_empty()) {
            Some(name) => format!("{name} <{address}>"),
            None => address.to_string(),
        };
        mailboxes.push(
            mailbox
                .parse::<Mailbox>()
                .map_err(|err| MailError::invalid_input(err.to_string()))?,
        );
    }

    if mailboxes.is_empty() {
        Err(MailError::invalid_input(
            "at least one recipient address is required",
        ))
    } else {
        Ok(mailboxes)
    }
}

fn parse_mailbox(value: &str) -> MailResult<Mailbox> {
    parse_mailboxes(value)?
        .into_iter()
        .next()
        .ok_or_else(|| MailError::invalid_input("address list was empty"))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_template_message, pick_sent_folder, pick_trash_folder, sanitize_file_name,
        search_criteria,
    };

    #[test]
    fn sanitize_file_name_replaces_path_separators() {
        assert_eq!(
            sanitize_file_name("report:Q2/2026?.pdf"),
            "report_Q2_2026_.pdf"
        );
    }

    #[test]
    fn template_parser_splits_headers_and_body() {
        let parsed = parse_template_message("To: a@example.com\nSubject: Hi\n\nHello");
        assert_eq!(parsed.header("to"), Some("a@example.com"));
        assert_eq!(parsed.header("subject"), Some("Hi"));
        assert_eq!(parsed.body, "Hello");
    }

    #[test]
    fn search_criteria_translate_the_app_query_grammar() {
        assert_eq!(search_criteria(None), "ALL");
        assert_eq!(search_criteria(Some("   ")), "ALL");
        assert_eq!(search_criteria(Some("flag seen")), "SEEN");
        assert_eq!(search_criteria(Some("not flag seen")), "UNSEEN");
        assert_eq!(search_criteria(Some("flag flagged")), "FLAGGED");
        assert_eq!(
            search_criteria(Some("subject quarterly")),
            "SUBJECT \"quarterly\""
        );
        assert_eq!(
            search_criteria(Some("from alice and not flag seen")),
            "FROM \"alice\" UNSEEN"
        );
        assert_eq!(search_criteria(Some("revenue")), "TEXT \"revenue\"");
        assert_eq!(
            search_criteria(Some("subject \"quoted\"")),
            "SUBJECT \"\\\"quoted\\\"\""
        );
    }

    #[test]
    fn sent_folder_prefers_special_use_over_names() {
        let folders = vec![
            ("INBOX".to_string(), Vec::new()),
            ("Gesendet".to_string(), vec!["\\Sent".to_string()]),
            ("Sent".to_string(), Vec::new()),
        ];
        assert_eq!(pick_sent_folder(&folders).as_deref(), Some("Gesendet"));

        let fallback = vec![
            ("INBOX".to_string(), Vec::new()),
            ("Sent Items".to_string(), Vec::new()),
        ];
        assert_eq!(pick_sent_folder(&fallback).as_deref(), Some("Sent Items"));
        assert!(pick_sent_folder(&[("INBOX".to_string(), Vec::new())]).is_none());
    }

    #[test]
    fn trash_folder_prefers_special_use_over_names() {
        let folders = vec![
            ("INBOX".to_string(), Vec::new()),
            ("Papierkorb".to_string(), vec!["\\Trash".to_string()]),
            ("Trash".to_string(), Vec::new()),
        ];
        assert_eq!(pick_trash_folder(&folders).as_deref(), Some("Papierkorb"));

        let fallback = vec![
            ("INBOX".to_string(), Vec::new()),
            ("Deleted Items".to_string(), Vec::new()),
        ];
        assert_eq!(
            pick_trash_folder(&fallback).as_deref(),
            Some("Deleted Items")
        );
        assert!(pick_trash_folder(&[("INBOX".to_string(), Vec::new())]).is_none());
    }
}
