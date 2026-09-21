/*! Small key/value preferences stored in the `meta` table. */

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Read a preference, returning `default` when unset.
pub fn get(conn: &Connection, key: &str, default: bool) -> Result<bool> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .ok();
    Ok(match value.as_deref() {
        Some("off") | Some("false") | Some("0") => false,
        Some(_) => true,
        None => default,
    })
}

/// Store a boolean preference.
pub fn set(conn: &Connection, key: &str, value: bool) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, if value { "on" } else { "off" }],
    )
    .with_context(|| format!("cannot store preference {key}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{get, set};

    #[test]
    fn round_trips_a_boolean_preference() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        assert!(
            get(&conn, "notifications", true).unwrap(),
            "default is used"
        );
        set(&conn, "notifications", false).unwrap();
        assert!(!get(&conn, "notifications", true).unwrap());
        set(&conn, "notifications", true).unwrap();
        assert!(get(&conn, "notifications", false).unwrap());
    }
}
