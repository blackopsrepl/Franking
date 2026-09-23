/*! Account identity and sender routing in the message-list workflow. */

use crate::app::App;
use crate::db::sender_routes::{self, Route};
use crate::mail::types::{Envelope, Sender};

fn envelope(account: &str, id: &str, sender: &str) -> Envelope {
    Envelope {
        id: id.into(),
        flags: Vec::new(),
        subject: id.into(),
        sender: Sender::Structured {
            name: None,
            addr: Some(sender.into()),
        },
        date: "2026-09-23".into(),
        message_id: None,
        in_reply_to: None,
        account: Some(account.into()),
        folder: Some("INBOX".into()),
    }
}

#[test]
fn a_combined_lane_respects_each_receiving_account() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    sender_routes::set(&conn, "work", "same@example.org", Route::Reading).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    app.current_folder = "All Inboxes".into();
    app.triage_lane = Some(Route::Screening);
    app.handle_envelopes_loaded(vec![
        envelope("work", "1", "same@example.org"),
        envelope("personal", "2", "same@example.org"),
    ]);
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].account.as_deref(), Some("personal"));

    app.route_selected_sender(Route::Receipts);
    assert_eq!(
        sender_routes::get(app.db.as_ref().unwrap(), "personal", "same@example.org").unwrap(),
        Route::Receipts
    );
    assert_eq!(
        sender_routes::get(app.db.as_ref().unwrap(), "work", "same@example.org").unwrap(),
        Route::Reading
    );
}

#[test]
fn unknown_sender_cannot_be_routed_by_display_name() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    let mut mail = envelope("personal", "1", "one@example.org");
    mail.sender = Sender::Plain("Unidentified".into());
    app.handle_envelopes_loaded(vec![mail]);
    app.route_selected_sender(Route::Blocked);
    assert!(app.status_is_error);
    assert!(app.status_message.contains("sender's mailbox"));
}

#[test]
fn reply_from_a_combined_lane_uses_the_message_account() {
    let mut app = App::new(Some("personal".into()));
    app.current_folder = "All Inboxes".into();
    app.handle_envelopes_loaded(vec![envelope("work", "42", "sender@example.org")]);
    app.reply(false);
    let reply = app.compose_state.as_ref().unwrap();
    assert_eq!(reply.account.as_deref(), Some("work"));
    assert_eq!(reply.reply_to_folder.as_deref(), Some("INBOX"));
}
