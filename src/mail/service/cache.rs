/*! Offline fallbacks backed by the local store. */

use super::super::errors::{MailError, MailErrorKind, MailResult};
use super::super::store::{self, StoredMessage, SyncState};
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

/// Persist envelope metadata for a freshly listed folder.
pub(super) fn cache_envelopes(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    envelopes: &[Envelope],
) -> anyhow::Result<()> {
    for envelope in envelopes {
        store::upsert_envelope(
            conn,
            &StoredMessage::from_envelope(account, folder, envelope),
        )?;
    }
    Ok(())
}

/// Record a folder's UIDVALIDITY/UIDNEXT resync anchor.
pub(super) fn record_sync_cursor(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    uid_validity: Option<u32>,
    uid_next: Option<u32>,
) -> anyhow::Result<()> {
    store::set_sync_state(
        conn,
        &SyncState {
            account: account.to_string(),
            folder: folder.to_string(),
            uid_validity,
            uid_next,
            highest_modseq: None,
            last_synced_at: None,
        },
    )
}

/// Cache a freshly read message, preserving flags and marking it Seen.
pub(super) fn cache_message(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    uid: &str,
    raw: &[u8],
    document: &crate::mail::model::MessageDocument,
) -> anyhow::Result<()> {
    let mut flags = store::get_message(conn, account, folder, uid)?
        .map(|message| message.flags)
        .unwrap_or_default();
    if !flags.iter().any(|flag| flag.eq_ignore_ascii_case("seen")) {
        flags.push("Seen".to_string());
    }
    let stored = StoredMessage::from_document(
        account,
        folder,
        uid,
        None,
        &flags,
        document,
        Some(raw.to_vec()),
    );
    store::upsert_message(conn, &stored)
}
