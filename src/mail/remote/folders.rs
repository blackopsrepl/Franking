/*! Folder and envelope listing plus raw message reads. */

use std::io::{Read, Write};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::session::{map_imap_error, ConnectedImapSession};
use crate::mail::types::{Envelope, Folder};

use super::envelope::fetch_envelope_metadata;
use super::model::ImapSmtpService;
use super::roles::role_from_attributes;
use super::search::search_criteria;

impl ImapSmtpService {
    pub fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(session: &mut imap::Session<S>) -> MailResult<Vec<Folder>> {
            let names = session
                .list(None, Some("*"))
                .map_err(map_imap_error)?
                .into_iter()
                .filter(|name| {
                    !name
                        .attributes()
                        .iter()
                        .any(|attr| matches!(attr, imap::types::NameAttribute::NoSelect))
                })
                .map(|name| {
                    let role = role_from_attributes(name.attributes());
                    Folder {
                        name: name.name().to_string(),
                        desc: role.description().map(str::to_string),
                        role,
                    }
                })
                .collect::<Vec<_>>();
            Ok(names)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session),
                ConnectedImapSession::Tls(session) => exec(session),
            })
    }

    pub fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            page: usize,
            page_size: usize,
            query: Option<&str>,
        ) -> MailResult<Vec<Envelope>> {
            session.select(folder).map_err(map_imap_error)?;

            let criteria = search_criteria(query);
            let mut uids = session
                .uid_search(&criteria)
                .map_err(map_imap_error)?
                .into_iter()
                .collect::<Vec<_>>();
            if uids.is_empty() {
                return Ok(Vec::new());
            }
            uids.sort_unstable_by(|left, right| right.cmp(left));

            let start = page.saturating_sub(1) * page_size;
            let page_uids = uids
                .into_iter()
                .skip(start)
                .take(page_size)
                .collect::<Vec<_>>();
            fetch_envelope_metadata(session, &page_uids)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => {
                    exec(session, folder, page, page_size, query)
                }
                ConnectedImapSession::Tls(session) => exec(session, folder, page, page_size, query),
            })
    }

    pub fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes(account, folder, 1, usize::MAX, query)
    }

    pub fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
            id: &str,
        ) -> MailResult<Vec<u8>> {
            session.select(folder).map_err(map_imap_error)?;
            let fetches = session.uid_fetch(id, "RFC822").map_err(map_imap_error)?;
            fetches
                .iter()
                .find_map(|fetch| fetch.body())
                .map(<[u8]>::to_vec)
                .ok_or_else(|| MailError::other("message body was not returned by the IMAP server"))
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder, id),
                ConnectedImapSession::Tls(session) => exec(session, folder, id),
            })
    }

    pub fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
        ) -> MailResult<usize> {
            session.select(folder).map_err(map_imap_error)?;
            let unseen = session.uid_search("UNSEEN").map_err(map_imap_error)?;
            Ok(unseen.len())
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder),
                ConnectedImapSession::Tls(session) => exec(session, folder),
            })
    }
}
