/*! App unit tests: preferences overlay. */

#[test]
fn settings_cycle_between_preferences() {
    use crate::app::App;
    use crate::keys::View;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    app.open_settings();
    assert_eq!(app.settings_index, 0);

    assert_eq!(
        app.notification_rule,
        crate::app::notification_rules::NotificationRule::Focused
    );
    app.settings_toggle();
    assert_eq!(
        app.notification_rule,
        crate::app::notification_rules::NotificationRule::All
    );

    app.settings_move(1);
    assert_eq!(app.settings_index, 1);
    assert!(app.mark_read_on_open);
    app.settings_toggle();
    assert!(!app.mark_read_on_open);
    assert!(app.status_message.contains("stay unread"));

    app.settings_move(1);
    assert_eq!(app.settings_index, 2);
    assert_eq!(app.page_size, 50);
    app.settings_toggle();
    assert_eq!(app.page_size, 100);
    assert!(app.status_message.contains("Page size: 100"));
    assert_eq!(
        crate::db::preferences::get_number(app.db.as_ref().unwrap(), "page_size").unwrap(),
        Some(100)
    );

    app.settings_move(1);
    assert_eq!(app.settings_index, 3);
    assert!(!app.encrypt_drafts);
    app.settings_toggle();
    assert!(app.encrypt_drafts);
    assert!(app.status_message.contains("encrypted to you"));

    app.settings_move(1);
    assert_eq!(app.settings_index, 4);
    assert_eq!(app.autosave_seconds, 30);
    app.settings_toggle();
    assert_eq!(app.autosave_seconds, 60);
    assert!(app.status_message.contains("every 60s"));

    app.settings_move(1);
    assert_eq!(app.settings_index, 5);
    assert!(!app.cover_seen);
    app.settings_toggle();
    assert!(app.cover_seen);
    assert!(app.status_message.contains("press V"));

    app.settings_move(1);
    assert_eq!(app.settings_index, 6);
    app.settings_toggle();
    assert_eq!(app.view, View::BypassPrompt, "the token prompt opens");
    app.cancel_bypass();

    app.settings_move(1);
    assert_eq!(app.settings_index, 0, "wraps around");
    app.settings_move(-1);
    assert_eq!(app.settings_index, 6);

    // The stored values come back on the next start.
    let mut reloaded = App::new(None);
    reloaded.db = app.db.take();
    reloaded.load_preferences();
    assert_eq!(reloaded.page_size, 100);
    assert_eq!(reloaded.autosave_seconds, 60);
    assert!(reloaded.encrypt_drafts, "the choice comes back");
    assert!(reloaded.cover_seen, "the cover choice comes back");
}

#[test]
fn opening_a_message_marks_it_seen_and_queues_the_flag() {
    use crate::app::App;
    use crate::keys::View;
    use crate::mail::types::{Envelope, Sender};

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let raw = b"From: alice@example.com\r\nSubject: Unread\r\n\r\nbody\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.db = Some(conn);
    app.view = View::MessageView;
    app.envelopes = vec![Envelope {
        id: "5".to_string(),
        flags: Vec::new(),
        subject: "Unread".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));

    app.handle_message_loaded(document);
    assert!(
        app.envelopes[0].is_seen(),
        "the local envelope reflects the read state"
    );

    // With the preference off, the load path leaves the flag alone.
    let mut app = App::new(None);
    app.mark_read_on_open = false;
    app.view = View::MessageView;
    app.envelopes = vec![Envelope {
        id: "6".to_string(),
        flags: Vec::new(),
        subject: "Unread".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));
    app.handle_message_loaded(crate::mail::mime::parse_message(raw).unwrap());
    assert!(app.envelopes[0].is_seen(), "local state still updates");
}
