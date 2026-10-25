/*! Explicit rules applied at fetch entry, before envelopes reach the UI. */

use crate::db::folder_rules;
use crate::mail::errors::MailResult;
use crate::mail::types::Envelope;
use crate::mail::{MailError, MailService};
use rusqlite::Connection;

/// Only envelopes carrying their actual receiving account + INBOX source qualify.
/// Failures surface to the caller instead of silently hiding mail that did not move.
pub(super) fn apply(
    service: &dyn MailService,
    envelopes: Vec<Envelope>,
) -> MailResult<Vec<Envelope>> {
    let conn = crate::db::open().map_err(|e| MailError::other(format!("Folder rules: {e}")))?;
    apply_with(&conn, envelopes, |account, folder, target, id| {
        service.move_message(Some(account), folder, target, id)
    })
}

pub(super) fn apply_with(
    conn: &Connection,
    envelopes: Vec<Envelope>,
    mut move_message: impl FnMut(&str, &str, &str, &str) -> MailResult<()>,
) -> MailResult<Vec<Envelope>> {
    let rules =
        folder_rules::list(conn).map_err(|e| MailError::other(format!("Folder rules: {e}")))?;
    let mut visible = Vec::with_capacity(envelopes.len());
    for envelope in envelopes {
        let target = folder_rules::target_for(conn, &rules, &envelope)
            .map_err(|e| MailError::other(format!("Folder rules: {e}")))?;
        if let Some(target) = target {
            // target_for established provenance; never use UI's current account/folder.
            move_message(
                envelope.account.as_deref().unwrap(),
                envelope.folder.as_deref().unwrap(),
                target,
                &envelope.id,
            )
            .map_err(|e| {
                MailError::new(
                    e.kind,
                    format!("Could not route {} to {target}: {}", envelope.id, e.detail),
                )
            })?;
        } else {
            visible.push(envelope);
        }
    }
    Ok(visible)
}

#[cfg(test)]
#[path = "folder_rules_live.rs"]
mod live_tests;

#[cfg(test)]
#[path = "folder_rule_tests.rs"]
mod tests;
