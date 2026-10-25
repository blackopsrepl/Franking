/*! Explicit sender folder rules with suspension state.

The first rules build created `folder_rules` at runtime without a version bump,
so an installed v16 database may already hold rules; keep them and add the
suspension column instead of recreating the table. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS folder_rules (
             account          TEXT    NOT NULL,
             sender           TEXT    NOT NULL,
             target           TEXT    NOT NULL,
             uid_validity     INTEGER NOT NULL CHECK(uid_validity > 0),
             first_uid        INTEGER NOT NULL CHECK(first_uid > 0),
             suspended_reason TEXT,
             PRIMARY KEY (account, sender)
         );",
    )?;
    let has_reason: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('folder_rules') WHERE name = 'suspended_reason'")?
        .exists([])?;
    if !has_reason {
        conn.execute_batch("ALTER TABLE folder_rules ADD COLUMN suspended_reason TEXT;")?;
    }
    Ok(())
}
