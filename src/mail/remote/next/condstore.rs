/*! CONDSTORE and QRESYNC: mailbox state and flag deltas. */

use std::num::{NonZeroU32, NonZeroU64};

use imap_types::command::{CommandBody, FetchModifier, SelectParameter};
use imap_types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName};
use imap_types::response::{Code, Data, Response};
use imap_types::sequence::SequenceSet;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::CommandOutput;

use super::{imap_error, mailbox_of, Connection};

/// What SELECT reports about a mailbox, including QRESYNC deltas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectState {
    pub exists: Option<u32>,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
    pub highest_modseq: Option<u64>,
    /// UIDs removed since the modseq given to QRESYNC.
    pub vanished: Vec<u32>,
    /// UIDs whose flags changed since the modseq given to QRESYNC.
    pub changed: Vec<u32>,
}

/// The anchor a client must remember to re-synchronize a mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncAnchor {
    pub uid_validity: u32,
    pub highest_modseq: u64,
}

/// The RFC 7162 ENABLE argument. `CapabilityEnable` has no QRESYNC variant, so
/// the extension name is carried as an atom.
fn qresync_enable() -> MailResult<imap_types::extensions::enable::CapabilityEnable<'static>> {
    imap_types::extensions::enable::CapabilityEnable::try_from("QRESYNC")
        .map_err(|error| MailError::other(format!("cannot name QRESYNC: {error:?}")))
}

/// ENABLE QRESYNC, which the server requires before honouring QRESYNC
/// parameters and before it reports VANISHED.
pub fn enable_qresync(client: &mut Connection) -> MailResult<()> {
    client
        .run(CommandBody::Enable {
            capabilities: imap_types::core::Vec1::from(qresync_enable()?),
        })
        .map_err(imap_error)?
        .require_ok("ENABLE QRESYNC")?;
    Ok(())
}

/// SELECT a mailbox with CONDSTORE, and with QRESYNC when an anchor is given.
///
/// QRESYNC makes the server report the messages that vanished and the messages
/// whose flags changed since `anchor`, so a resync transfers a delta instead of
/// the whole mailbox.
pub fn select_condstore(
    client: &mut Connection,
    folder: &str,
    anchor: Option<SyncAnchor>,
) -> MailResult<SelectState> {
    let parameters = match anchor {
        Some(anchor) => vec![SelectParameter::QResync {
            uid_validity: NonZeroU32::new(anchor.uid_validity)
                .ok_or_else(|| MailError::invalid_input("a UIDVALIDITY of zero is not valid"))?,
            mod_sequence_value: NonZeroU64::new(anchor.highest_modseq).ok_or_else(|| {
                MailError::invalid_input("a change sequence of zero is not valid")
            })?,
            known_uids: None,
            seq_match_data: None,
        }],
        None => vec![SelectParameter::CondStore],
    };
    let output = client
        .run(CommandBody::Select {
            mailbox: mailbox_of(folder)?,
            parameters,
        })
        .map_err(imap_error)?
        .require_ok("SELECT")?;
    Ok(select_state(&output))
}

/// UIDs and flags that changed since `modseq`, via CHANGEDSINCE.
pub fn fetch_changed_flags(
    client: &mut Connection,
    modseq: u64,
) -> MailResult<Vec<(u32, Vec<String>)>> {
    let items: Vec<MessageDataItemName<'static>> = vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::ModSeq,
    ];
    let output = client
        .run(CommandBody::Fetch {
            sequence_set: SequenceSet::try_from("1:*")
                .map_err(|error| MailError::other(format!("invalid message set: {error:?}")))?,
            macro_or_item_names: MacroOrMessageDataItemNames::MessageDataItemNames(items),
            uid: true,
            modifiers: vec![FetchModifier::ChangedSince(
                NonZeroU64::new(modseq).ok_or_else(|| {
                    MailError::invalid_input("a change sequence of zero is not valid")
                })?,
            )],
        })
        .map_err(imap_error)?
        .require_ok("UID FETCH CHANGEDSINCE")?;
    Ok(changed_flags(&output))
}

fn select_state(output: &CommandOutput) -> SelectState {
    let mut state = SelectState::default();
    for response in &output.responses {
        match response {
            Response::Data(Data::Exists(count)) => state.exists = Some(*count),
            Response::Data(Data::Vanished { known_uids, .. }) => {
                state.vanished.extend(uid_values(known_uids));
            }
            Response::Data(Data::Fetch { items, .. }) => {
                for item in items.as_ref() {
                    if let MessageDataItem::Uid(uid) = item {
                        state.changed.push(uid.get());
                    }
                }
            }
            _ => {}
        }
    }
    for body in &output.untagged {
        match body.code.as_ref() {
            Some(Code::UidValidity(value)) => state.uid_validity = Some(value.get()),
            Some(Code::UidNext(value)) => state.uid_next = Some(value.get()),
            Some(Code::HighestModSeq(value)) => state.highest_modseq = Some(value.get()),
            _ => {}
        }
    }
    if let Some(completion) = &output.completion {
        match completion.code.as_ref() {
            Some(Code::UidValidity(value)) => state.uid_validity = Some(value.get()),
            Some(Code::UidNext(value)) => state.uid_next = Some(value.get()),
            Some(Code::HighestModSeq(value)) => state.highest_modseq = Some(value.get()),
            _ => {}
        }
    }
    state
}

fn changed_flags(output: &CommandOutput) -> Vec<(u32, Vec<String>)> {
    let mut changed = Vec::new();
    for response in &output.responses {
        let Response::Data(Data::Fetch { items, .. }) = response else {
            continue;
        };
        let mut uid = None;
        let mut flags = Vec::new();
        for item in items.as_ref() {
            match item {
                MessageDataItem::Uid(value) => uid = Some(value.get()),
                MessageDataItem::Flags(values) => {
                    flags = values.iter().map(super::map::flag_name).collect();
                }
                _ => {}
            }
        }
        if let Some(uid) = uid {
            changed.push((uid, flags));
        }
    }
    changed
}

fn uid_values(set: &SequenceSet) -> Vec<u32> {
    let mut uids = Vec::new();
    for range in set.0.clone() {
        match range {
            imap_types::sequence::Sequence::Single(value) => {
                if let Some(uid) = seq_value(&value) {
                    uids.push(uid);
                }
            }
            imap_types::sequence::Sequence::Range(from, to) => {
                let (Some(from), Some(to)) = (seq_value(&from), seq_value(&to)) else {
                    continue;
                };
                uids.extend(from..=to);
            }
        }
    }
    uids
}

/// A sequence value; `*` carries no number and is dropped.
fn seq_value(value: &imap_types::sequence::SeqOrUid) -> Option<u32> {
    match value {
        imap_types::sequence::SeqOrUid::Value(value) => Some(value.get()),
        imap_types::sequence::SeqOrUid::Asterisk => None,
    }
}
