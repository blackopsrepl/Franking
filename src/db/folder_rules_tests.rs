use super::*;
use crate::mail::store::{set_sync_state, SyncState};

fn setup() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    for account in ["work", "home"] {
        set_inbox(&conn, account, 7, 100);
    }
    conn
}

fn set_inbox(conn: &Connection, account: &str, validity: u32, next: u32) {
    set_sync_state(
        conn,
        &SyncState {
            account: account.into(),
            folder: "INBOX".into(),
            uid_validity: Some(validity),
            uid_next: Some(next),
            highest_modseq: None,
            last_synced_at: None,
        },
    )
    .unwrap();
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
    let work = draft(&conn, "work", "Alice@Example.org", "Reading").unwrap();
    assert!(list(&conn).unwrap().is_empty(), "a draft is not stored");
    save(&conn, &work).unwrap();
    save(
        &conn,
        &draft(&conn, "home", "alice@example.org", "Receipts").unwrap(),
    )
    .unwrap();
    let rules = list(&conn).unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[1], work);
    assert_eq!(
        (work.sender.as_str(), work.uid_validity, work.first_uid),
        ("alice@example.org", 7, 100)
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
fn suspended_rules_resume_at_a_fresh_boundary() {
    let conn = setup();
    save(
        &conn,
        &draft(&conn, "work", "alice@example.org", "Reading").unwrap(),
    )
    .unwrap();
    assert!(suspend(&conn, "work", "alice@example.org", MAILBOX_RESET).unwrap());
    assert_eq!(
        list(&conn).unwrap()[0].suspended.as_deref(),
        Some(MAILBOX_RESET)
    );
    resume(&conn, "work", "alice@example.org", (8, 340)).unwrap();
    let rule = &list(&conn).unwrap()[0];
    assert_eq!((rule.uid_validity, rule.first_uid), (8, 340));
    assert_eq!(rule.suspended, None);
    assert!(resume(&conn, "work", "bob@example.org", (8, 340)).is_err());
}

#[test]
fn folder_rename_and_delete_follow_only_that_account() {
    let conn = setup();
    for (account, sender) in [
        ("work", "a@x.org"),
        ("work", "b@x.org"),
        ("home", "a@x.org"),
    ] {
        save(&conn, &draft(&conn, account, sender, "Reading").unwrap()).unwrap();
    }
    assert_eq!(
        retarget(&conn, "work", "Reading", "Later", None).unwrap(),
        2
    );
    let targets: Vec<_> = list(&conn).unwrap().into_iter().map(|r| r.target).collect();
    assert_eq!(targets, ["Reading", "Later", "Later"]);
    assert_eq!(delete_targeting(&conn, "work", "Later").unwrap(), 2);
    assert_eq!(list(&conn).unwrap().len(), 1);
}

#[test]
fn rejects_missing_inbox_cursor_and_invalid_targets() {
    let conn = setup();
    assert!(draft(&conn, "unknown", "alice@example.org", "Reading").is_err());
    for target in ["", "INBOX", "All Inboxes", "bad\nfolder"] {
        assert!(draft(&conn, "work", "alice@example.org", target).is_err());
    }
    assert!(list(&conn).unwrap().is_empty());
}
