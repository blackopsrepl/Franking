/*! Account-scoped workflow stages and the conversation assigned to each. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

pub fn list(conn: &Connection, account: &str) -> Result<Vec<Stage>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position FROM stages WHERE account = ?1 ORDER BY position, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([account], |row| {
        Ok(Stage {
            id: row.get(0)?,
            name: row.get(1)?,
            position: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Create a stage, or rename the one that already carries this name.
pub fn upsert(conn: &Connection, account: &str, name: &str) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("A stage needs a name");
    }
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM stages WHERE account = ?1 AND name = ?2",
            params![account, name],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    let next: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM stages WHERE account = ?1",
            [account],
            |row| row.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO stages (account, name, position) VALUES (?1, ?2, ?3)",
        params![account, name, next],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("A stage needs a name");
    }
    conn.execute(
        "UPDATE stages SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    let removed = conn.execute("DELETE FROM stages WHERE id = ?1", [id])? > 0;
    conn.execute("DELETE FROM conversation_stages WHERE stage_id = ?1", [id])?;
    Ok(removed)
}

/// Move a conversation into a stage, replacing any earlier stage.
pub fn assign(conn: &Connection, account: &str, anchors: &[String], stage_id: i64) -> Result<()> {
    if anchors.is_empty() {
        anyhow::bail!("A conversation identity is required");
    }
    for anchor in anchors {
        conn.execute(
            "DELETE FROM conversation_stages WHERE account = ?1 AND anchor = ?2",
            params![account, anchor],
        )?;
        conn.execute(
            "INSERT INTO conversation_stages (account, anchor, stage_id, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![account, anchor, stage_id],
        )?;
    }
    Ok(())
}

pub fn clear(conn: &Connection, account: &str, anchors: &[String]) -> Result<()> {
    for anchor in anchors {
        conn.execute(
            "DELETE FROM conversation_stages WHERE account = ?1 AND anchor = ?2",
            params![account, anchor],
        )?;
    }
    Ok(())
}

/// Stage name for every assigned conversation anchor of an account.
pub fn assignments_for_account(
    conn: &Connection,
    account: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare(
        "SELECT cs.anchor, s.name
         FROM conversation_stages cs JOIN stages s ON s.id = cs.stage_id
         WHERE cs.account = ?1",
    )?;
    let rows = stmt.query_map([account], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<std::collections::HashMap<_, _>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conversation_moves_between_stages() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        let todo = upsert(&conn, "work", "To do").unwrap();
        let done = upsert(&conn, "work", "Done").unwrap();
        assert_eq!(list(&conn, "work").unwrap().len(), 2);
        let anchors = vec!["root@x".to_string()];
        assign(&conn, "work", &anchors, todo).unwrap();
        assert_eq!(
            assignments_for_account(&conn, "work")
                .unwrap()
                .get("root@x"),
            Some(&"To do".to_string())
        );
        assign(&conn, "work", &anchors, done).unwrap();
        let map = assignments_for_account(&conn, "work").unwrap();
        assert_eq!(map.get("root@x"), Some(&"Done".to_string()));
        clear(&conn, "work", &anchors).unwrap();
        assert!(assignments_for_account(&conn, "work").unwrap().is_empty());
        assert!(delete(&conn, done).unwrap());
    }
}
