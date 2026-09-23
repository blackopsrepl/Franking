/*! Account-scoped conversation rules: quiet threads and resurfacing. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS conversation_rules (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             account      TEXT    NOT NULL,
             anchor       TEXT    NOT NULL,
             muted        INTEGER NOT NULL DEFAULT 0 CHECK(muted IN (0, 1)),
             resurface_at TEXT,
             created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
             UNIQUE(account, anchor)
         );
         CREATE INDEX IF NOT EXISTS idx_conversation_rules_muted
             ON conversation_rules(account, muted);
         CREATE INDEX IF NOT EXISTS idx_conversation_rules_resurface
             ON conversation_rules(account, resurface_at);",
    )?;
    Ok(())
}
