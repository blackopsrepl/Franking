/*! Ordered upgrades of installed databases. Version changes are atomic with data. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn migrate(conn: &Connection, from: u32) -> Result<()> {
    match from {
        1 => super::migration_v1_v2::apply(conn),
        2 => super::migration_v2_v3::apply(conn),
        3 => repair_v3(conn),
        4 => super::migration_v4_v5::apply(conn),
        5 => super::migration_v5_v6::apply(conn),
        6 => super::migration_v6_v7::apply(conn),
        7 => super::migration_v7_v8::apply(conn),
        8 => super::migration_v8_v9::apply(conn),
        9 => super::migration_v9_v10::apply(conn),
        _ => anyhow::bail!("no migration from schema version {from}"),
    }
}

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
fn repair_v3(conn: &Connection) -> Result<()> {
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
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sender_routes (
             account TEXT NOT NULL,
             sender  TEXT NOT NULL,
             route   TEXT NOT NULL CHECK(route IN ('inbox', 'reading', 'receipts', 'blocked')),
             PRIMARY KEY (account, sender)
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
