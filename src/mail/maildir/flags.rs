/*! Maildir message lookup and flag handling. */

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::mime;
use crate::mail::model::MessageDocument;

pub(super) fn find_message_path(dir: &Path, id: &str) -> MailResult<PathBuf> {
    let base_id = id.split_once(":2,").map(|(base, _)| base).unwrap_or(id);
    for child in ["cur", "new"] {
        let candidate = dir.join(child).join(id);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    for child in ["cur", "new"] {
        let bucket = dir.join(child);
        let read_dir = fs::read_dir(&bucket)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        for item in read_dir {
            let item = item.map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            let path = item.path();
            if !path.is_file() {
                continue;
            }
            if base_message_name(&path)? == base_id {
                return Ok(path);
            }
        }
    }
    Err(MailError::account_not_found(format!(
        "message {id} was not found"
    )))
}

pub(super) fn mark_seen(path: &Path) -> MailResult<()> {
    update_flag(path, "seen", true)
}

pub(super) fn update_flag(path: &Path, flag: &str, present: bool) -> MailResult<()> {
    let mut flags = parse_flag_codes(path).into_iter().collect::<BTreeSet<_>>();
    let code = flag_code(flag)?;
    if present {
        flags.insert(code);
    } else {
        flags.remove(&code);
    }

    let parent = path
        .parent()
        .ok_or_else(|| MailError::local_maildir_failure("maildir path is missing a parent"))?;
    let mailbox = parent
        .parent()
        .ok_or_else(|| MailError::local_maildir_failure("maildir path is missing a mailbox"))?;
    let destination = if flags.is_empty() {
        mailbox.join("new").join(base_message_name(path)?)
    } else {
        let rendered = flags.iter().collect::<String>();
        mailbox
            .join("cur")
            .join(format!("{}:2,{}", base_message_name(path)?, rendered))
    };

    if destination != path {
        fs::rename(path, destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    }
    Ok(())
}

pub(super) fn flag_code(flag: &str) -> MailResult<char> {
    match flag.to_ascii_lowercase().as_str() {
        "seen" => Ok('S'),
        "flagged" => Ok('F'),
        "answered" => Ok('R'),
        "deleted" => Ok('T'),
        other => Err(MailError::unsupported_feature(format!(
            "maildir flag {other} is not supported"
        ))),
    }
}

pub(super) fn parse_flag_codes(path: &Path) -> Vec<char> {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let Some((_, suffix)) = name.rsplit_once(":2,") else {
        return Vec::new();
    };
    suffix.chars().collect()
}

pub(super) fn flags_to_names(flags: &[char]) -> Vec<String> {
    flags
        .iter()
        .filter_map(|flag| match flag {
            'S' => Some("Seen"),
            'F' => Some("Flagged"),
            'R' => Some("Answered"),
            'T' => Some("Deleted"),
            _ => None,
        })
        .map(str::to_string)
        .collect()
}

pub(super) fn local_message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("<{nanos}.{}@franking.local>", std::process::id())
}

pub(super) fn next_message_path(mailbox_dir: &Path, flags: &[char]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let base = format!("{nanos}.{}.franking", std::process::id());
    if flags.is_empty() {
        mailbox_dir.join("new").join(base)
    } else {
        let rendered = flags.iter().collect::<String>();
        mailbox_dir.join("cur").join(format!("{base}:2,{rendered}"))
    }
}

pub(super) fn file_name(path: &Path) -> MailResult<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| MailError::local_maildir_failure("message path is missing a file name"))
}

pub(super) fn base_message_name(path: &Path) -> MailResult<String> {
    let name = file_name(path)?;
    Ok(name
        .split_once(":2,")
        .map(|(base, _)| base.to_string())
        .unwrap_or(name))
}

pub(super) fn read_parsed_message(path: &Path) -> MailResult<MessageDocument> {
    let raw = fs::read(path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    mime::parse_message(&raw)
}
