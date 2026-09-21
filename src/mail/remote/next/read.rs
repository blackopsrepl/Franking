/*! Read commands on the app-owned IMAP client. */

use std::borrow::Cow;

use imap_types::command::CommandBody;
use imap_types::core::Charset;
use imap_types::core::Vec1;
use imap_types::extensions::sort::SortCriterion;
use imap_types::extensions::thread::ThreadingAlgorithm;
use imap_types::fetch::{MacroOrMessageDataItemNames, MessageDataItemName};
use imap_types::response::{Data, Response};
use imap_types::search::SearchKey;
use imap_types::sequence::SequenceSet;
use imap_types::status::{StatusDataItem, StatusDataItemName};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::CommandOutput;
use crate::mail::types::{Envelope, Folder};

use super::map::{envelopes_from_fetch, folders_from_list, thread_groups, uids_from_search};
use super::{imap_error, mailbox_of, Connection};

/// LIST every selectable mailbox.
pub fn list_folders(client: &mut Connection) -> MailResult<Vec<Folder>> {
    let body = CommandBody::list("", "*")
        .map_err(|error| MailError::other(format!("cannot build LIST command: {error:?}")))?;
    let output = client.run(body).map_err(imap_error)?.require_ok("LIST")?;
    Ok(folders_from_list(&output))
}

/// SELECT a mailbox read-write.
pub fn select(client: &mut Connection, folder: &str) -> MailResult<CommandOutput> {
    client
        .run(CommandBody::Select {
            mailbox: mailbox_of(folder)?,
            parameters: vec![],
        })
        .map_err(imap_error)?
        .require_ok("SELECT")
}

/// EXAMINE a mailbox read-only.
pub fn examine(client: &mut Connection, folder: &str) -> MailResult<CommandOutput> {
    client
        .run(CommandBody::Examine {
            mailbox: mailbox_of(folder)?,
            parameters: vec![],
        })
        .map_err(imap_error)?
        .require_ok("EXAMINE")
}

/// Search a mailbox, returning matching UIDs.
pub fn search_uids(
    client: &mut Connection,
    criteria: Vec1<SearchKey<'static>>,
) -> MailResult<Vec<u32>> {
    let output = client
        .run(CommandBody::Search {
            charset: None,
            criteria,
            uid: true,
        })
        .map_err(imap_error)?
        .require_ok("UID SEARCH")?;
    Ok(uids_from_search(&output))
}

/// Server-side SORT, returning UIDs in server order.
pub fn sort_uids(
    client: &mut Connection,
    sort_criteria: Vec1<SortCriterion>,
    search_criteria: Vec1<SearchKey<'static>>,
) -> MailResult<Vec<u32>> {
    let output = client
        .run(CommandBody::Sort {
            sort_criteria,
            charset: Charset::Atom(
                imap_types::core::Atom::try_from("UTF-8").expect("static charset"),
            ),
            search_criteria,
            uid: true,
        })
        .map_err(imap_error)?
        .require_ok("UID SORT")?;
    Ok(uids_from_search(&output))
}

/// Server-side THREAD, returning UID groups.
pub fn thread_uids(
    client: &mut Connection,
    algorithm: ThreadingAlgorithm<'static>,
    search_criteria: Vec1<SearchKey<'static>>,
) -> MailResult<Vec<Vec<u32>>> {
    let output = client
        .run(CommandBody::Thread {
            algorithm,
            charset: Charset::Atom(
                imap_types::core::Atom::try_from("UTF-8").expect("static charset"),
            ),
            search_criteria,
            uid: true,
        })
        .map_err(imap_error)?
        .require_ok("UID THREAD")?;
    Ok(thread_groups(&output))
}

