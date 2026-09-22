/*! Offline fallbacks backed by the local store. */

use super::super::errors::{MailError, MailErrorKind, MailResult};
use super::super::remote::next::SyncAnchor;
use super::super::store::{self, StoredMessage, SyncState};
use super::super::types::{Envelope, FolderDelta};

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

/// The stored anchor a delta sync can resume from, when one is complete.
pub(super) fn stored_anchor(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
) -> Option<SyncAnchor> {
    let state = store::get_sync_state(conn, account, folder)
        .ok()
        .flatten()?;
    Some(SyncAnchor {
        uid_validity: state.uid_validity?,
        highest_modseq: state.highest_modseq?,
    })
}

/// Apply a server delta to the cached folder and advance its anchor.
///
/// A changed UIDVALIDITY means the cached UIDs no longer refer to the same
/// messages, so the caller must fall back to a full listing instead.
pub(super) fn apply_folder_delta(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    delta: &FolderDelta,
) -> anyhow::Result<()> {
    let previous = store::get_sync_state(conn, account, folder)?;
    if let (Some(previous), Some(reported)) = (&previous, delta.uid_validity) {
        if previous.uid_validity.is_some_and(|value| value != reported) {
            for message in store::list_messages(conn, account, folder, usize::MAX, 0)? {
                store::delete_message(conn, account, folder, &message.uid)?;
            }
        }
    }

    for uid in &delta.vanished {
        store::delete_message(conn, account, folder, &uid.to_string())?;
    }
    for (uid, flags) in &delta.changed_flags {
        if store::get_message(conn, account, folder, &uid.to_string())?.is_some() {
            store::set_flags(conn, account, folder, &uid.to_string(), flags)?;
        }
    }

    let uid_validity = delta
        .uid_validity
        .or_else(|| previous.as_ref().and_then(|state| state.uid_validity));
    let uid_next = delta
        .uid_next
        .or_else(|| previous.as_ref().and_then(|state| state.uid_next));
    let highest_modseq = delta
        .highest_modseq
        .or_else(|| previous.as_ref().and_then(|state| state.highest_modseq));
    store::set_sync_state(
        conn,
        &SyncState {
            account: account.to_string(),
            folder: folder.to_string(),
            uid_validity,
            uid_next,
            highest_modseq,
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

/// Cache a listing and its sync cursor together.
pub(super) fn record_listing(
    conn: &rusqlite::Connection,
    account: &str,
    folder: &str,
    envelopes: &[Envelope],
    cursor: (Option<u32>, Option<u32>),
) -> anyhow::Result<()> {
    cache_envelopes(conn, account, folder, envelopes)?;
    record_sync_cursor(conn, account, folder, cursor.0, cursor.1)
}
