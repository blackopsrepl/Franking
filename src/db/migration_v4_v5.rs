/*! Durable per-message follow-up and reference markers. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_markers (
             account TEXT NOT NULL,
             folder TEXT NOT NULL,
             uid TEXT NOT NULL,
             message_id TEXT,
             reply_later INTEGER NOT NULL DEFAULT 0 CHECK(reply_later IN (0, 1)),
             saved INTEGER NOT NULL DEFAULT 0 CHECK(saved IN (0, 1)),
             PRIMARY KEY (account, folder, uid)
         );
         CREATE INDEX IF NOT EXISTS idx_message_markers_account
             ON message_markers(account, reply_later, saved);",
    )?;
    Ok(())
}
