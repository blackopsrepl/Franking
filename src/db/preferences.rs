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

/// Read a text preference.
pub fn get_text(conn: &Connection, key: &str) -> Result<Option<String>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .ok();
    Ok(value)
}

/// Store a text preference.
pub fn set_text(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .with_context(|| format!("cannot store preference {key}"))?;
    Ok(())
}

/// Read a numeric preference.
pub fn get_number(conn: &Connection, key: &str) -> Result<Option<usize>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .ok();
    Ok(value.and_then(|value| value.parse().ok()))
}

/// Store a numeric preference.
pub fn set_number(conn: &Connection, key: &str, value: usize) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value.to_string()],
    )
    .with_context(|| format!("cannot store preference {key}"))?;
    Ok(())
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
    fn round_trips_a_numeric_preference() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        assert_eq!(super::get_number(&conn, "page_size").unwrap(), None);
        super::set_number(&conn, "page_size", 100).unwrap();
        assert_eq!(super::get_number(&conn, "page_size").unwrap(), Some(100));
    }

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
