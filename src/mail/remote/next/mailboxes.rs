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

/// RENAME a mailbox. IMAP renames its subfolders too, so the server's
/// hierarchy delimiter is returned for callers that track folder paths.
pub fn rename_folder(client: &mut Connection, from: &str, to: &str) -> MailResult<Option<char>> {
    let delimiter = hierarchy_delimiter(client, from)?;
    client
        .run(CommandBody::Rename {
            from: mailbox_of(from)?,
            to: mailbox_of(to)?,
        })
        .map_err(imap_error)?
        .require_ok("RENAME")?;
    Ok(delimiter)
}

/// The delimiter the server reports for `name`; `None` for a flat namespace.
fn hierarchy_delimiter(client: &mut Connection, name: &str) -> MailResult<Option<char>> {
    use imap_types::response::{Data, Response};
    let body = CommandBody::list("", name.to_owned()).map_err(|error| {
        crate::mail::errors::MailError::other(format!("cannot build LIST command: {error:?}"))
    })?;
    let output = client.run(body).map_err(imap_error)?.require_ok("LIST")?;
    Ok(output.responses.iter().find_map(|response| match response {
        Response::Data(Data::List { delimiter, .. }) => delimiter.as_ref().map(|d| d.inner()),
        _ => None,
    }))
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
