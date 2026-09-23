/* SQLite database layer. The database is stored under the app's data directory.
Every recognized schema upgrade runs inside a transaction before the app starts. */

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use super::schema::create_schema;
use super::schema_migrations::migrate;

/// Increment only with a corresponding migration in `schema_migrations`.
const SCHEMA_VERSION: u32 = 10;

/// Return the path to the database file.
pub fn db_path() -> PathBuf {
    crate::brand::data_dir().join("mail.db")
}

/// Open (or create) the database. Creates parent directories if needed.
pub fn open() -> Result<Connection> {
    let path = db_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create data directory: {}", parent.display()))?;
    }

    let conn = Connection::open(&path)
        .with_context(|| format!("cannot open database: {}", path.display()))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )
    .context("cannot configure pragmas")?;

    init_for_test(&conn).context("schema initialization failed")?;

    Ok(conn)
}

/// Initialize the current schema. Exposed for tests.
pub fn init_for_test(conn: &Connection) -> Result<()> {
    ensure_current_schema(conn)
}

fn ensure_current_schema(conn: &Connection) -> Result<()> {
    let version = stored_schema_version(conn)?;
    if let Some(version) = version {
        if version > SCHEMA_VERSION {
            bail!("database schema {version} is newer than supported schema {SCHEMA_VERSION}");
        }
        if version == 0 {
            bail!("database schema version 0 is not supported");
        }
    } else if !database_is_empty(conn)? {
        bail!("existing database has no schema version; refusing to overwrite it");
    }
    let tx = conn.unchecked_transaction()?;
    match version {
        None => {
            create_schema(&tx)?;
            set_schema_version(&tx, SCHEMA_VERSION)?;
        }
        Some(mut version) => {
            while version < SCHEMA_VERSION {
                migrate(&tx, version).with_context(|| format!("upgrading schema {version}"))?;
                version += 1;
                set_schema_version(&tx, version)?;
            }
        }
    }
    tx.commit()?;
    crate::mail::account_store::seed_defaults(conn)?;
    Ok(())
}

fn stored_schema_version(conn: &Connection) -> Result<Option<u32>> {
    if !table_exists(conn, "meta")? {
        return Ok(None);
    }

    use rusqlite::OptionalExtension;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let raw = raw.context("database meta table has no schema_version")?;
    Ok(Some(
        raw.parse().context("database schema_version is invalid")?,
    ))
}

fn database_is_empty(conn: &Connection) -> Result<bool> {
    Ok(!conn.prepare(
        "SELECT 1 FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' AND type IN ('table', 'view', 'index', 'trigger') LIMIT 1",
    )?.exists([])?)
}

fn set_schema_version(conn: &Connection, version: u32) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
        [version.to_string()],
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let exists = conn.query_row(
        "SELECT EXISTS(
             SELECT 1
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1
         )",
        [table_name],
        |row| row.get::<_, i64>(0),
    )? != 0;
    Ok(exists)
}

/// Schema version for diagnostics / tests.
pub fn schema_version(conn: &Connection) -> u32 {
    stored_schema_version(conn).ok().flatten().unwrap_or(0)
}

/// The expected schema version constant (exposed for tests).
pub const CURRENT_SCHEMA_VERSION: u32 = SCHEMA_VERSION;

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{init_for_test, schema_version, table_exists, CURRENT_SCHEMA_VERSION};

    #[test]
    fn init_creates_current_schema() {
        let conn = Connection::open_in_memory().unwrap();

        init_for_test(&conn).unwrap();

        assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&conn, "meta").unwrap());
        assert!(table_exists(&conn, "contacts").unwrap());
        assert!(table_exists(&conn, "contact_tags").unwrap());
        assert!(table_exists(&conn, "identities").unwrap());
        assert!(table_exists(&conn, "accounts").unwrap());
        assert!(table_exists(&conn, "account_endpoints").unwrap());
        assert!(table_exists(&conn, "auth_bindings").unwrap());
        assert!(table_exists(&conn, "oauth_states").unwrap());
        assert!(table_exists(&conn, "messages").unwrap());
        assert!(table_exists(&conn, "messages_fts").unwrap());
        assert!(table_exists(&conn, "sync_state").unwrap());
        assert!(!table_exists(&conn, "envelope_cache").unwrap());
    }

    #[test]
    fn an_unknown_future_schema_is_left_untouched() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             INSERT INTO meta (key, value) VALUES ('schema_version', '999');
             CREATE TABLE credentials (
                 id INTEGER PRIMARY KEY AUTOINCREMENT
             );",
        )
        .unwrap();

        assert!(init_for_test(&conn)
            .unwrap_err()
            .to_string()
            .contains("newer"));
        assert_eq!(schema_version(&conn), 999);
        assert!(table_exists(&conn, "credentials").unwrap());
    }
}
