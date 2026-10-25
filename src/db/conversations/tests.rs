/*! Conversation mute and anchor tests. */

use super::*;
use crate::mail::types::Sender;

pub(super) fn envelope(id: &str, message_id: Option<&str>, in_reply_to: Option<&str>) -> Envelope {
    Envelope {
        id: id.into(),
        flags: Vec::new(),
        subject: "s".into(),
        sender: Sender::Plain("a@example.org".into()),
        date: "2026-09-23".into(),
        message_id: message_id.map(str::to_string),
        in_reply_to: in_reply_to.map(str::to_string),
        account: Some("work".into()),
        folder: Some("INBOX".into()),
    }
}

#[test]
fn muting_a_root_quiets_its_direct_reply() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let root = envelope("1", Some("root@x"), None);
    set_muted(&conn, "work", &anchors(&root), true).unwrap();
    let reply = envelope("2", Some("reply@x"), Some("root@x"));
    assert!(is_muted(&conn, "work", &anchors(&reply)).unwrap());
    assert!(!is_muted(&conn, "personal", &anchors(&reply)).unwrap());
}

#[test]
fn unmuting_forgets_a_rule_that_carries_no_other_decision() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let root = envelope("1", Some("root@x"), None);
    set_muted(&conn, "work", &anchors(&root), true).unwrap();
    set_muted(&conn, "work", &anchors(&root), false).unwrap();
    assert!(!is_muted(&conn, "work", &anchors(&root)).unwrap());
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversation_rules", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn a_uid_anchor_keeps_messages_without_a_message_id_distinct() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let first = envelope("7", None, None);
    let second = envelope("8", None, None);
    assert_ne!(anchors(&first), anchors(&second));
    set_muted(&conn, "work", &anchors(&first), true).unwrap();
    assert!(is_muted(&conn, "work", &anchors(&first)).unwrap());
    assert!(!is_muted(&conn, "work", &anchors(&second)).unwrap());
}
