/*! Each published schema revision upgrades to the current layout without losing
account, message, or decision state. */

use rusqlite::Connection;

use super::migration_tests::versioned_v1;
use super::{init_for_test, schema_version, CURRENT_SCHEMA_VERSION};

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
    conn.execute(
        "INSERT INTO envelope_cache (account_id, folder_remote_id, remote_uid, subject, sender_display, received_at, flags)
         VALUES (1, 'INBOX', '42', 'Quarterly report', 'Alice', '2026-09-23 12:00:00', 'seen')",
        [],
    )
    .unwrap();
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
    conn.execute_batch(
        "INSERT INTO accounts (name, backend_kind) VALUES ('work', 'imap');
         INSERT INTO messages (account, folder, uid, subject, body_text) VALUES ('work', 'INBOX', '11', 'Preserved', 'important body');
         INSERT INTO outbox (account, template) VALUES ('work', 'draft content');",
    )
    .unwrap();
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

#[test]
fn v5_upgrade_adds_conversation_rules_and_keeps_followups() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute_batch(
        "UPDATE meta SET value = '5' WHERE key = 'schema_version';
         DROP TABLE conversation_rules;
         INSERT INTO message_markers (account, folder, uid, message_id, saved)
             VALUES ('work', 'INBOX', '9', 'nine@example.org', 1);",
    )
    .unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    let saved: i64 = conn
        .query_row(
            "SELECT saved FROM message_markers WHERE uid = '9'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(saved, 1);
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'conversation_rules')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(exists);
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
