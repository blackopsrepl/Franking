/*! Upgrade robustness: unsupported, malformed, and failing databases are left
intact rather than reset. Historical upgrades live in `migration_history`. */

use rusqlite::Connection;

use super::init_for_test;

/// A minimal database in the very first published layout.
pub(super) fn versioned_v1() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO meta VALUES ('schema_version', '1');
         CREATE TABLE contacts (
             id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, email TEXT NOT NULL,
             phone TEXT, org TEXT, notes TEXT, harvested INTEGER NOT NULL DEFAULT 0,
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE UNIQUE INDEX idx_contacts_email ON contacts(email);
         CREATE INDEX idx_contacts_name ON contacts(name);
         CREATE TABLE contact_tags (
             contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
             tag TEXT NOT NULL, PRIMARY KEY (contact_id, tag)
         );
         CREATE TABLE identities (
             id INTEGER PRIMARY KEY AUTOINCREMENT, account TEXT NOT NULL,
             name TEXT, display_name TEXT, email TEXT NOT NULL,
             is_default INTEGER NOT NULL DEFAULT 0,
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             UNIQUE(account, email)
         );
         CREATE INDEX idx_identities_account ON identities(account);
         CREATE TABLE accounts (
             id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE,
             backend_kind TEXT NOT NULL, provider_kind TEXT NOT NULL DEFAULT 'custom',
             enabled INTEGER NOT NULL DEFAULT 1, is_default INTEGER NOT NULL DEFAULT 0,
             maildir_path TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX idx_accounts_enabled ON accounts(enabled);
         CREATE INDEX idx_accounts_default ON accounts(is_default);
         INSERT INTO accounts (name, backend_kind) VALUES ('work', 'imap');
         INSERT INTO contacts (name, email) VALUES ('Alice', 'alice@example.org');
         INSERT INTO contact_tags (contact_id, tag) VALUES (1, 'important');
         INSERT INTO identities (account, email) VALUES ('work', 'me@example.org');",
    )
    .unwrap();
    conn
}

#[test]
fn failed_upgrade_rolls_back_ddl_and_version() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute(
        "UPDATE meta SET value = '3' WHERE key = 'schema_version'",
        [],
    )
    .unwrap();
    conn.execute_batch("DROP TABLE sender_routes; DROP TABLE account_endpoints;")
        .unwrap();
    assert!(init_for_test(&conn).is_err());
    assert_eq!(super::schema_version(&conn), 3);
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'sender_routes')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!exists);
}

#[test]
fn unknown_unversioned_database_is_not_rebuilt() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE personal_notes (body TEXT); INSERT INTO personal_notes VALUES ('keep me');",
    )
    .unwrap();
    assert!(init_for_test(&conn).is_err());
    let body: String = conn
        .query_row("SELECT body FROM personal_notes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(body, "keep me");
}

#[test]
fn malformed_schema_version_fails_without_changes() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO meta VALUES ('schema_version', 'oops');
        CREATE TABLE personal_notes (body TEXT);
        INSERT INTO personal_notes VALUES ('keep me');",
    )
    .unwrap();
    assert!(init_for_test(&conn)
        .unwrap_err()
        .to_string()
        .contains("invalid"));
    let body: String = conn
        .query_row("SELECT body FROM personal_notes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(body, "keep me");
}
