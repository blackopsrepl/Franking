/*! Folder and envelope listing plus raw message reads. */

use crate::mail::errors::MailResult;
use crate::mail::types::{Envelope, Folder};

use super::model::ImapSmtpService;
use super::next;

impl ImapSmtpService {
    pub fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.ensure_requested_account(account)?;
        self.pool.with_client(&self.account, next::list_folders)
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

        let mut envelopes = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            let mut uids = next::search_uids(client, next::search::criteria(query))?;
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
            next::fetch_envelopes(client, &page_uids)
        })?;
        self.tag(&mut envelopes, folder);
        Ok(envelopes)
    }

    /// List a page in a requested order, using server-side SORT.
    ///
    /// SORT orders the whole folder on the server before paging; servers that
    /// do not implement it fall back to arrival order, and the caller's local
    /// ordering still applies to the page.
    pub fn list_envelopes_sorted(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
        order: crate::mail::sort::SortOrder,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;

        let mut envelopes = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            let criteria = next::search::criteria(query);
            let uids = match next::sort_uids(client, order.criteria(), criteria.clone()) {
                Ok(uids) => uids,
                Err(error) if error.is_transport() => return Err(error),
                Err(_) => {
                    // No SORT support: arrival order, newest first.
                    let mut uids = next::search_uids(client, criteria)?;
                    uids.sort_unstable_by(|left, right| right.cmp(left));
                    uids
                }
            };
            let start = page.saturating_sub(1) * page_size;
            let page_uids = uids
                .into_iter()
                .skip(start)
                .take(page_size)
                .collect::<Vec<_>>();
            next::fetch_envelopes(client, &page_uids)
        })?;
        self.tag(&mut envelopes, folder);
        Ok(envelopes)
    }

    pub fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;

        let uids = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            let groups = next::thread_uids(
                client,
                imap_types::extensions::thread::ThreadingAlgorithm::References,
                next::search::criteria(query),
            )?;
            let mut uids: Vec<u32> = groups.into_iter().flatten().collect();
            uids.dedup();
            Ok(uids)
        })?;
        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let mut envelopes = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::fetch_envelopes(client, &uids)
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
        self.tag(&mut envelopes, folder);
        Ok(envelopes)
    }

    pub fn read_message_raw(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        self.ensure_requested_account(account)?;
        let uid = id.parse::<u32>().map_err(|_| {
            crate::mail::errors::MailError::invalid_input(format!("invalid message id {id}"))
        })?;
        self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            next::read_message_raw(client, uid)
        })
    }

    /// Tag envelopes with their source account and folder.
    fn tag(&self, envelopes: &mut [Envelope], folder: &str) {
        for envelope in envelopes {
            envelope.account = Some(self.account.name.clone());
            envelope.folder = Some(folder.to_string());
        }
    }

    pub fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        self.ensure_requested_account(account)?;
        let status = self
            .pool
            .with_client(&self.account, |client| next::status(client, folder))?;
        Ok(status.unseen.unwrap_or(0) as usize)
    }

    /// Fetch every envelope in a folder for caching.
    pub fn sync_folder(&self, account: Option<&str>, folder: &str) -> MailResult<Vec<Envelope>> {
        self.ensure_requested_account(account)?;
        let mut envelopes = self.pool.with_client(&self.account, |client| {
            next::select(client, folder)?;
            let uids = next::search_uids(client, next::search::criteria(None))?;
            next::fetch_envelopes(client, &uids)
        })?;
        self.tag(&mut envelopes, folder);
        Ok(envelopes)
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
        let status = self
            .pool
            .with_client(&self.account, |client| next::status(client, folder))?;
        Ok((status.uid_validity, status.uid_next))
    }
}
