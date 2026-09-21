/*! Mailbox lifecycle operations (create, rename, delete). */

use std::io::{Read, Write};

use crate::mail::errors::MailResult;
use crate::mail::session::{map_imap_error, ConnectedImapSession};

use super::model::ImapSmtpService;

impl ImapSmtpService {
    /// Create a new mailbox.
    pub fn create_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(session: &mut imap::Session<S>, name: &str) -> MailResult<()> {
            session.create(name).map_err(map_imap_error)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, name),
                ConnectedImapSession::Tls(session) => exec(session, name),
            })
    }

    /// Rename a mailbox.
    pub fn rename_folder(&self, account: Option<&str>, from: &str, to: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            from: &str,
            to: &str,
        ) -> MailResult<()> {
            session.rename(from, to).map_err(map_imap_error)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, from, to),
                ConnectedImapSession::Tls(session) => exec(session, from, to),
            })
    }

    /// Permanently remove every message in a mailbox.
    pub fn empty_folder(&self, account: Option<&str>, name: &str) -> MailResult<String> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(session: &mut imap::Session<S>, name: &str) -> MailResult<String> {
            session.select(name).map_err(map_imap_error)?;
            session
                .uid_store("1:*", "+FLAGS.SILENT (\\Deleted)")
                .map_err(map_imap_error)?;
            session.expunge().map_err(map_imap_error)?;
            Ok(format!("Emptied {name}."))
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, name),
                ConnectedImapSession::Tls(session) => exec(session, name),
            })
    }

    /// Delete a mailbox.
    pub fn delete_folder(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(session: &mut imap::Session<S>, name: &str) -> MailResult<()> {
            session.delete(name).map_err(map_imap_error)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, name),
                ConnectedImapSession::Tls(session) => exec(session, name),
            })
    }
}
