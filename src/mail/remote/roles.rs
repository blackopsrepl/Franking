/*! RFC 6154 mailbox role detection. */

use std::io::{Read, Write};

use crate::mail::errors::MailResult;
use crate::mail::session::map_imap_error;
use crate::mail::types::FolderRole;

/// Resolve a mailbox role from RFC 6154 attributes.
pub(super) fn role_from_attributes(attributes: &[imap::types::NameAttribute<'_>]) -> FolderRole {
    for attribute in attributes {
        let name = attribute_name(attribute);
        if name.eq_ignore_ascii_case("\\Sent") {
            return FolderRole::Sent;
        }
        if name.eq_ignore_ascii_case("\\Drafts") {
            return FolderRole::Drafts;
        }
        if name.eq_ignore_ascii_case("\\Trash") {
            return FolderRole::Trash;
        }
        if name.eq_ignore_ascii_case("\\Archive") {
            return FolderRole::Archive;
        }
        if name.eq_ignore_ascii_case("\\Junk") {
            return FolderRole::Junk;
        }
        if name.eq_ignore_ascii_case("\\Flagged") {
            return FolderRole::Flagged;
        }
        if name.eq_ignore_ascii_case("\\All") {
            return FolderRole::All;
        }
    }
    FolderRole::Other
}

pub(super) fn list_folder_attributes<S: Read + Write>(
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
pub(super) fn pick_sent_folder(folders: &[(String, Vec<String>)]) -> Option<String> {
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
pub(super) fn pick_trash_folder(folders: &[(String, Vec<String>)]) -> Option<String> {
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

/// Choose the Drafts mailbox from LIST results, preferring the RFC 6154
/// `\Drafts` attribute over localized or historical names.
pub(super) fn pick_drafts_folder(folders: &[(String, Vec<String>)]) -> Option<String> {
    folders
        .iter()
        .find(|(_, attributes)| {
            attributes
                .iter()
                .any(|attribute| attribute.eq_ignore_ascii_case("\\Drafts"))
        })
        .or_else(|| {
            folders.iter().find(|(name, _)| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "drafts" | "inbox.drafts"
                )
            })
        })
        .map(|(name, _)| name.clone())
}
