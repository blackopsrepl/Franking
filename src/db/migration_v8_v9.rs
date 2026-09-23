/*! Account policy: a revocable subject token that bypasses screening. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS account_policy (
             account      TEXT PRIMARY KEY,
             bypass_token TEXT
         );",
    )?;
    Ok(())
}
