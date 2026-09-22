/*! Whole-flag-list updates, as reported by CHANGEDSINCE. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

/// Replace the cached flags of a message with the server's current set.
pub fn set_flags(
    conn: &Connection,
    account: &str,
    folder: &str,
    uid: &str,
    flags: &[String],
) -> Result<()> {
    conn.execute(
        "UPDATE messages SET flags = ?4, updated_at = datetime('now')
         WHERE account = ?1 AND folder = ?2 AND uid = ?3",
        params![account, folder, uid, flags.join(" ")],
    )
    .context("failed to replace cached flags")?;
    Ok(())
}
