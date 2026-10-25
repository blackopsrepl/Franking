use super::*;
use crate::mail::types::{Account, Envelope, Folder, FolderRole, Sender};
use crate::mail::MailError;
use crate::worker::{FolderChange, RoutingReport};

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
fn rule_is_stored_only_after_its_move_succeeds() {
    let mut app = app();
    app.enter_move_prompt();
    app.toggle_move_future_rule();
    let draft = app.move_rule_draft("Reading").unwrap();
    assert_eq!((draft.uid_validity, draft.first_uid), (7, 100));
    app.handle_rule_move_done(draft.clone(), Err(MailError::other("no such folder")));
    assert!(store::list(app.db.as_ref().unwrap()).unwrap().is_empty());
    assert!(app
        .status_message
        .starts_with("Rule not saved because the move failed"));
    app.handle_rule_move_done(draft.clone(), Ok("Moved to Reading.".into()));
    assert_eq!(store::list(app.db.as_ref().unwrap()).unwrap(), [draft]);
    app.accounts = vec![Account {
        name: "work".into(),
        backend: "maildir".into(),
        default: true,
    }];
    assert!(app.move_rule_draft("Reading").is_err());
}

#[test]
fn failing_rules_are_suspended_shown_and_resumed_only_with_their_folder() {
    let mut app = app();
    app.enter_move_prompt();
    let rule = app.move_rule_draft("Reading").unwrap();
    store::save(app.db.as_ref().unwrap(), &rule).unwrap();
    assert!(app.rule_snapshot(Some("work")).is_some());
    app.handle_routing_report(RoutingReport {
        failed: vec![(rule.clone(), "no such mailbox".into())],
        ..Default::default()
    });
    assert!(app.status_message.contains("alice@example.org → Reading"));
    assert!(app.status_message.contains("Actions → Sender folder rules"));
    assert!(
        app.rule_snapshot(Some("work")).is_none(),
        "suspended rules never apply"
    );
    app.open_folder_rules();
    assert_eq!(
        app.folder_rules.rules[0].suspended.as_deref(),
        Some("no such mailbox")
    );
    app.account_name = Some("work".into());
    let folders = std::mem::take(&mut app.folders);
    app.resume_folder_rule();
    assert!(app.folder_rules.rules[0].suspended.is_some());
    app.folders = folders;
    app.resume_folder_rule();
    assert_eq!(app.folder_rules.rules[0].suspended, None);
    app.handle_routing_report(RoutingReport {
        moved: vec![(rule, 2)],
        ..Default::default()
    });
    assert_eq!(app.status_message, "Moved 2 messages by sender rules.");
}

#[test]
fn folder_rename_and_delete_carry_rules_only_on_success() {
    let mut app = app();
    app.enter_move_prompt();
    let rule = app.move_rule_draft("Reading").unwrap();
    store::save(app.db.as_ref().unwrap(), &rule).unwrap();
    let account = Some("work".to_string());
    let renamed = FolderChange::Renamed {
        account: account.clone(),
        from: "Reading".into(),
        to: "Later".into(),
    };
    app.handle_folder_changed(renamed.clone(), Err(MailError::other("denied")));
    assert_eq!(
        store::list(app.db.as_ref().unwrap()).unwrap()[0].target,
        "Reading"
    );
    app.handle_folder_changed(renamed, Ok("Renamed Reading to Later.".into()));
    assert_eq!(
        store::list(app.db.as_ref().unwrap()).unwrap()[0].target,
        "Later"
    );
    assert!(app.status_message.ends_with("1 sender rule updated."));
    let deleted = FolderChange::Deleted {
        account,
        name: "Later".into(),
    };
    app.handle_folder_changed(deleted, Ok("Deleted folder Later.".into()));
    assert!(store::list(app.db.as_ref().unwrap()).unwrap().is_empty());
    assert!(app.status_message.ends_with("1 sender rule removed."));
}

#[test]
fn rule_can_be_inspected_and_removed_without_moving_existing_mail() {
    let mut app = app();
    app.enter_move_prompt();
    let rule = app.move_rule_draft("Reading").unwrap();
    store::save(app.db.as_ref().unwrap(), &rule).unwrap();
    app.open_folder_rules();
    assert_eq!(app.folder_rules.rules.len(), 1);
    app.delete_folder_rule();
    assert_eq!(app.folder_rules.rules.len(), 1);
    app.delete_folder_rule();
    assert!(app.folder_rules.rules.is_empty());
    assert_eq!(app.envelopes[0].id, "7");
}
