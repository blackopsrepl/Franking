/*! A per-conversation "always notify" flag beside muting. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    let exists: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('conversation_rules') WHERE name = 'loud'")?
        .exists([])?;
    if !exists {
        conn.execute_batch(
            "ALTER TABLE conversation_rules ADD COLUMN loud INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    Ok(())
}
