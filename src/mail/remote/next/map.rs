/*! Mapping codec responses into the app's domain types. */

use imap_types::core::NString;
use imap_types::extensions::thread::Thread;
use imap_types::fetch::MessageDataItem;
use imap_types::flag::FlagFetch;
use imap_types::flag::{Flag, FlagNameAttribute};
use imap_types::mailbox::Mailbox;
use imap_types::response::{Data, Response};

use crate::mail::session::CommandOutput;
use crate::mail::types::{Envelope, Folder, FolderRole, Sender};

/// Folders from a LIST/SELECT response, skipping unselectable names.
pub fn folders_from_list(output: &CommandOutput) -> Vec<Folder> {
    output
        .responses
        .iter()
        .filter_map(|response| match response {
            Response::Data(Data::List { items, mailbox, .. }) => {
                if items
                    .iter()
                    .any(|item| matches!(item, FlagNameAttribute::Noselect))
                {
                    return None;
                }
                let names: Vec<String> = items.iter().map(attribute_name).collect();
                let role = role_from_names(&names);
                Some(Folder {
                    name: mailbox_name(mailbox),
                    desc: role.description().map(str::to_string),
                    role,
                })
            }
            _ => None,
        })
        .collect()
}

/// Envelopes from a FETCH response.
pub fn envelopes_from_fetch(output: &CommandOutput) -> Vec<Envelope> {
    let mut envelopes: Vec<Envelope> = output
        .responses
        .iter()
        .filter_map(|response| match response {
            Response::Data(Data::Fetch { items, .. }) => Some(envelope_from_items(items.as_ref())),
            _ => None,
        })
        .collect();
    envelopes.sort_by(|left, right| {
        right
            .id
            .parse::<u32>()
            .unwrap_or_default()
            .cmp(&left.id.parse::<u32>().unwrap_or_default())
    });
    envelopes
}

/// UIDs from a SEARCH/SORT response.
pub fn uids_from_search(output: &CommandOutput) -> Vec<u32> {
    let mut uids = Vec::new();
    for response in &output.responses {
        match response {
            Response::Data(Data::Search(ids, _)) => {
                uids.extend(ids.iter().map(|id| id.get()));
            }
            Response::Data(Data::Sort(ids, _)) => {
                uids.extend(ids.iter().map(|id| id.get()));
            }
            _ => {}
        }
    }
    uids
}

/// Threads from a THREAD response, flattened to UID groups.
pub fn thread_groups(output: &CommandOutput) -> Vec<Vec<u32>> {
    let mut groups = Vec::new();
    for response in &output.responses {
        if let Response::Data(Data::Thread(threads)) = response {
            for thread in threads {
                let mut group = Vec::new();
                collect_thread(thread, &mut group);
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
    }
    groups
}

fn collect_thread(thread: &Thread, group: &mut Vec<u32>) {
    match thread {
        Thread::Members { prefix, answers } => {
            group.extend(prefix.clone().into_iter().map(|uid| uid.get()));
            if let Some(answers) = answers {
                for answer in answers.clone().into_iter() {
                    collect_thread(&answer, group);
                }
            }
        }
        Thread::Nested { answers } => {
            for answer in answers.clone().into_iter() {
                collect_thread(&answer, group);
            }
        }
    }
}

fn envelope_from_items(items: &[MessageDataItem<'_>]) -> Envelope {
    let mut id = String::new();
    let mut flags = Vec::new();
    let mut subject = String::new();
    let mut sender = Sender::Unknown;
    let mut date = String::new();
    let mut message_id = None;
    let mut in_reply_to = None;

    for item in items {
        match item {
            MessageDataItem::Uid(uid) => id = uid.get().to_string(),
            MessageDataItem::Flags(list) => {
                flags = list.iter().map(flag_name).collect();
            }
            MessageDataItem::InternalDate(value) => {
                date = value.as_ref().format("%Y-%m-%d %H:%M:%S%:z").to_string();
            }
            MessageDataItem::Envelope(envelope) => {
                subject = nstring_text(&envelope.subject);
                sender = sender_from(&envelope.from);
                if date.is_empty() {
                    date = nstring_text(&envelope.date);
                }
                message_id = optional_id(nstring_text(&envelope.message_id));
                in_reply_to = optional_id(nstring_text(&envelope.in_reply_to));
            }
            _ => {}
        }
    }

    Envelope {
        id,
        flags,
        subject,
        sender,
        date,
        message_id,
        in_reply_to,
        account: None,
        folder: None,
    }
}

fn sender_from(addresses: &[imap_types::envelope::Address<'_>]) -> Sender {
    let Some(address) = addresses.first() else {
        return Sender::Unknown;
    };
    let name = nstring_text(&address.name);
    let mailbox = nstring_text(&address.mailbox);
    let host = nstring_text(&address.host);
    let addr = if mailbox.is_empty() || host.is_empty() {
        None
    } else {
        Some(format!("{mailbox}@{host}"))
    };
    match (name.is_empty(), addr) {
        (true, Some(addr)) => Sender::Plain(addr),
        (false, Some(addr)) => Sender::Structured {
            name: Some(name),
            addr: Some(addr),
        },
        (false, None) => Sender::Structured {
            name: Some(name),
            addr: None,
        },
        (true, None) => Sender::Unknown,
    }
}

fn nstring_text(value: &NString<'_>) -> String {
    match value.0.as_ref() {
        None => String::new(),
        Some(value) => String::from_utf8_lossy(&value.clone().into_inner()).to_string(),
    }
}

fn optional_id(value: String) -> Option<String> {
    let trimmed = value.trim().trim_matches(['<', '>']).to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn mailbox_name(mailbox: &Mailbox<'_>) -> String {
    match mailbox {
        Mailbox::Inbox => "INBOX".to_string(),
        Mailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).to_string(),
    }
}

fn flag_name(flag: &FlagFetch<'_>) -> String {
    match flag {
        FlagFetch::Recent => "Recent".to_string(),
        FlagFetch::Flag(Flag::Seen) => "Seen".to_string(),
        FlagFetch::Flag(Flag::Answered) => "Answered".to_string(),
        FlagFetch::Flag(Flag::Flagged) => "Flagged".to_string(),
        FlagFetch::Flag(Flag::Deleted) => "Deleted".to_string(),
        FlagFetch::Flag(Flag::Draft) => "Draft".to_string(),
        FlagFetch::Flag(Flag::Keyword(atom)) => atom.as_ref().to_string(),
        FlagFetch::Flag(Flag::Extension(extension)) => {
            // The extension atom is private; its Debug form carries the text.
            let debug = format!("{extension:?}");
            debug.split('"').nth(1).unwrap_or_default().to_string()
        }
    }
}

fn attribute_name(attribute: &FlagNameAttribute<'_>) -> String {
    match attribute {
        FlagNameAttribute::Noinferiors => "\\Noinferiors".to_string(),
        FlagNameAttribute::Noselect => "\\Noselect".to_string(),
        FlagNameAttribute::Marked => "\\Marked".to_string(),
        FlagNameAttribute::Unmarked => "\\Unmarked".to_string(),
        FlagNameAttribute::Extension(extension) => {
            // The extension atom is private and the codec strips the leading
            // backslash (`\Sent` arrives as `Atom("Sent")`), so put it back to
            // match the RFC 6154 attribute names.
            let debug = format!("{extension:?}");
            let atom = debug.split('"').nth(1).unwrap_or_default();
            format!("\\{atom}")
        }
    }
}

/// RFC 6154 role from attribute names.
pub fn role_from_names(names: &[String]) -> FolderRole {
    for name in names {
        let name = name.trim();
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
