/*! Reusable compose text snippets. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snippets (
             name       TEXT PRIMARY KEY,
             body       TEXT NOT NULL,
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    Ok(())
}
