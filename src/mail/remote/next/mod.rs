/*! Commands implemented on the app-owned IMAP client.
Replaces the string-command bodies used by the neighbouring modules; the legacy
implementations stay until the dual-run equivalence test passes for each. */

use imap_types::mailbox::Mailbox;
use imap_types::sequence::SequenceSet;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::{map_codec_error, ImapClient, ReadWrite};

mod condstore;
mod map;
mod read;
pub mod search;
mod write;

pub use condstore::{
    enable_qresync, fetch_changed_flags, select_condstore, SelectState, SyncAnchor,
};
pub use map::{envelopes_from_fetch, folders_from_list, thread_groups, uids_from_search};
pub use read::{
    examine, fetch_envelopes, list_folders, read_message_raw, search_uids, select, sort_uids,
    status, thread_uids, MailboxStatus,
};
pub use write::{
    append, copy, create_folder, delete_folder, expunge, expunge_uids, flag_of, list_subscribed,
    move_messages, rename_folder, store_flags, subscribe, unsubscribe, AppendResult, CopyResult,
    FlagChange,
};

/// An app-owned IMAP client over any transport.
pub type Connection = ImapClient<Box<dyn ReadWrite>>;

pub(crate) fn imap_error(error: anyhow::Error) -> MailError {
    map_codec_error(error)
}

/// Mailbox value for a folder name.
pub(crate) fn mailbox_of(folder: &str) -> MailResult<Mailbox<'static>> {
    Mailbox::try_from(folder.to_string())
        .map_err(|error| MailError::invalid_input(format!("invalid mailbox {folder}: {error:?}")))
}

/// Sequence or UID set value for a message set such as `1:*` or `4,7,9`.
pub(crate) fn sequence_of(value: &str) -> MailResult<SequenceSet> {
    SequenceSet::try_from(value).map_err(|error| {
        MailError::invalid_input(format!("invalid message set {value}: {error:?}"))
    })
}

#[cfg(test)]
mod tests;
