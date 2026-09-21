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
        self.ensure_requested_account(account)?;
        let criteria = search_criteria(query);

        let threaded = self.pool.with_session(&self.account, |session| {
            if !session.capabilities().thread {
                return Ok(None);
            }
            match session.connection() {
                ConnectedImapSession::Plain(session) => {
                    session.select(folder).map_err(map_imap_error)?;
                    Ok(Some(thread_uids(session, &criteria)?))
                }
                ConnectedImapSession::Tls(session) => {
                    session.select(folder).map_err(map_imap_error)?;
                    Ok(Some(thread_uids(session, &criteria)?))
                }
            }
        })?;

        let Some(uids) = threaded else {
            return self.list_envelopes(account, folder, 1, usize::MAX, query);
        };
        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let mut envelopes =
            self.pool
                .with_connection(&self.account, |connection| match connection {
                    ConnectedImapSession::Plain(session) => {
                        session.select(folder).map_err(map_imap_error)?;
                        fetch_envelope_metadata(session, &uids)
                    }
                    ConnectedImapSession::Tls(session) => {
                        session.select(folder).map_err(map_imap_error)?;
                        fetch_envelope_metadata(session, &uids)
                    }
                })?;

        let order: std::collections::HashMap<u32, usize> = uids
            .iter()
            .enumerate()
            .map(|(index, uid)| (*uid, index))
            .collect();
        envelopes.sort_by_key(|envelope| {
            order
                .get(&envelope.id.parse::<u32>().unwrap_or_default())
                .copied()
                .unwrap_or(usize::MAX)
        });
        Ok(envelopes)
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

        let mailbox = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => {
                    session.status(folder, "(UNSEEN)").map_err(map_imap_error)
                }
                ConnectedImapSession::Tls(session) => {
                    session.status(folder, "(UNSEEN)").map_err(map_imap_error)
                }
            })?;
        Ok(mailbox.unseen.unwrap_or(0) as usize)
    }

    /// Fetch every envelope in a folder for caching.
    pub fn sync_folder(&self, account: Option<&str>, folder: &str) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;

        fn exec<S: Read + Write>(
            session: &mut imap::Session<S>,
            folder: &str,
        ) -> MailResult<Vec<Envelope>> {
            session.select(folder).map_err(map_imap_error)?;
            let uids = session
                .uid_search("ALL")
                .map_err(map_imap_error)?
                .into_iter()
                .collect::<Vec<_>>();
            fetch_envelope_metadata(session, &uids)
        }

        self.pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => exec(session, folder),
                ConnectedImapSession::Tls(session) => exec(session, folder),
            })
    }

    /// Build a resume template from a stored draft message.
    pub fn draft_template(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        let raw = self.read_message_raw(account, folder, id)?;
        let document = crate::mail::mime::parse_message(&raw)?;
        Ok(crate::mail::draft::draft_template(&document))
    }

    /// UIDVALIDITY and UIDNEXT for a folder, used to seed sync cursors.
    pub fn folder_sync_cursor(
        &self,
        account: Option<&str>,
        folder: &str,
    ) -> MailResult<(Option<u32>, Option<u32>)> {
        self.ensure_requested_account(account)?;

        let mailbox = self
            .pool
            .with_connection(&self.account, |connection| match connection {
                ConnectedImapSession::Plain(session) => {
                    session.examine(folder).map_err(map_imap_error)
                }
                ConnectedImapSession::Tls(session) => {
                    session.examine(folder).map_err(map_imap_error)
                }
            })?;
        Ok((mailbox.uid_validity, mailbox.uid_next))
    }
}

/// Run `UID THREAD REFERENCES` and flatten the thread groups into UID order.
fn thread_uids<S: Read + Write>(
    session: &mut imap::Session<S>,
    criteria: &str,
) -> MailResult<Vec<u32>> {
    let response = session
        .run_command_and_read_response(format!("UID THREAD REFERENCES UTF-8 {criteria}"))
        .map_err(map_imap_error)?;
    Ok(parse_thread_response(&String::from_utf8_lossy(&response)))
}

fn parse_thread_response(response: &str) -> Vec<u32> {
    let mut uids = Vec::new();
    for line in response.lines() {
        let Some(rest) = line.strip_prefix("* THREAD") else {
            continue;
        };
        for token in rest.split(|ch: char| !ch.is_ascii_digit()) {
            if let Ok(uid) = token.parse() {
                uids.push(uid);
            }
        }
    }
    uids
}
