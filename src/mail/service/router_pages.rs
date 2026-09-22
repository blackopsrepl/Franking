/*! Folder page listing with caching and offline fallback. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::types::Envelope;

use super::cache::{cached_envelopes, is_offline, record_listing};
use super::router::{route, RouterMailService};
use super::service_trait::MailService;

impl RouterMailService {
    /// Fetch a folder page, optionally in a requested order, caching the result
    /// and falling back to the local store when the server is unreachable.
    pub(super) fn list_envelopes_page(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
        order: Option<crate::mail::sort::SortOrder>,
    ) -> MailResult<Vec<Envelope>> {
        let record = self.choose_account(account)?;
        let result = match order {
            Some(order) => route!(self, Some(&record.name), service => {
                service.list_envelopes_sorted(account, folder, page, page_size, query, order)
            }),
            None => route!(self, Some(&record.name), service => {
                service.list_envelopes(account, folder, page, page_size, query)
            }),
        };

        match result {
            Ok(envelopes) => {
                let cursor = self
                    .folder_sync_cursor(Some(&record.name), folder)
                    .unwrap_or((None, None));
                let _ = self.with_db(|conn| {
                    record_listing(conn, &record.name, folder, &envelopes, cursor)
                        .map_err(|err| MailError::config_invalid(err.to_string()))
                });
                Ok(envelopes)
            }
            Err(error) if is_offline(&error) => self.with_db(|conn| {
                let mut envelopes =
                    cached_envelopes(conn, &record.name, folder, page, page_size, query)?;
                if let Some(order) = order {
                    order.apply(&mut envelopes);
                }
                Ok(envelopes)
            }),
            Err(error) => Err(error),
        }
    }
}
