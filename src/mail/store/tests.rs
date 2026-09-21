use rusqlite::Connection;

use super::{
    count_messages, delete_message, get_message, get_sync_state, list_messages, move_message,
    search_messages, set_flag, set_sync_state, thread_messages, upsert_envelope, upsert_message,
    StoredMessage, SyncState,
};
use crate::mail::types::{Envelope, Sender};

fn store() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    conn
}

fn message(uid: &str, subject: &str, body: &str) -> StoredMessage {
    StoredMessage {
        account: "work".to_string(),
        folder: "INBOX".to_string(),
        uid: uid.to_string(),
        subject: subject.to_string(),
        from_display: "Alice <alice@example.com>".to_string(),
        to_display: "bob@example.com".to_string(),
        date_epoch: Some(1_700_000_000),
        body_text: body.to_string(),
        snippet: body.to_string(),
        ..StoredMessage::default()
    }
}

#[test]
fn upsert_and_get_round_trip() {
    let conn = store();
    let mut row = message("42", "Project update", "the body");
    row.message_id = Some("m42@example.com".to_string());
    row.thread_root = Some("root@example.com".to_string());
    row.flags = vec!["Seen".to_string()];

    upsert_message(&conn, &row).unwrap();

    let loaded = get_message(&conn, "work", "INBOX", "42").unwrap().unwrap();
    assert_eq!(loaded.subject, "Project update");
    assert_eq!(loaded.message_id.as_deref(), Some("m42@example.com"));
    assert_eq!(loaded.flags, vec!["Seen".to_string()]);

    upsert_message(&conn, &row).unwrap();
    assert_eq!(count_messages(&conn, "work", "INBOX").unwrap(), 1);
}

#[test]
fn search_indexes_subject_and_body() {
    let conn = store();
    upsert_message(&conn, &message("1", "Quarterly report", "revenue grew")).unwrap();
    upsert_message(&conn, &message("2", "Lunch", "sandwich")).unwrap();

    let by_subject = search_messages(&conn, Some("work"), "quarterly", 10).unwrap();
    assert_eq!(by_subject.len(), 1);
    assert_eq!(by_subject[0].uid, "1");

    let by_body = search_messages(&conn, None, "sandwich", 10).unwrap();
    assert_eq!(by_body.len(), 1);
    assert_eq!(by_body[0].uid, "2");

    assert!(search_messages(&conn, None, "missing", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn listing_and_threading() {
    let conn = store();
    let mut child = message("2", "Re: Project", "reply");
    child.thread_root = Some("root@example.com".to_string());
    child.message_id = Some("child@example.com".to_string());
    let mut root = message("1", "Project", "start");
    root.thread_root = Some("root@example.com".to_string());
    root.message_id = Some("root@example.com".to_string());
    root.date_epoch = Some(1_600_000_000);

    upsert_message(&conn, &child).unwrap();
    upsert_message(&conn, &root).unwrap();

    let listed = list_messages(&conn, "work", "INBOX", 10, 0).unwrap();
    assert_eq!(listed[0].uid, "2", "newest first");

    let thread = thread_messages(&conn, "work", "root@example.com").unwrap();
    assert_eq!(thread.len(), 2);

    delete_message(&conn, "work", "INBOX", "2").unwrap();
    assert_eq!(count_messages(&conn, "work", "INBOX").unwrap(), 1);
    assert!(search_messages(&conn, None, "reply", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn sync_state_round_trip() {
    let conn = store();
    assert!(get_sync_state(&conn, "work", "INBOX").unwrap().is_none());

    set_sync_state(
        &conn,
        &SyncState {
            account: "work".to_string(),
            folder: "INBOX".to_string(),
            uid_validity: Some(7),
            uid_next: Some(100),
            highest_modseq: Some(5000),
            last_synced_at: None,
        },
    )
    .unwrap();

    let loaded = get_sync_state(&conn, "work", "INBOX").unwrap().unwrap();
    assert_eq!(loaded.uid_validity, Some(7));
    assert_eq!(loaded.uid_next, Some(100));
    assert_eq!(loaded.highest_modseq, Some(5000));
    assert!(loaded.last_synced_at.is_some());
}

#[test]
fn envelope_metadata_update_keeps_cached_body_and_raw() {
    let conn = store();
    let mut full = message("7", "original", "full body text");
    full.raw = Some(b"raw bytes".to_vec());
    full.message_id = Some("m7@example.com".to_string());
    upsert_message(&conn, &full).unwrap();

    let envelope = Envelope {
        id: "7".to_string(),
        flags: vec!["Seen".to_string()],
        subject: "updated subject".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: "2026-04-13 09:00:00+00:00".to_string(),
        message_id: None,
        in_reply_to: None,
    };
    upsert_envelope(
        &conn,
        &StoredMessage::from_envelope("work", "INBOX", &envelope),
    )
    .unwrap();

    let loaded = get_message(&conn, "work", "INBOX", "7").unwrap().unwrap();
    assert_eq!(loaded.subject, "updated subject");
    assert_eq!(loaded.flags, vec!["Seen".to_string()]);
    assert_eq!(loaded.message_id.as_deref(), Some("m7@example.com"));
    assert_eq!(loaded.raw.as_deref(), Some(b"raw bytes".as_slice()));
    assert!(loaded.body_text.contains("full body text"));
}

#[test]
fn envelope_round_trips_through_the_store_shape() {
    let envelope = Envelope {
        id: "9".to_string(),
        flags: vec!["Flagged".to_string()],
        subject: "hi".to_string(),
        sender: Sender::Plain("Bob <bob@example.com>".to_string()),
        date: "2026-04-13 09:00:00+00:00".to_string(),
        message_id: None,
        in_reply_to: None,
    };

    let stored = StoredMessage::from_envelope("work", "INBOX", &envelope);
    assert_eq!(stored.from_email.as_deref(), Some("bob@example.com"));
    assert_eq!(stored.to_envelope(), envelope);
}

#[test]
fn flag_and_move_updates_are_cached() {
    let conn = store();
    upsert_message(&conn, &message("1", "hi", "body")).unwrap();

    set_flag(&conn, "work", "INBOX", "1", "Seen", true).unwrap();
    set_flag(&conn, "work", "INBOX", "1", "Flagged", true).unwrap();
    let loaded = get_message(&conn, "work", "INBOX", "1").unwrap().unwrap();
    assert!(loaded.flags.iter().any(|flag| flag == "Seen"));
    assert!(loaded.flags.iter().any(|flag| flag == "Flagged"));

    set_flag(&conn, "work", "INBOX", "1", "Seen", false).unwrap();
    let loaded = get_message(&conn, "work", "INBOX", "1").unwrap().unwrap();
    assert!(!loaded.flags.iter().any(|flag| flag == "Seen"));

    move_message(&conn, "work", "INBOX", "1", "Trash").unwrap();
    assert!(get_message(&conn, "work", "INBOX", "1").unwrap().is_none());
    assert!(get_message(&conn, "work", "Trash", "1").unwrap().is_some());
}
