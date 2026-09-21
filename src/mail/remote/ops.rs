/*! Probe, flag, and mutation operations. */

use std::io::{Read, Write};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::{map_imap_error, ConnectedImapSession};

use super::errors::map_smtp_error;
use super::model::ImapSmtpService;
use super::search::imap_flag;
use super::template::extract_attachments;

impl ImapSmtpService {
    pub fn probe_account(&self, account: &str) -> MailResult<()> {
        self.ensure_account(account)?;
        self.probe_imap()?;
        self.probe_smtp()?;
        Ok(())
    }

    fn probe_imap(&self) -> MailResult<()> {
        fn exec<S: Read + Write>(session: &mut imap::Session<S>) -> MailResult<()> {
            session.list(None, Some("*")).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session),
                ConnectedImapSession::Tls(session) => exec(session),
            })
    }

    fn probe_smtp(&self) -> MailResult<()> {
        let transport = self.smtp_transport()?;
        let ok = transport.test_connection().map_err(map_smtp_error)?;
        if ok {
            Ok(())
        } else {
            Err(MailError::connection_dropped(
                "SMTP server closed the connection during NOOP",
            ))
        }
    }

    fn set_flag(&self, folder: &str, id: &str, flag: &str, add: bool) -> MailResult<()> {
        let op = if add {
            "+FLAGS.SILENT"
        } else {
            "-FLAGS.SILENT"
        };
        let mapped = imap_flag(flag);
        let command = format!("{op} ({mapped})");

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
            command: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            session.uid_store(id, command).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, id, &command),
                ConnectedImapSession::Tls(session) => exec(session, folder, id, &command),
            })
    }

    pub fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        let trash = self
            .trash_folder_name()?
            .unwrap_or_else(|| "Trash".to_string());

        if folder.eq_ignore_ascii_case(&trash) {
            fn exec<S: Read + Write>(
                session: &mut imap::Session<S>,
                folder: &str,
                id: &str,
            ) -> MailResult<()> {
                session.select(folder).map_err(map_imap_error)?;
                session
                    .uid_store(id, "+FLAGS.SILENT (\\Deleted)")
                    .map_err(map_imap_error)?;
                session.uid_expunge(id).map_err(map_imap_error)?;
                Ok(())
            }

            return self
                .pool
                .with_connection(&self.account, |connection| match connection {
                    ConnectedImapSession::Plain(session) => exec(session, folder, id),
                    ConnectedImapSession::Tls(session) => exec(session, folder, id),
                });
        }

        self.move_message(account, folder, &trash, id)
    }

    pub fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            target: &str,
            id: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            match session.mv(id, target) {
                Ok(()) => Ok(()),
                Err(_) => {
                    session.uid_copy(id, target).map_err(map_imap_error)?;
                    session
                        .uid_store(id, "+FLAGS.SILENT (\\Deleted)")
                        .map_err(map_imap_error)?;
                    session.uid_expunge(id).map_err(map_imap_error)?;
                    Ok(())
                }
            }
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, target, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, target, id),
            })
    }

    pub fn copy_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            target: &str,
            id: &str,
        ) -> MailResult<()> {
            session.select(folder).map_err(map_imap_error)?;
            session.uid_copy(id, target).map_err(map_imap_error)?;
            Ok(())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, target, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, target, id),
            })
    }

    pub fn flag_add(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.set_flag(folder, id, flag, true)
    }

    pub fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_requested_account(account)?;
        self.set_flag(folder, id, flag, false)
    }

    pub fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
        ) -> MailResult<Vec<(String, Vec<u8>)>> {
            session.select(folder).map_err(map_imap_error)?;
            let fetches = session
                .uid_fetch(id, "BODY.PEEK[]")
                .map_err(map_imap_error)?;
            let raw = fetches
                .iter()
                .find_map(|fetch| fetch.body())
                .ok_or_else(|| {
                    MailError::other("message body was not returned by the IMAP server")
                })?;
            extract_attachments(raw)
        }

        let attachments =
            self.pool
                .with_connection(&self.account, |connection| match connection {
                    ConnectedImapSession::Plain(session) => exec(session, folder, id),
                    ConnectedImapSession::Tls(session) => exec(session, folder, id),
                })?;

        crate::mail::attachments::save_to_downloads(attachments)
    }
}
