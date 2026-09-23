/*! Saved text excerpts from mail. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clips (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             account    TEXT,
             folder     TEXT,
             uid        TEXT NOT NULL,
             message_id TEXT,
             body       TEXT NOT NULL,
             source     TEXT NOT NULL DEFAULT '',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    Ok(())
}
