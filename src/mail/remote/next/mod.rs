/*! Commands implemented on the app-owned IMAP client.
Replaces the string-command bodies in the neighbouring modules; the legacy
implementations stay until the dual-run equivalence test passes. */

use imap_types::command::CommandBody;
use imap_types::core::Charset;
use imap_types::core::Vec1;
use imap_types::extensions::sort::SortCriterion;
use imap_types::extensions::thread::ThreadingAlgorithm;
use imap_types::fetch::{MacroOrMessageDataItemNames, MessageDataItemName};
use imap_types::mailbox::Mailbox;
use imap_types::search::SearchKey;
use imap_types::sequence::SequenceSet;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::{CommandOutput, ImapClient, ReadWrite};
use crate::mail::types::{Envelope, Folder};

mod map;
pub mod search;

pub use map::{envelopes_from_fetch, folders_from_list, thread_groups, uids_from_search};

type Connection = ImapClient<Box<dyn ReadWrite>>;

fn imap_error(error: anyhow::Error) -> MailError {
    MailError::other(error.to_string())
}

/// Mailbox value for a folder name.
fn mailbox_of(folder: &str) -> MailResult<Mailbox<'static>> {
    Mailbox::try_from(folder.to_string())
        .map_err(|error| MailError::invalid_input(format!("invalid mailbox {folder}: {error:?}")))
}

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
    Ok(envelopes_from_fetch(&output))
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

#[cfg(test)]
mod tests;
