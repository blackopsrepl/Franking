/*! Named searches the user can re-run. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::mail::search_scope::SearchScope;

/// A stored search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSearch {
    pub name: String,
    pub query: String,
    /// How far the search reached when it was saved.
    pub scope: SearchScope,
}

/// Every saved search, by name.
pub fn list(conn: &Connection) -> Result<Vec<SavedSearch>> {
    let mut statement =
        conn.prepare("SELECT name, query, scope FROM saved_searches ORDER BY name")?;
    let rows = statement.query_map([], |row| {
        let scope: String = row.get(2)?;
        Ok(SavedSearch {
            name: row.get(0)?,
            query: row.get(1)?,
            scope: SearchScope::parse(&scope),
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("cannot list saved searches")
}

/// Store a search under `name`, replacing any search of that name.
pub fn save(conn: &Connection, search: &SavedSearch) -> Result<()> {
    conn.execute(
        "INSERT INTO saved_searches (name, query, scope)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET
             query = excluded.query,
             scope = excluded.scope",
        params![search.name, search.query, search.scope.as_str()],
    )
    .context("cannot save the search")?;
    Ok(())
}

/// Remove a saved search, reporting whether it was there.
pub fn delete(conn: &Connection, name: &str) -> Result<bool> {
    let removed = conn
        .execute("DELETE FROM saved_searches WHERE name = ?1", [name])
        .context("cannot delete the saved search")?;
    Ok(removed > 0)
}

/// One saved search by name.
pub fn get(conn: &Connection, name: &str) -> Result<Option<SavedSearch>> {
    conn.query_row(
        "SELECT name, query, scope FROM saved_searches WHERE name = ?1",
        [name],
        |row| {
            let scope: String = row.get(2)?;
            Ok(SavedSearch {
                name: row.get(0)?,
                query: row.get(1)?,
                scope: SearchScope::parse(&scope),
            })
        },
    )
    .optional()
    .context("cannot load the saved search")
}

#[cfg(test)]
mod tests {
    use crate::mail::search_scope::SearchScope;

    use super::{delete, get, list, save, SavedSearch};

    fn conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        conn
    }

    #[test]
    fn searches_round_trip_by_name() {
        let conn = conn();
        assert!(list(&conn).unwrap().is_empty());

        save(
            &conn,
            &SavedSearch {
                name: "Quarterly".to_string(),
                query: "subject quarterly and not flag seen".to_string(),
                scope: SearchScope::Folders,
            },
        )
        .unwrap();
        save(
            &conn,
            &SavedSearch {
                name: "From Alice".to_string(),
                query: "from alice".to_string(),
                scope: SearchScope::Folder,
            },
        )
        .unwrap();

        let names: Vec<String> = list(&conn).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["From Alice", "Quarterly"], "sorted by name");
        assert_eq!(
            get(&conn, "Quarterly").unwrap().unwrap().scope,
            SearchScope::Folders
        );

        // Saving the same name replaces the query.
        save(
            &conn,
            &SavedSearch {
                name: "Quarterly".to_string(),
                query: "subject revenue".to_string(),
                scope: SearchScope::Accounts,
            },
        )
        .unwrap();
        let updated = get(&conn, "Quarterly").unwrap().unwrap();
        assert_eq!(updated.query, "subject revenue");
        assert_eq!(updated.scope, SearchScope::Accounts);
        assert_eq!(list(&conn).unwrap().len(), 2);

        assert!(delete(&conn, "Quarterly").unwrap());
        assert!(!delete(&conn, "Quarterly").unwrap());
        assert_eq!(list(&conn).unwrap().len(), 1);
    }
}
