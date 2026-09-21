/*! Mutating commands on the app-owned IMAP client. */

use imap_types::command::CommandBody;
use imap_types::extensions::binary::LiteralOrLiteral8;
use imap_types::extensions::uidplus::{UidElement, UidSet};
use imap_types::flag::{Flag, StoreResponse, StoreType};
use imap_types::response::{Code, Response};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::types::Folder;

use super::map::folders_from_list;
use super::{imap_error, mailbox_of, sequence_of, Connection};

/// How a flag change combines with the flags already on a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagChange {
    Replace,
    Add,
    Remove,
}

impl FlagChange {
    fn store_type(self) -> StoreType {
        match self {
            FlagChange::Replace => StoreType::Replace,
            FlagChange::Add => StoreType::Add,
            FlagChange::Remove => StoreType::Remove,
        }
    }
}

/// A COPYUID or APPENDUID code, naming the UIDs a server assigned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyResult {
    pub uid_validity: u32,
    pub source: Vec<u32>,
    pub destination: Vec<u32>,
}

/// The UID a server assigned to an appended message, when it reports one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendResult {
    pub uid_validity: u32,
    pub uid: u32,
}

/// Map an app flag name onto an IMAP flag.
///
/// App flag names arrive without a leading backslash. The five system flags
/// must become their dedicated variants; anything else is a keyword, and
/// `Flag::system` would wrongly render those as `\$label1`.
pub fn flag_of(name: &str) -> MailResult<Flag<'static>> {
    let trimmed = name.trim();
    let bare = trimmed.strip_prefix('\\').unwrap_or(trimmed);
    if bare.is_empty() {
        return Err(MailError::invalid_input("empty flag name"));
    }
    let atom = imap_types::core::Atom::try_from(bare.to_string())
        .map_err(|error| MailError::invalid_input(format!("invalid flag {name}: {error:?}")))?;
    Ok(match atom.as_ref().to_ascii_lowercase().as_str() {
        "answered" => Flag::Answered,
        "deleted" => Flag::Deleted,
        "draft" => Flag::Draft,
        "flagged" => Flag::Flagged,
        "seen" => Flag::Seen,
        _ => Flag::keyword(atom),
    })
}

/// STORE flags on a message set.
pub fn store_flags(
    client: &mut Connection,
    set: &str,
    change: FlagChange,
    flags: Vec<Flag<'static>>,
    silent: bool,
) -> MailResult<()> {
    let response = if silent {
        StoreResponse::Silent
    } else {
        StoreResponse::Answer
    };
    client
        .run(CommandBody::Store {
            sequence_set: sequence_of(set)?,
            kind: change.store_type(),
            response,
            flags,
            uid: true,
            modifiers: vec![],
        })
        .map_err(imap_error)?
        .require_ok("UID STORE")?;
    Ok(())
}

/// UID COPY a message set, returning the server's COPYUID mapping when given.
pub fn copy(client: &mut Connection, set: &str, target: &str) -> MailResult<Option<CopyResult>> {
    let output = client
        .run(CommandBody::Copy {
            sequence_set: sequence_of(set)?,
            mailbox: mailbox_of(target)?,
            uid: true,
        })
        .map_err(imap_error)?
        .require_ok("UID COPY")?;
    Ok(copy_result(&output))
}

/// UID MOVE a message set, returning the server's COPYUID mapping when given.
pub fn move_messages(
    client: &mut Connection,
    set: &str,
    target: &str,
) -> MailResult<Option<CopyResult>> {
    let output = client
        .run(CommandBody::Move {
            sequence_set: sequence_of(set)?,
            mailbox: mailbox_of(target)?,
            uid: true,
        })
        .map_err(imap_error)?
        .require_ok("UID MOVE")?;
    Ok(copy_result(&output))
}

/// EXPUNGE every message with the `\Deleted` flag in the selected mailbox.
pub fn expunge(client: &mut Connection) -> MailResult<()> {
    client
        .run(CommandBody::Expunge)
        .map_err(imap_error)?
        .require_ok("EXPUNGE")?;
    Ok(())
}

