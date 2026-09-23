/*! Named collections of conversations. A conversation can belong to many. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub id: i64,
    pub name: String,
}

pub fn list(conn: &Connection, account: &str) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT id, name FROM collections WHERE account = ?1 ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([account], |row| {
        Ok(Collection {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn upsert(conn: &Connection, account: &str, name: &str) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("A collection needs a name");
    }
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM collections WHERE account = ?1 AND name = ?2",
            params![account, name],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO collections (account, name) VALUES (?1, ?2)",
        params![account, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("A collection needs a name");
    }
    conn.execute(
        "UPDATE collections SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    let removed = conn.execute("DELETE FROM collections WHERE id = ?1", [id])? > 0;
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1",
        [id],
    )?;
    Ok(removed)
}

/// Add or remove each anchor from a collection. Returns true when added.
pub fn toggle(conn: &Connection, account: &str, id: i64, anchors: &[String]) -> Result<bool> {
    if anchors.is_empty() {
        anyhow::bail!("A conversation identity is required");
    }
    let anchor = &anchors[0];
    let present: bool = conn
        .query_row(
            "SELECT 1 FROM collection_items WHERE account = ?1 AND collection_id = ?2 AND anchor = ?3",
            params![account, id, anchor],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if present {
        for anchor in anchors {
            conn.execute(
                "DELETE FROM collection_items WHERE account = ?1 AND collection_id = ?2 AND anchor = ?3",
                params![account, id, anchor],
            )?;
        }
        Ok(false)
    } else {
        for anchor in anchors {
            conn.execute(
                "INSERT OR IGNORE INTO collection_items (account, collection_id, anchor)
                 VALUES (?1, ?2, ?3)",
                params![account, id, anchor],
            )?;
        }
        Ok(true)
    }
}

/// Collection names for every anchor of an account.
pub fn membership_for_account(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare(
        "SELECT ci.anchor, c.name
         FROM collection_items ci JOIN collections c ON c.id = ci.collection_id
         WHERE ci.account = ?1",
    )?;
    let rows = stmt.query_map([account], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for row in rows {
        let (anchor, name) = row?;
        map.entry(anchor).or_default().push(name);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_toggles_and_is_per_collection() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let projects = upsert(&conn, "work", "Projects").unwrap();
        let trips = upsert(&conn, "work", "Trips").unwrap();
        let anchors = vec!["root@x".to_string()];
        assert!(toggle(&conn, "work", projects, &anchors).unwrap());
        assert!(toggle(&conn, "work", trips, &anchors).unwrap());
        let map = membership_for_account(&conn, "work").unwrap();
        assert_eq!(map.get("root@x").map(Vec::len), Some(2));
        assert!(
            !toggle(&conn, "work", projects, &anchors).unwrap(),
            "removed"
        );
        assert_eq!(
            membership_for_account(&conn, "work")
                .unwrap()
                .get("root@x")
                .map(Vec::len),
            Some(1)
        );
    }
}
