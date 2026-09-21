/*! Offline fallbacks backed by the local store. */

use super::super::errors::{MailError, MailErrorKind, MailResult};
use super::super::store::{self, StoredMessage};
use super::super::types::Envelope;

pub(super) fn is_offline(error: &MailError) -> bool {
    error.is_transport() || error.kind == MailErrorKind::BackendUnavailable
}

/// Serve a folder listing or search from the local store.
pub(super) fn cached_envelopes(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    page: usize,
    page_size: usize,
    query: Option<&str>,
) -> MailResult<Vec<Envelope>> {
    let offset = page.saturating_sub(1) * page_size;
    let messages = match query.map(str::trim).filter(|query| !query.is_empty()) {
        Some(query) => store::search_messages(conn, Some(account), query, usize::MAX)
            .map_err(|err| MailError::config_invalid(err.to_string()))?
            .into_iter()
            .filter(|message| message.folder == folder)
            .skip(offset)
            .take(page_size)
            .collect::<Vec<_>>(),
        None => store::list_messages(conn, account, folder, page_size, offset)
            .map_err(|err| MailError::config_invalid(err.to_string()))?,
    };
    Ok(messages.iter().map(StoredMessage::to_envelope).collect())
}
