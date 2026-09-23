/*! Local message annotations: private notes and display-only subject aliases. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_notes (
             account    TEXT NOT NULL,
             folder     TEXT NOT NULL,
             uid        TEXT NOT NULL,
             message_id TEXT,
             body       TEXT NOT NULL,
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY (account, folder, uid)
         );
         CREATE TABLE IF NOT EXISTS subject_aliases (
             account    TEXT NOT NULL,
             anchor     TEXT NOT NULL,
             alias      TEXT NOT NULL,
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY (account, anchor)
         );",
    )?;
    Ok(())
}
