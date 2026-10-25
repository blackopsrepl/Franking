/*! Account-scoped conversation mute.
A conversation is identified by the message-id anchors observable in an
envelope: its own Message-ID and the Message-ID it replies to. Muting records
every anchor, so a direct reply matches even when the original is outside the
loaded page. Deeper replies whose immediate parent is not loaded fall back to
their own Message-ID and may not match. The table also carries retired loud
and resurface columns; their rows are never deleted by an unmute. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::types::Envelope;

/// Anchors that identify a conversation from a single envelope.
pub fn anchors(envelope: &Envelope) -> Vec<String> {
    let mut anchors = Vec::new();
    if let Some(id) = envelope.message_id.as_ref().filter(|id| !id.is_empty()) {
        anchors.push(id.clone());
    }
    if let Some(id) = envelope.in_reply_to.as_ref().filter(|id| !id.is_empty()) {
        if !anchors.contains(id) {
            anchors.push(id.clone());
        }
    }
    if anchors.is_empty() {
        anchors.push(format!("uid:{}", envelope.id));
    }
    anchors
}

/// Anchors for every envelope, resolving roots against one shared index.
///
/// Building the Message-ID index once keeps a full-page load linear instead of
/// rebuilding it for each row.
pub fn anchors_for_list(envelopes: &[Envelope]) -> Vec<Vec<String>> {
    let by_id: std::collections::HashMap<&str, usize> = envelopes
        .iter()
        .enumerate()
        .filter_map(|(i, envelope)| envelope.message_id.as_deref().map(|id| (id, i)))
        .collect();
    envelopes
        .iter()
        .enumerate()
        .map(|(index, envelope)| {
            let mut anchors = anchors(envelope);
            if let Some(root) = resolved_root_with(&by_id, envelopes, index) {
                if !anchors.contains(&root) {
                    anchors.push(root);
                }
            }
            anchors
        })
        .collect()
}

fn resolved_root_with(
    by_id: &std::collections::HashMap<&str, usize>,
    envelopes: &[Envelope],
    index: usize,
) -> Option<String> {
    let mut current = index;
    let mut hops = 0;
    while hops < 8 {
        let Some(parent) = envelopes[current].in_reply_to.as_deref() else {
            break;
        };
        let Some(&parent_index) = by_id.get(parent) else {
            break;
        };
        current = parent_index;
        hops += 1;
    }
    envelopes[current].message_id.clone()
}

/// Every muted anchor for one account, loaded once per list refresh.
pub fn muted_anchors(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT anchor FROM conversation_rules WHERE account = ?1 AND muted = 1")?;
    let rows = stmt.query_map([account], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Whether any anchor of the conversation is muted.
pub fn is_muted(conn: &Connection, account: &str, anchors: &[String]) -> Result<bool> {
    for anchor in anchors {
        let muted: bool = conn
            .query_row(
                "SELECT muted FROM conversation_rules
                 WHERE account = ?1 AND anchor = ?2",
                params![account, anchor],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some_and(|value| value != 0);
        if muted {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Mute or unmute every anchor of a conversation.
pub fn set_muted(conn: &Connection, account: &str, anchors: &[String], muted: bool) -> Result<()> {
    for anchor in anchors {
        conn.execute(
            "INSERT INTO conversation_rules (account, anchor, muted) VALUES (?1, ?2, ?3)
             ON CONFLICT(account, anchor) DO UPDATE SET muted = excluded.muted",
            params![account, anchor, if muted { 1 } else { 0 }],
        )?;
    }
    if !muted {
        for anchor in anchors {
            conn.execute(
                "DELETE FROM conversation_rules WHERE account = ?1 AND anchor = ?2
                 AND muted = 0 AND loud = 0 AND resurface_at IS NULL",
                params![account, anchor],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
