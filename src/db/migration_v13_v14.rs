/*! Account-scoped workflow stages. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS stages (
             id       INTEGER PRIMARY KEY AUTOINCREMENT,
             account  TEXT    NOT NULL,
             name     TEXT    NOT NULL,
             position INTEGER NOT NULL DEFAULT 0,
             UNIQUE(account, name)
         );
         CREATE TABLE IF NOT EXISTS conversation_stages (
             account    TEXT    NOT NULL,
             anchor     TEXT    NOT NULL,
             stage_id   INTEGER NOT NULL REFERENCES stages(id) ON DELETE CASCADE,
             updated_at TEXT    NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY (account, anchor)
         );",
    )?;
    Ok(())
}
