/*! Maildir filesystem layout, seeding, and listing. */

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::mime;
use crate::mail::types::{Envelope, Sender};
use crate::mail::types::{Folder, FolderRole};

use super::flags::{file_name, flags_to_names, next_message_path, parse_flag_codes};

const SAMPLE_MESSAGE: &str = include_str!("../../../tests/fixtures/message.txt");

pub fn default_test_maildir_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solverforge")
        .join("test-maildir")
}

pub(super) fn ensure_maildir_structure(root: &Path) -> MailResult<()> {
    ensure_maildir_dir(root)?;
    for folder in [".Sent", ".Drafts", ".Trash"] {
        ensure_maildir_dir(&root.join(folder))?;
    }
    Ok(())
}

pub(super) fn ensure_maildir_dir(path: &Path) -> MailResult<()> {
    for child in ["cur", "new", "tmp"] {
        fs::create_dir_all(path.join(child))
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    }
    Ok(())
}

pub(super) fn seed_demo_message(root: &Path) -> MailResult<()> {
    let inbox_new = root.join("new");
    let has_messages = fs::read_dir(&inbox_new)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .next()
        .transpose()
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .is_some();

    let inbox_cur = root.join("cur");
    let has_cur_messages = fs::read_dir(&inbox_cur)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .next()
        .transpose()
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .is_some();

    if has_messages || has_cur_messages {
        return Ok(());
    }

    let destination = next_message_path(root, &[]);
    fs::write(destination, SAMPLE_MESSAGE)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    Ok(())
}

pub(super) fn folder_path(root: &Path, folder: &str) -> MailResult<PathBuf> {
    match folder {
        "INBOX" => Ok(root.to_path_buf()),
        "Sent" => Ok(root.join(".Sent")),
        "Drafts" => Ok(root.join(".Drafts")),
        "Trash" => Ok(root.join(".Trash")),
        other => Err(MailError::account_not_found(format!(
            "unknown folder {other}"
        ))),
    }
}

pub(super) fn list_message_entries(dir: &Path) -> MailResult<Vec<MessageEntry>> {
    let mut entries = Vec::new();
    for child in ["cur", "new"] {
        let bucket = dir.join(child);
        let read_dir = fs::read_dir(&bucket)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        for item in read_dir {
            let item = item.map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            if !item
                .file_type()
                .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
                .is_file()
            {
                continue;
            }
            let path = item.path();
            let raw =
                fs::read(&path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            let parsed = mime::parse_message(&raw)?;
            let sort_key = item
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);

            entries.push(MessageEntry {
                sort_key,
                searchable: format!(
                    "{}\n{}\n{}\n{}",
                    parsed.header_value("From").unwrap_or_default(),
                    parsed.header_value("To").unwrap_or_default(),
                    parsed.subject(),
                    parsed.render(78)
                )
                .to_ascii_lowercase(),
                envelope: Envelope {
                    id: file_name(&path)?,
                    flags: flags_to_names(&parse_flag_codes(&path)),
                    subject: parsed.subject().to_string(),
                    sender: Sender::Plain(
                        parsed.header_value("From").unwrap_or_default().to_string(),
                    ),
                    date: parsed.header_value("Date").unwrap_or_default().to_string(),
                    message_id: parsed.headers.message_id.clone(),
                    in_reply_to: parsed.headers.in_reply_to.first().cloned(),
                    account: None,
                    folder: None,
                },
            });
        }
    }
    Ok(entries)
}

pub(super) fn matches_query(entry: &MessageEntry, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    if query == "not flag seen" {
        return !entry.envelope.is_seen();
    }
    if query == "flag seen" {
        return entry.envelope.is_seen();
    }

    for part in query.split(" and ") {
        let part = part.trim();
        if let Some(term) = part.strip_prefix("subject ") {
            if !entry
                .envelope
                .subject
                .to_ascii_lowercase()
                .contains(term.trim())
            {
                return false;
            }
        } else if let Some(term) = part.strip_prefix("from ") {
            if !entry
                .envelope
                .sender_display()
                .to_ascii_lowercase()
                .contains(term.trim())
            {
                return false;
            }
        } else if !entry.searchable.contains(part) {
            return false;
        }
    }

    true
}

pub(super) struct MessageEntry {
    pub(super) sort_key: u64,
    pub(super) searchable: String,
    pub(super) envelope: Envelope,
}

/// Remove every message file from a folder, returning how many were removed.
pub(super) fn empty_folder(root: &Path, folder: &str) -> MailResult<usize> {
    let dir = folder_path(root, folder)?;
    let mut removed = 0;
    for bucket in ["cur", "new"] {
        let Ok(entries) = fs::read_dir(dir.join(bucket)) else {
            continue;
        };
        for entry in entries.flatten() {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// The fixed folder set of a local maildir account.
pub(super) fn local_folders() -> Vec<Folder> {
    [
        ("INBOX", "Incoming messages", FolderRole::Inbox),
        ("Sent", "Sent messages", FolderRole::Sent),
        ("Drafts", "Draft messages", FolderRole::Drafts),
        ("Trash", "Deleted messages", FolderRole::Trash),
    ]
    .into_iter()
    .map(|(name, desc, role)| Folder {
        name: name.to_string(),
        desc: Some(desc.to_string()),
        role,
        // A maildir has no subscriptions; the filesystem is the source of truth.
        subscribed: None,
    })
    .collect()
}
