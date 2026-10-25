use super::*;
use crate::mail::store::{set_sync_state, SyncState};
use crate::mail::types::{Envelope, Sender};

fn setup() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    for account in ["work", "home"] {
        set_sync_state(
            &conn,
            &SyncState {
                account: account.into(),
                folder: "INBOX".into(),
                uid_validity: Some(7),
                uid_next: Some(100),
                highest_modseq: None,
                last_synced_at: None,
            },
        )
        .unwrap();
    }
    conn
}

fn envelope(account: &str, uid: &str, folder: &str) -> Envelope {
    Envelope {
        id: uid.into(),
        flags: vec![],
        subject: "Mail".into(),
        sender: Sender::Plain("Alice <ALICE@example.org>".into()),
        date: "1990-01-01".into(),
        message_id: None,
        in_reply_to: None,
        account: Some(account.into()),
        folder: Some(folder.into()),
    }
}

#[test]
fn explicit_rules_are_account_scoped_persistent_and_removable() {
    let conn = setup();
    conn.execute(
        "INSERT INTO sender_routes (account, sender, route)
         VALUES ('work', 'alice@example.org', 'blocked')",
        [],
    )
    .unwrap();
    assert!(list(&conn).unwrap().is_empty());
    save(&conn, "work", "Alice@Example.org", "Reading").unwrap();
    save(&conn, "home", "alice@example.org", "Receipts").unwrap();
    let rules = list(&conn).unwrap();
    assert_eq!(
        target_for(&conn, &rules, &envelope("work", "100", "INBOX")).unwrap(),
        Some("Reading")
    );
    assert_eq!(
        target_for(&conn, &rules, &envelope("home", "101", "INBOX")).unwrap(),
        Some("Receipts")
    );
    assert!(delete(&conn, "work", "alice@example.org").unwrap());
    assert_eq!(list(&conn).unwrap().len(), 1);
    let legacy: i64 = conn
        .query_row("SELECT COUNT(*) FROM sender_routes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        legacy, 1,
        "old screening decisions are never imported or deleted"
    );
}

#[test]
fn only_future_inbox_mail_in_same_uid_epoch_qualifies() {
    let conn = setup();
    save(&conn, "work", "alice@example.org", "Reading").unwrap();
    let rules = list(&conn).unwrap();
    for e in [
        envelope("work", "99", "INBOX"),
        envelope("work", "100", "Sent"),
        envelope("other", "100", "INBOX"),
        envelope("work", "nonnumeric", "INBOX"),
    ] {
        assert_eq!(target_for(&conn, &rules, &e).unwrap(), None);
    }
    let mut e = envelope("work", "100", "INBOX");
    e.account = None;
    assert_eq!(target_for(&conn, &rules, &e).unwrap(), None);
    conn.execute(
        "UPDATE sync_state SET uid_validity=8 WHERE account='work'",
        [],
    )
    .unwrap();
    assert_eq!(
        target_for(&conn, &rules, &envelope("work", "100", "INBOX")).unwrap(),
        None
    );
}

#[test]
fn rejects_missing_inbox_cursor_and_invalid_targets() {
    let conn = setup();
    assert!(save(&conn, "unknown", "alice@example.org", "Reading").is_err());
    for target in ["", "INBOX", "All Inboxes", "bad\nfolder"] {
        assert!(save(&conn, "work", "alice@example.org", target).is_err());
    }
    assert!(list(&conn).unwrap().is_empty());
}
