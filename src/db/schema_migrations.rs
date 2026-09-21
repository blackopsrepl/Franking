/*! Non-destructive column additions for databases created by an older
schema. */

use anyhow::Result;
use rusqlite::Connection;

/// The table and column named by an `ALTER TABLE … ADD COLUMN …` statement.
fn added_column(statement: &str) -> Option<(&str, &str)> {
    let table = statement
        .strip_prefix("ALTER TABLE ")?
        .split_whitespace()
        .next()?;
    let column = statement
        .split("ADD COLUMN ")
        .nth(1)?
        .split_whitespace()
        .next()?;
    Some((table, column))
}

/// Add columns introduced after the first schema without resetting local data.
pub(super) fn migrate_schema(conn: &Connection) -> Result<()> {
    for column in ["signature TEXT", "sent_folder TEXT"] {
        let name = column.split_whitespace().next().unwrap_or_default();
        let exists: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('identities') WHERE name = ?1")?
            .exists([name])?;
        if !exists {
            conn.execute_batch(&format!("ALTER TABLE identities ADD COLUMN {column};"))?;
        }
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS outbox (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             account    TEXT,
             template   TEXT    NOT NULL,
             sign           INTEGER NOT NULL DEFAULT 0,
             encrypt        INTEGER NOT NULL DEFAULT 0,
             smime_sign     INTEGER NOT NULL DEFAULT 0,
             smime_encrypt  INTEGER NOT NULL DEFAULT 0,
             send_after     TEXT,
             created_at TEXT    NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    let has_send_after: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('outbox') WHERE name = 'send_after'")?
        .exists([])?;
    if !has_send_after {
        conn.execute_batch("ALTER TABLE outbox ADD COLUMN send_after TEXT;")?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_searches (
             name       TEXT PRIMARY KEY,
             query      TEXT NOT NULL,
             scope      TEXT NOT NULL DEFAULT 'folder',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    for column in [
        "ALTER TABLE outbox ADD COLUMN smime_sign INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE outbox ADD COLUMN smime_encrypt INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE outbox ADD COLUMN sent_folder TEXT;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_host TEXT;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_port INTEGER;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_security TEXT;",
    ] {
        let Some((table, name)) = added_column(column) else {
            continue;
        };
        // Guard against the column's own table: an earlier version checked
        // every statement against one table, so adding a column to any other
        // table ran an ALTER that the CREATE had already covered.
        let exists: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
            .exists([table, name])?;
        if !exists {
            conn.execute_batch(column)?;
        }
    }
    Ok(())
}
