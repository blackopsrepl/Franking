/*! Local conversation merges. A merge only changes how we group; the original
messages and outbound reply headers are untouched. */

use anyhow::Result;
use rusqlite::{params, Connection};

/// Merge source anchor to the root anchor it joins.
pub fn roots_for_account(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT anchor, root FROM thread_merges WHERE account = ?1")?;
    let rows = stmt.query_map([account], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<std::collections::HashMap<_, _>>>()?)
}

pub fn set(conn: &Connection, account: &str, anchor: &str, root: &str) -> Result<()> {
    if anchor.is_empty() || root.is_empty() || anchor == root {
        anyhow::bail!("A merge needs two distinct conversations");
    }
    conn.execute(
        "INSERT INTO thread_merges (account, anchor, root) VALUES (?1, ?2, ?3)
         ON CONFLICT(account, anchor) DO UPDATE SET root = excluded.root",
        params![account, anchor, root],
    )?;
    Ok(())
}

pub fn clear(conn: &Connection, account: &str, anchor: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM thread_merges WHERE account = ?1 AND anchor = ?2",
        params![account, anchor],
    )? > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_are_per_account_and_reversible() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        set(&conn, "work", "b@x", "a@x").unwrap();
        assert_eq!(
            roots_for_account(&conn, "work").unwrap().get("b@x"),
            Some(&"a@x".to_string())
        );
        assert!(roots_for_account(&conn, "personal").unwrap().is_empty());
        assert!(clear(&conn, "work", "b@x").unwrap());
        assert!(roots_for_account(&conn, "work").unwrap().is_empty());
    }
}
