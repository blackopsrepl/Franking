/*! Sender bundling decisions. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS bundles (
             account TEXT NOT NULL,
             sender  TEXT NOT NULL,
             PRIMARY KEY (account, sender)
         );",
    )?;
    Ok(())
}
