/*! Sender bundling: keep a high-volume sender to one row in a lane. */

use anyhow::Result;
use rusqlite::{params, Connection};

/// Normalized sender mailboxes bundled for an account.
pub fn bundled_for_account(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT sender FROM bundles WHERE account = ?1")?;
    let rows = stmt.query_map([account], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?)
}

pub fn set_bundled(conn: &Connection, account: &str, sender: &str, bundled: bool) -> Result<()> {
    let sender = sender.trim().to_ascii_lowercase();
    if account.is_empty() || !sender.contains('@') {
        anyhow::bail!("A receiving account and sender mailbox are required");
    }
    if bundled {
        conn.execute(
            "INSERT OR IGNORE INTO bundles (account, sender) VALUES (?1, ?2)",
            params![account, sender],
        )?;
    } else {
        conn.execute(
            "DELETE FROM bundles WHERE account = ?1 AND sender = ?2",
            params![account, sender],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundling_is_per_account_and_reversible() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        set_bundled(&conn, "work", "Bot@Example.org", true).unwrap();
        assert!(bundled_for_account(&conn, "work")
            .unwrap()
            .contains("bot@example.org"));
        assert!(bundled_for_account(&conn, "personal").unwrap().is_empty());
        set_bundled(&conn, "work", "bot@example.org", false).unwrap();
        assert!(bundled_for_account(&conn, "work").unwrap().is_empty());
    }
}
