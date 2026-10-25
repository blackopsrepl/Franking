use super::*;
use crate::mail::types::{Envelope, Folder, FolderRole, Sender};

fn app() -> App {
    let mut app = App::new(Some("work".into()));
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    crate::mail::store::set_sync_state(
        &conn,
        &crate::mail::store::SyncState {
            account: "work".into(),
            folder: "INBOX".into(),
            uid_validity: Some(7),
            uid_next: Some(100),
            ..Default::default()
        },
    )
    .unwrap();
    app.db = Some(conn);
    app.current_folder = "INBOX".into();
    app.folders = vec![Folder {
        name: "Reading".into(),
        desc: None,
        role: FolderRole::Other,
        subscribed: None,
    }];
    app.envelopes = vec![Envelope {
        id: "7".into(),
        flags: vec![],
        subject: "Mail".into(),
        sender: Sender::Plain("alice@example.org".into()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: Some("work".into()),
        folder: Some("INBOX".into()),
    }];
    app.envelope_state.select(Some(0));
    app
}

#[test]
fn folder_rule_opt_in_resets_and_copy_never_offers_it() {
    let mut app = app();
    app.enter_move_prompt();
    assert!(!app.folder_rules.also_future);
    assert!(store::list(app.db.as_ref().unwrap()).unwrap().is_empty());
    app.toggle_move_future_rule();
    assert!(app.folder_rules.also_future);
    app.cancel_move();
    app.enter_move_prompt();
    assert!(!app.folder_rules.also_future);
    app.enter_copy_prompt();
    assert!(app.move_rule_sender().is_none());
    app.toggle_move_future_rule();
    assert!(!app.folder_rules.also_future);
}

#[test]
fn rule_can_be_inspected_and_removed_without_moving_existing_mail() {
    let mut app = app();
    app.enter_move_prompt();
    app.toggle_move_future_rule();
    app.save_move_future_rule("Reading").unwrap();
    app.open_folder_rules();
    assert_eq!(app.folder_rules.rules.len(), 1);
    app.delete_folder_rule();
    assert_eq!(app.folder_rules.rules.len(), 1);
    app.delete_folder_rule();
    assert!(app.folder_rules.rules.is_empty());
    assert_eq!(app.envelopes[0].id, "7");
}
