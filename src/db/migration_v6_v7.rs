/*! A per-message placement that overrides the sender's route for one message. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_routes (
             account    TEXT NOT NULL,
             folder     TEXT NOT NULL,
             uid        TEXT NOT NULL,
             message_id TEXT,
             route      TEXT NOT NULL,
             PRIMARY KEY (account, folder, uid)
         );
         CREATE INDEX IF NOT EXISTS idx_message_routes_account
             ON message_routes(account);",
    )?;
    Ok(())
}
