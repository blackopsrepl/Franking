/*! Stored message and sync-state types. */

use crate::mail::model::MessageDocument;
use crate::mail::types::{Envelope, Sender};

/// A message persisted in the local store.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredMessage {
    pub account: String,
    pub folder: String,
    pub uid: String,
    pub uid_validity: Option<u32>,
    pub message_id: Option<String>,
    pub thread_root: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub subject: String,
    pub from_display: String,
    pub from_email: Option<String>,
    pub to_display: String,
    pub date_epoch: Option<i64>,
    pub flags: Vec<String>,
    pub size: u64,
    pub has_attachments: bool,
    pub snippet: String,
    pub body_text: String,
    pub raw: Option<Vec<u8>>,
}

/// Per-folder synchronization cursor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncState {
    pub account: String,
    pub folder: String,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
    pub highest_modseq: Option<u64>,
    pub last_synced_at: Option<String>,
}

impl StoredMessage {
    /// Project a parsed document into a storable row.
    pub fn from_document(
        account: &str,
        folder: &str,
        uid: &str,
        uid_validity: Option<u32>,
        flags: &[String],
        document: &MessageDocument,
        raw: Option<Vec<u8>>,
    ) -> Self {
        let message_id = document.thread.message_id.clone();
        let thread_root = document
            .thread
            .root_id()
            .map(str::to_string)
            .or_else(|| message_id.clone());
        let from_display = document
            .headers
            .from
            .first()
            .map(|address| address.display())
            .unwrap_or_default();
        let from_email = document
            .headers
            .from
            .first()
            .and_then(|address| address.normalized_email());
        let to_display = document
            .headers
            .recipients()
            .map(|address| address.display())
            .collect::<Vec<_>>()
            .join(", ");
        let body_text = document.search_text();

        Self {
            account: account.to_string(),
            folder: folder.to_string(),
            uid: uid.to_string(),
            uid_validity,
            message_id,
            thread_root,
            in_reply_to: document.thread.parent_id().map(str::to_string),
            references: document.thread.references.clone(),
            subject: document.subject().to_string(),
            from_display,
            from_email,
            to_display,
            date_epoch: document.headers.date.as_ref().map(|date| date.timestamp()),
            flags: flags.to_vec(),
            size: raw
                .as_ref()
                .map(|bytes| bytes.len() as u64)
                .unwrap_or(body_text.len() as u64),
            has_attachments: !document.attachments.is_empty(),
            snippet: snippet(&body_text),
            body_text,
            raw,
        }
    }
}

impl StoredMessage {
    /// Build envelope-only metadata from a server envelope.
    pub fn from_envelope(account: &str, folder: &str, envelope: &Envelope) -> Self {
        let from_email = match &envelope.sender {
            Sender::Structured {
                addr: Some(addr), ..
            } => Some(addr.to_ascii_lowercase()),
            Sender::Plain(value) => plain_email(value),
            _ => None,
        };
        Self {
            account: account.to_string(),
            folder: folder.to_string(),
            uid: envelope.id.clone(),
            subject: envelope.subject.clone(),
            message_id: envelope.message_id.clone(),
            in_reply_to: envelope.in_reply_to.clone(),
            from_display: envelope.sender.display(),
            from_email,
            date_epoch: parse_date_epoch(&envelope.date),
            flags: envelope.flags.clone(),
            ..Self::default()
        }
    }

    pub fn to_envelope(&self) -> Envelope {
        Envelope {
            id: self.uid.clone(),
            flags: self.flags.clone(),
            subject: self.subject.clone(),
            sender: Sender::Plain(self.from_display.clone()),
            date: self.date_epoch.and_then(format_date).unwrap_or_default(),
            message_id: self.message_id.clone(),
            in_reply_to: self.in_reply_to.clone(),
        }
    }
}

fn plain_email(value: &str) -> Option<String> {
    if let (Some(start), Some(end)) = (value.find('<'), value.find('>')) {
        if start < end {
            return Some(value[start + 1..end].trim().to_ascii_lowercase());
        }
    }
    let trimmed = value.trim();
    if trimmed.contains('@') && !trimmed.contains(char::is_whitespace) {
        Some(trimmed.to_ascii_lowercase())
    } else {
        None
    }
}

fn parse_date_epoch(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    for format in [
        "%Y-%m-%d %H:%M:%S%:z",
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
    ] {
        if let Ok(parsed) = chrono::DateTime::parse_from_str(value, format) {
            return Some(parsed.timestamp());
        }
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.timestamp())
}

fn format_date(epoch: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(epoch, 0)
        .map(|date| date.format("%Y-%m-%d %H:%M:%S%:z").to_string())
}

/// Insert or update envelope metadata without discarding cached body or raw
/// bytes. Listing a folder must never erase a previously cached full message.
fn snippet(body: &str) -> String {
    body.trim()
        .chars()
        .take(200)
        .collect::<String>()
        .replace('\n', " ")
}
