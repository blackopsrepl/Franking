/*! Named collections of conversations. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS collections (
             id      INTEGER PRIMARY KEY AUTOINCREMENT,
             account TEXT    NOT NULL,
             name    TEXT    NOT NULL,
             UNIQUE(account, name)
         );
         CREATE TABLE IF NOT EXISTS collection_items (
             account       TEXT    NOT NULL,
             collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
             anchor        TEXT    NOT NULL,
             PRIMARY KEY (account, collection_id, anchor)
         );",
    )?;
    Ok(())
}
