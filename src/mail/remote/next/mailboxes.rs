/*! Mailbox lifecycle commands. */

use imap_types::command::CommandBody;

use crate::mail::errors::MailResult;

use super::{imap_error, mailbox_of, Connection};

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
