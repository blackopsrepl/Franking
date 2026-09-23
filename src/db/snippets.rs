/*! Reusable text snippets for compose. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub name: String,
    pub body: String,
}

pub fn list(conn: &Connection) -> Result<Vec<Snippet>> {
    let mut stmt = conn.prepare("SELECT name, body FROM snippets ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |row| {
        Ok(Snippet {
            name: row.get(0)?,
            body: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, name: &str) -> Result<Option<Snippet>> {
    conn.query_row(
        "SELECT name, body FROM snippets WHERE name = ?1",
        [name],
        |row| {
            Ok(Snippet {
                name: row.get(0)?,
                body: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn save(conn: &Connection, snippet: &Snippet) -> Result<()> {
    let name = snippet.name.trim();
    if name.is_empty() || snippet.body.trim().is_empty() {
        anyhow::bail!("A snippet needs a name and a body");
    }
    conn.execute(
        "INSERT INTO snippets (name, body, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(name) DO UPDATE SET body = excluded.body, updated_at = datetime('now')",
        params![name, snippet.body],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, name: &str) -> Result<bool> {
    Ok(conn.execute("DELETE FROM snippets WHERE name = ?1", [name])? > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets_round_trip_by_name() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        save(
            &conn,
            &Snippet {
                name: "Greeting".into(),
                body: "Hi there,".into(),
            },
        )
        .unwrap();
        save(
            &conn,
            &Snippet {
                name: "Greeting".into(),
                body: "Hello,".into(),
            },
        )
        .unwrap();
        let items = list(&conn).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].body, "Hello,");
        assert!(delete(&conn, "Greeting").unwrap());
        assert!(list(&conn).unwrap().is_empty());
    }
}
