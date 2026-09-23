/*! Local conversation merges. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS thread_merges (
             account TEXT NOT NULL,
             anchor  TEXT NOT NULL,
             root    TEXT NOT NULL,
             PRIMARY KEY (account, anchor)
         );",
    )?;
    Ok(())
}
