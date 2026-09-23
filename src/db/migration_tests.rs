/*! Upgrade real historical layouts without sacrificing account or mail state. */

use rusqlite::Connection;

use super::{init_for_test, schema_version, CURRENT_SCHEMA_VERSION};

fn versioned_v1() -> Connection {
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
fn v1_to_current_preserves_accounts_contacts_tags_and_identities() {
    let conn = versioned_v1();
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    let email: String = conn
        .query_row(
            "SELECT email FROM identities WHERE account = 'work'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(email, "me@example.org");
    let tag: String = conn
        .query_row(
            "SELECT tag FROM contact_tags WHERE contact_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tag, "important");
    assert!(super::connection::schema_version(&conn) == CURRENT_SCHEMA_VERSION);
    assert_eq!(
        super::sender_routes::get(&conn, "work", "alice@example.org").unwrap(),
        super::sender_routes::Route::Screening
    );
}

#[test]
fn v2_cache_becomes_searchable_without_discarding_originals() {
    let conn = versioned_v1();
    super::migration_v1_v2::apply(&conn).unwrap();
    conn.execute(
        "UPDATE meta SET value = '2' WHERE key = 'schema_version'",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO envelope_cache (account_id, folder_remote_id, remote_uid, subject, sender_display, received_at, flags)
                  VALUES (1, 'INBOX', '42', 'Quarterly report', 'Alice', '2026-09-23 12:00:00', 'seen')", []).unwrap();
    init_for_test(&conn).unwrap();
    let (subject, sender): (String, String) = conn
        .query_row(
            "SELECT subject, from_display FROM messages WHERE account = 'work' AND uid = '42'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (subject.as_str(), sender.as_str()),
        ("Quarterly report", "Alice")
    );
    let matches: i64 = conn
        .query_row(
            "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'Quarterly'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(matches, 1);
    let legacy: i64 = conn
        .query_row("SELECT count(*) FROM envelope_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(legacy, 1);
}

#[test]
fn v3_upgrade_keeps_existing_rows_and_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute(
        "UPDATE meta SET value = '3' WHERE key = 'schema_version'",
        [],
    )
    .unwrap();
    super::sender_routes::set(
        &conn,
        "work",
        "alice@example.org",
        super::sender_routes::Route::Reading,
    )
    .unwrap();
    conn.execute_batch("INSERT INTO accounts (name, backend_kind) VALUES ('work', 'imap');
        INSERT INTO messages (account, folder, uid, subject, body_text) VALUES ('work', 'INBOX', '11', 'Preserved', 'important body');
        INSERT INTO outbox (account, template) VALUES ('work', 'draft content');").unwrap();
    init_for_test(&conn).unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    let body: String = conn
        .query_row(
            "SELECT body_text FROM messages WHERE uid = '11'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(body, "important body");
    let draft: String = conn
        .query_row(
            "SELECT template FROM outbox WHERE account = 'work'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(draft, "draft content");
    assert_eq!(
        super::sender_routes::get(&conn, "work", "alice@example.org").unwrap(),
        super::sender_routes::Route::Reading
    );
}

#[test]
fn v3_without_sender_routes_gains_the_new_table() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute_batch(
        "UPDATE meta SET value = '3' WHERE key = 'schema_version'; DROP TABLE sender_routes;",
    )
    .unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(
        super::sender_routes::get(&conn, "test", "alice@example.org").unwrap(),
        super::sender_routes::Route::Screening
    );
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
    assert_eq!(schema_version(&conn), 3);
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

#[test]
fn file_backed_upgrade_survives_process_restart() {
    let path = std::env::temp_dir().join(format!(
        "franking-upgrade-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let conn = Connection::open(&path).unwrap();
        init_for_test(&conn).unwrap();
        conn.execute_batch(
            "UPDATE meta SET value = '3' WHERE key = 'schema_version';
            DROP TABLE sender_routes;
            INSERT INTO contacts (email, notes) VALUES ('alice@example.org', 'do not lose');",
        )
        .unwrap();
    }
    {
        let conn = Connection::open(&path).unwrap();
        init_for_test(&conn).unwrap();
        assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
        let note: String = conn
            .query_row(
                "SELECT notes FROM contacts WHERE email = 'alice@example.org'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(note, "do not lose");
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn v4_upgrade_adds_followup_without_losing_sender_decisions() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute_batch(
        "UPDATE meta SET value = '4' WHERE key = 'schema_version'; DROP TABLE message_markers;",
    )
    .unwrap();
    super::sender_routes::set(
        &conn,
        "work",
        "alice@example.org",
        super::sender_routes::Route::Reading,
    )
    .unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert_eq!(
        super::sender_routes::get(&conn, "work", "alice@example.org").unwrap(),
        super::sender_routes::Route::Reading
    );
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'message_markers')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(exists);
}
