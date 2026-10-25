/*! Each published schema revision upgrades to the current layout without losing
account, message, or decision state. */

use rusqlite::Connection;

use super::migration_tests::versioned_v1;
use super::{init_for_test, schema_version, CURRENT_SCHEMA_VERSION};

/// A stored sender decision, read without the retired routing API.
fn sender_route(conn: &Connection, account: &str, sender: &str) -> Option<String> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT route FROM sender_routes WHERE account = ?1 AND sender = ?2",
        [account, sender],
        |row| row.get(0),
    )
    .optional()
    .unwrap()
}

fn store_sender_route(conn: &Connection, account: &str, sender: &str, route: &str) {
    conn.execute(
        "INSERT INTO sender_routes (account, sender, route) VALUES (?1, ?2, ?3)",
        [account, sender, route],
    )
    .unwrap();
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
    assert_eq!(sender_route(&conn, "work", "alice@example.org"), None);
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
    store_sender_route(&conn, "work", "alice@example.org", "reading");
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
        sender_route(&conn, "work", "alice@example.org").as_deref(),
        Some("reading")
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
    assert_eq!(sender_route(&conn, "test", "alice@example.org"), None);
}

#[test]
fn v4_upgrade_adds_followup_without_losing_sender_decisions() {
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    conn.execute_batch(
        "UPDATE meta SET value = '4' WHERE key = 'schema_version'; DROP TABLE message_markers;",
    )
    .unwrap();
    store_sender_route(&conn, "work", "alice@example.org", "reading");
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert_eq!(
        sender_route(&conn, "work", "alice@example.org").as_deref(),
        Some("reading")
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
fn v16_upgrade_keeps_runtime_folder_rules_and_matches_fresh_layout() {
    let columns = |conn: &Connection| -> Vec<(String, String, bool)> {
        let mut stmt = conn
            .prepare("SELECT name, type, \"notnull\" FROM pragma_table_info('folder_rules')")
            .unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)));
        rows.unwrap().map(Result::unwrap).collect()
    };
    let fresh = Connection::open_in_memory().unwrap();
    init_for_test(&fresh).unwrap();
    let conn = Connection::open_in_memory().unwrap();
    init_for_test(&conn).unwrap();
    // The first rules build created this table at runtime under schema 16.
    conn.execute_batch(
        "UPDATE meta SET value = '16' WHERE key = 'schema_version';
         DROP TABLE folder_rules;
         CREATE TABLE folder_rules (account TEXT NOT NULL, sender TEXT NOT NULL,
             target TEXT NOT NULL, uid_validity INTEGER NOT NULL CHECK(uid_validity > 0),
             first_uid INTEGER NOT NULL CHECK(first_uid > 0), PRIMARY KEY(account, sender));
         INSERT INTO folder_rules VALUES ('work', 'alice@example.org', 'Reading', 7, 100);",
    )
    .unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert_eq!(columns(&conn), columns(&fresh));
    let rules = super::folder_rules::list(&conn).unwrap();
    assert_eq!(
        (rules[0].target.as_str(), rules[0].suspended.as_deref()),
        ("Reading", None)
    );
    conn.execute_batch(
        "UPDATE meta SET value = '16' WHERE key = 'schema_version'; DROP TABLE folder_rules;",
    )
    .unwrap();
    init_for_test(&conn).unwrap();
    assert_eq!(columns(&conn), columns(&fresh));
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