/// UID EXPUNGE a message set, leaving other `\Deleted` messages alone.
pub fn expunge_uids(client: &mut Connection, set: &str) -> MailResult<()> {
    client
        .run(CommandBody::ExpungeUid {
            sequence_set: sequence_of(set)?,
        })
        .map_err(imap_error)?
        .require_ok("UID EXPUNGE")?;
    Ok(())
}

/// APPEND a message, returning the UID the server assigned when it reports one.
pub fn append(
    client: &mut Connection,
    folder: &str,
    flags: Vec<Flag<'static>>,
    raw: &[u8],
) -> MailResult<Option<AppendResult>> {
    let message = LiteralOrLiteral8::Literal(
        imap_types::core::Literal::try_from(raw.to_vec())
            .map_err(|error| MailError::other(format!("message is not appendable: {error:?}")))?,
    );
    let output = client
        .run(CommandBody::Append {
            mailbox: mailbox_of(folder)?,
            flags,
            date: None,
            message,
        })
        .map_err(imap_error)?
        .require_ok("APPEND")?;

    // APPENDUID arrives as a response code on the tagged completion.
    Ok(output.any_code(|code| match code {
        Code::AppendUid { uid_validity, uid } => Some(AppendResult {
            uid_validity: uid_validity.get(),
            uid: uid.get(),
        }),
        _ => None,
    }))
}

/// CREATE a mailbox.
pub fn create_folder(client: &mut Connection, name: &str) -> MailResult<()> {
    client
        .run(CommandBody::Create {
            mailbox: mailbox_of(name)?,
        })
        .map_err(imap_error)?
        .require_ok("CREATE")?;
    Ok(())
}

/// DELETE a mailbox.
pub fn delete_folder(client: &mut Connection, name: &str) -> MailResult<()> {
    client
        .run(CommandBody::Delete {
            mailbox: mailbox_of(name)?,
        })
        .map_err(imap_error)?
        .require_ok("DELETE")?;
    Ok(())
}

/// RENAME a mailbox.
pub fn rename_folder(client: &mut Connection, from: &str, to: &str) -> MailResult<()> {
    client
        .run(CommandBody::Rename {
            from: mailbox_of(from)?,
            to: mailbox_of(to)?,
        })
        .map_err(imap_error)?
        .require_ok("RENAME")?;
    Ok(())
}

/// SUBSCRIBE to a mailbox.
pub fn subscribe(client: &mut Connection, name: &str) -> MailResult<()> {
    client
        .run(CommandBody::Subscribe {
            mailbox: mailbox_of(name)?,
        })
        .map_err(imap_error)?
        .require_ok("SUBSCRIBE")?;
    Ok(())
}

/// UNSUBSCRIBE from a mailbox.
pub fn unsubscribe(client: &mut Connection, name: &str) -> MailResult<()> {
    client
        .run(CommandBody::Unsubscribe {
            mailbox: mailbox_of(name)?,
        })
        .map_err(imap_error)?
        .require_ok("UNSUBSCRIBE")?;
    Ok(())
}

/// LSUB: the mailboxes the account is subscribed to.
pub fn list_subscribed(client: &mut Connection) -> MailResult<Vec<Folder>> {
    let body = CommandBody::lsub("", "*")
        .map_err(|error| MailError::other(format!("cannot build LSUB command: {error:?}")))?;
    let output = client.run(body).map_err(imap_error)?.require_ok("LSUB")?;
    Ok(folders_from_list(&output))
}

fn copy_result(output: &crate::mail::session::CommandOutput) -> Option<CopyResult> {
    for response in &output.responses {
        let Response::Status(status) = response else {
            continue;
        };
        let Some(Code::CopyUid {
            uid_validity,
            source,
            destination,
        }) = status.code()
        else {
            continue;
        };
        return Some(CopyResult {
            uid_validity: uid_validity.get(),
            source: uid_values(source),
            destination: uid_values(destination),
        });
    }
    None
}

fn uid_values(set: &UidSet) -> Vec<u32> {
    let mut uids = Vec::new();
    for element in set.0.clone() {
        match element {
            UidElement::Single(uid) => uids.push(uid.get()),
            UidElement::Range(from, to) => uids.extend(from.get()..=to.get()),
        }
    }
    uids
}