/// Fetch envelope metadata for specific UIDs.
pub fn fetch_envelopes(client: &mut Connection, uids: &[u32]) -> MailResult<Vec<Envelope>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }
    let uid_set = uids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let sequence = SequenceSet::try_from(uid_set.as_str())
        .map_err(|error| MailError::other(format!("invalid uid set: {error:?}")))?;

    let items: Vec<MessageDataItemName<'static>> = vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::InternalDate,
        MessageDataItemName::Envelope,
    ];
    let output = client
        .run(CommandBody::Fetch {
            sequence_set: sequence,
            macro_or_item_names: MacroOrMessageDataItemNames::MessageDataItemNames(items),
            uid: true,
            modifiers: vec![],
        })
        .map_err(imap_error)?
        .require_ok("UID FETCH")?;
    Ok(in_requested_order(envelopes_from_fetch(&output), uids))
}

/// Servers answer FETCH in mailbox order, so restore the caller's order.
///
/// The order carries meaning: it is the server's SORT order, or newest-first
/// for a plain listing.
fn in_requested_order(envelopes: Vec<Envelope>, uids: &[u32]) -> Vec<Envelope> {
    let position: std::collections::HashMap<u32, usize> = uids
        .iter()
        .enumerate()
        .map(|(index, uid)| (*uid, index))
        .collect();
    let mut envelopes = envelopes;
    envelopes.sort_by_key(|envelope| {
        position
            .get(&envelope.id.parse::<u32>().unwrap_or_default())
            .copied()
            .unwrap_or(usize::MAX)
    });
    envelopes
}

/// Read a whole message, without setting the Seen flag.
pub fn read_message_raw(client: &mut Connection, uid: u32) -> MailResult<Vec<u8>> {
    let uid_set = uid.to_string();
    let sequence = SequenceSet::try_from(uid_set.as_str())
        .map_err(|error| MailError::other(format!("invalid uid: {error:?}")))?;
    let output = client
        .run(CommandBody::Fetch {
            sequence_set: sequence,
            macro_or_item_names: MacroOrMessageDataItemNames::MessageDataItemNames(vec![
                MessageDataItemName::Uid,
                MessageDataItemName::BodyExt {
                    section: None,
                    partial: None,
                    peek: true,
                },
            ]),
            uid: true,
            modifiers: vec![],
        })
        .map_err(imap_error)?
        .require_ok("UID FETCH BODY.PEEK[]")?;

    for response in &output.responses {
        if let imap_types::response::Response::Data(imap_types::response::Data::Fetch {
            items,
            ..
        }) = response
        {
            for item in items.as_ref() {
                if let imap_types::fetch::MessageDataItem::BodyExt { data, .. } = item {
                    // `data` is an NString; a nil value means no body came back.
                    if let Some(istring) = data.0.clone() {
                        return Ok(istring.into_inner().into_owned());
                    }
                }
            }
        }
    }
    Err(MailError::other(
        "message body was not returned by the server",
    ))
}

/// The status items needed to seed sync cursors and unread counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MailboxStatus {
    pub messages: Option<u32>,
    pub unseen: Option<u32>,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
}

/// STATUS for a mailbox, without selecting it.
pub fn status(client: &mut Connection, folder: &str) -> MailResult<MailboxStatus> {
    let item_names: Cow<'static, [StatusDataItemName]> = Cow::Owned(vec![
        StatusDataItemName::Messages,
        StatusDataItemName::Unseen,
        StatusDataItemName::UidValidity,
        StatusDataItemName::UidNext,
    ]);
    let output = client
        .run(CommandBody::Status {
            mailbox: mailbox_of(folder)?,
            item_names,
        })
        .map_err(imap_error)?
        .require_ok("STATUS")?;

    let mut status = MailboxStatus::default();
    for response in &output.responses {
        let Response::Data(Data::Status { items, .. }) = response else {
            continue;
        };
        for item in items.as_ref() {
            match item {
                StatusDataItem::Messages(value) => status.messages = Some(*value),
                StatusDataItem::Unseen(value) => status.unseen = Some(*value),
                StatusDataItem::UidValidity(value) => status.uid_validity = Some(value.get()),
                StatusDataItem::UidNext(value) => status.uid_next = Some(value.get()),
                _ => {}
            }
        }
    }
    Ok(status)
}
