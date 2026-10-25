/*! Explicit sender-to-server-folder rules. Legacy sender_routes are never read. */

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRule {
    pub account: String,
    pub sender: String,
    pub target: String,
    pub uid_validity: u32,
    pub first_uid: u32,
}

/// Additive local metadata: deliberately does not migrate legacy triage decisions.
pub fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS folder_rules (
            account TEXT NOT NULL,
            sender TEXT NOT NULL,
            target TEXT NOT NULL,
            uid_validity INTEGER NOT NULL CHECK(uid_validity > 0),
            first_uid INTEGER NOT NULL CHECK(first_uid > 0),
            PRIMARY KEY(account, sender)
        );",
    )?;
    Ok(())
}

pub fn save(conn: &Connection, account: &str, sender: &str, target: &str) -> Result<()> {
    ensure_schema(conn)?;
    let sender = sender.trim().to_ascii_lowercase();
    if account.is_empty() || !sender.contains('@') || sender.contains(char::is_whitespace) {
        bail!("A receiving account and sender mailbox are required");
    }
    if target.trim().is_empty()
        || target.eq_ignore_ascii_case("INBOX")
        || target == "All Inboxes"
        || target.contains(['\r', '\n', '\0'])
    {
        bail!("Choose a server folder other than INBOX");
    }
    // UIDNEXT excludes all mail already present, even old unseen/unfetched mail.
    // Never infer arrival from the untrusted sender-supplied Date header.
    let state = crate::mail::store::get_sync_state(conn, account, "INBOX")?
        .context("Refresh this account's INBOX before saving a future-mail rule")?;
    let validity = state
        .uid_validity
        .filter(|v| *v > 0)
        .context("Future-mail rules require an IMAP UIDVALIDITY")?;
    let first_uid = state
        .uid_next
        .filter(|v| *v > 0)
        .context("Refresh INBOX to obtain UIDNEXT before saving this rule")?;
    conn.execute(
        "INSERT INTO folder_rules(account,sender,target,uid_validity,first_uid)
         VALUES (?1,?2,?3,?4,?5) ON CONFLICT(account,sender) DO UPDATE SET
         target=excluded.target,uid_validity=excluded.uid_validity,first_uid=excluded.first_uid",
        params![account, sender, target, validity, first_uid],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<FolderRule>> {
    ensure_schema(conn)?;
    let mut stmt = conn.prepare(
        "SELECT account,sender,target,uid_validity,first_uid FROM folder_rules ORDER BY account,sender",
    )?;
    let rules = stmt
        .query_map([], |row| {
            Ok(FolderRule {
                account: row.get(0)?,
                sender: row.get(1)?,
                target: row.get(2)?,
                uid_validity: row.get(3)?,
                first_uid: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rules)
}

pub fn delete(conn: &Connection, account: &str, sender: &str) -> Result<bool> {
    ensure_schema(conn)?;
    Ok(conn.execute(
        "DELETE FROM folder_rules WHERE account=?1 AND sender=?2",
        params![account, sender.trim().to_ascii_lowercase()],
    )? > 0)
}

/// Fail closed on a mailbox reset, missing provenance, or pre-rule message.
pub fn target_for<'a>(
    conn: &Connection,
    rules: &'a [FolderRule],
    envelope: &crate::mail::types::Envelope,
) -> Result<Option<&'a str>> {
    let (Some(account), Some(folder), Some(sender)) = (
        envelope.account.as_deref(),
        envelope.folder.as_deref(),
        envelope.sender.mailbox(),
    ) else {
        return Ok(None);
    };
    if !folder.eq_ignore_ascii_case("INBOX") {
        return Ok(None);
    }
    let Some(uid) = envelope.id.parse::<u32>().ok() else {
        return Ok(None);
    };
    let Some(rule) = rules
        .iter()
        .find(|r| r.account == account && r.sender == sender)
    else {
        return Ok(None);
    };
    let state = crate::mail::store::get_sync_state(conn, account, folder)?;
    Ok(
        (uid >= rule.first_uid && state.and_then(|s| s.uid_validity) == Some(rule.uid_validity))
            .then_some(rule.target.as_str()),
    )
}

#[cfg(test)]
#[path = "folder_rules_tests.rs"]
mod tests;
