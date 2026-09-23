/*! Account-scoped conversation rules: quiet threads and resurfacing.
A conversation is identified by the message-id anchors observable in an
envelope: its own Message-ID and the Message-ID it replies to. Muting or
resurfacing a conversation records a rule for each anchor, so a direct reply
matches even when the original is outside the loaded page. Deeper replies whose
immediate parent is not loaded fall back to their own Message-ID and may not
match, which the UI states. */

use anyhow::{Context, Result};
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

/// A conversation's local decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rule {
    pub muted: bool,
    pub loud: bool,
    pub resurface_at: Option<String>,
}

/// Every conversation rule for one account, keyed by anchor.
///
/// A list evaluates muted/resurface per envelope; loading the account's rules
/// once avoids three queries per row on every refresh.
pub fn rules_for_account(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashMap<String, Rule>> {
    let mut stmt = conn.prepare(
        "SELECT anchor, muted, loud, resurface_at FROM conversation_rules WHERE account = ?1",
    )?;
    let rows = stmt.query_map([account], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)? != 0,
            row.get::<_, i64>(2)? != 0,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut rules = std::collections::HashMap::new();
    for row in rows {
        let (anchor, muted, loud, resurface_at) = row?;
        rules.insert(
            anchor,
            Rule {
                muted,
                loud,
                resurface_at,
            },
        );
    }
    Ok(rules)
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
        delete_empty(conn, account, anchors)?;
    }
    Ok(())
}

/// Set or clear the always-notify flag on every anchor of a conversation.
pub fn set_loud(conn: &Connection, account: &str, anchors: &[String], loud: bool) -> Result<()> {
    for anchor in anchors {
        conn.execute(
            "INSERT INTO conversation_rules (account, anchor, loud) VALUES (?1, ?2, ?3)
             ON CONFLICT(account, anchor) DO UPDATE SET loud = excluded.loud",
            params![account, anchor, if loud { 1 } else { 0 }],
        )?;
    }
    if !loud {
        delete_empty(conn, account, anchors)?;
    }
    Ok(())
}

/// Whether any anchor of the conversation asks to always notify.
pub fn is_loud(conn: &Connection, account: &str, anchors: &[String]) -> Result<bool> {
    for anchor in anchors {
        let loud: bool = conn
            .query_row(
                "SELECT loud FROM conversation_rules WHERE account = ?1 AND anchor = ?2",
                params![account, anchor],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some_and(|value| value != 0);
        if loud {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The earliest resurface time recorded for the conversation, if any.
pub fn resurface_at(
    conn: &Connection,
    account: &str,
    anchors: &[String],
) -> Result<Option<String>> {
    for anchor in anchors {
        if let Some(at) = conn
            .query_row(
                "SELECT resurface_at FROM conversation_rules
                 WHERE account = ?1 AND anchor = ?2 AND resurface_at IS NOT NULL",
                params![account, anchor],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(Some(at));
        }
    }
    Ok(None)
}

/// Set or clear the resurface time for every anchor of a conversation.
pub fn set_resurface(
    conn: &Connection,
    account: &str,
    anchors: &[String],
    at: Option<&str>,
) -> Result<()> {
    for anchor in anchors {
        conn.execute(
            "INSERT INTO conversation_rules (account, anchor, resurface_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(account, anchor) DO UPDATE SET resurface_at = excluded.resurface_at",
            params![account, anchor, at],
        )
        .with_context(|| format!("cannot set resurface time for {anchor}"))?;
    }
    if at.is_none() {
        delete_empty(conn, account, anchors)?;
    }
    Ok(())
}

/// Remove rule rows that no longer carry any decision.
fn delete_empty(conn: &Connection, account: &str, anchors: &[String]) -> Result<()> {
    for anchor in anchors {
        conn.execute(
            "DELETE FROM conversation_rules
             WHERE account = ?1 AND anchor = ?2 AND muted = 0 AND loud = 0 AND resurface_at IS NULL",
            params![account, anchor],
        )?;
    }
    Ok(())
}

/// Whether the conversation is due to resurface at `now` (UTC RFC 3339).
pub fn is_due(conn: &Connection, account: &str, anchors: &[String], now: &str) -> Result<bool> {
    Ok(resurface_at(conn, account, anchors)?.is_some_and(|at| at.as_str() <= now))
}

#[cfg(test)]
mod tests;
