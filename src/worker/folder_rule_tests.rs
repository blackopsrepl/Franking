use super::*;
use crate::mail::store::{set_sync_state, SyncState};
use crate::mail::types::Sender;

#[test]
fn routing_moves_only_matching_future_mail_and_surfaces_server_failure() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    set_sync_state(
        &conn,
        &SyncState {
            account: "work".into(),
            folder: "INBOX".into(),
            uid_validity: Some(7),
            uid_next: Some(100),
            ..Default::default()
        },
    )
    .unwrap();
    folder_rules::save(&conn, "work", "alice@example.org", "Reading").unwrap();
    let make = |id: &str, account: &str| Envelope {
        id: id.into(),
        account: Some(account.into()),
        folder: Some("INBOX".into()),
        sender: Sender::Plain("alice@example.org".into()),
        flags: vec![],
        subject: "Mail".into(),
        date: "1990-01-01".into(),
        message_id: None,
        in_reply_to: None,
    };
    let mut moved = Vec::new();
    let visible = apply_with(
        &conn,
        vec![make("99", "work"), make("100", "home"), make("100", "work")],
        |a, f, t, id| {
            moved.push((a.to_string(), f.to_string(), t.to_string(), id.to_string()));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(visible.len(), 2);
    assert_eq!(
        moved,
        vec![(
            "work".into(),
            "INBOX".into(),
            "Reading".into(),
            "100".into()
        )]
    );
    let failure = apply_with(&conn, vec![make("101", "work")], |_, _, _, _| {
        Err(MailError::other("server refused move"))
    })
    .unwrap_err();
    assert!(failure.detail.contains("server refused move"));
}
