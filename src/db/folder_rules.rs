/*! Explicit sender-to-server-folder rules. Legacy sender_routes are never read.
The table is created by schema migration v16→v17. */

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

/// Stored when the server reports a different INBOX UIDVALIDITY than the rule's.
pub const MAILBOX_RESET: &str = "mailbox was reset";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRule {
    pub account: String,
    pub sender: String,
    pub target: String,
    pub uid_validity: u32,
    pub first_uid: u32,
    /// Why routing stopped. A suspended rule is never applied.
    pub suspended: Option<String>,
}

/// The receiving account's stored INBOX boundary: (UIDVALIDITY, UIDNEXT).
/// UIDNEXT excludes all mail already present, even old unseen/unfetched mail.
/// Never infer arrival from the untrusted sender-supplied Date header.
fn boundary(conn: &Connection, account: &str) -> Result<(u32, u32)> {
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
    Ok((validity, first_uid))
}

/// Validate a rule and capture its boundary now; nothing is stored yet.
pub fn draft(conn: &Connection, account: &str, sender: &str, target: &str) -> Result<FolderRule> {
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
    let (uid_validity, first_uid) = boundary(conn, account)?;
    Ok(FolderRule {
        account: account.into(),
        sender,
        target: target.into(),
        uid_validity,
        first_uid,
        suspended: None,
    })
}

/// Store a drafted rule, replacing any rule for the same account and sender.
pub fn save(conn: &Connection, rule: &FolderRule) -> Result<()> {
    conn.execute(
        "INSERT INTO folder_rules(account,sender,target,uid_validity,first_uid,suspended_reason)
         VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(account,sender) DO UPDATE SET
         target=excluded.target,uid_validity=excluded.uid_validity,
         first_uid=excluded.first_uid,suspended_reason=excluded.suspended_reason",
        params![
            rule.account,
            rule.sender,
            rule.target,
            rule.uid_validity,
            rule.first_uid,
            rule.suspended
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<FolderRule>> {
    let mut stmt = conn.prepare(
        "SELECT account,sender,target,uid_validity,first_uid,suspended_reason
         FROM folder_rules ORDER BY account,sender",
    )?;
    let rules = stmt
        .query_map([], |row| {
            Ok(FolderRule {
                account: row.get(0)?,
                sender: row.get(1)?,
                target: row.get(2)?,
                uid_validity: row.get(3)?,
                first_uid: row.get(4)?,
                suspended: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rules)
}

pub fn delete(conn: &Connection, account: &str, sender: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM folder_rules WHERE account=?1 AND sender=?2",
        params![account, sender.trim().to_ascii_lowercase()],
    )? > 0)
}

/// Stop applying a rule until the user resumes it.
pub fn suspend(conn: &Connection, account: &str, sender: &str, reason: &str) -> Result<bool> {
    Ok(conn.execute(
        "UPDATE folder_rules SET suspended_reason=?3 WHERE account=?1 AND sender=?2",
        params![account, sender, reason],
    )? > 0)
}

/// Re-arm a suspended rule at the server's live INBOX boundary, so mail that
/// arrived while it was suspended is never swept up.
pub fn resume(
    conn: &Connection,
    account: &str,
    sender: &str,
    (uid_validity, first_uid): (u32, u32),
) -> Result<()> {
    let changed = conn.execute(
        "UPDATE folder_rules SET uid_validity=?3,first_uid=?4,suspended_reason=NULL
         WHERE account=?1 AND sender=?2",
        params![account, sender, uid_validity, first_uid],
    )?;
    if changed == 0 {
        bail!("The rule no longer exists");
    }
    Ok(())
}

/// Follow a server folder rename, including its subfolders when the server
/// has a hierarchy `delimiter`. Returns the number of rules updated.
pub fn retarget(
    conn: &Connection,
    account: &str,
    from: &str,
    to: &str,
    delimiter: Option<char>,
) -> Result<usize> {
    let mut updated = conn.execute(
        "UPDATE folder_rules SET target=?3 WHERE account=?1 AND target=?2",
        params![account, from, to],
    )?;
    if let Some(delimiter) = delimiter {
        let parent = format!("{from}{delimiter}");
        updated += conn.execute(
            "UPDATE folder_rules SET target = ?3 || substr(target, length(?2) + 1)
             WHERE account=?1 AND substr(target, 1, length(?2)) = ?2",
            params![account, parent, format!("{to}{delimiter}")],
        )?;
    }
    Ok(updated)
}

/// Drop rules whose server folder was deleted. Returns the number removed.
pub fn delete_targeting(conn: &Connection, account: &str, folder: &str) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM folder_rules WHERE account=?1 AND target=?2",
        params![account, folder],
    )?)
}

#[cfg(test)]
#[path = "folder_rules_tests.rs"]
mod tests;
