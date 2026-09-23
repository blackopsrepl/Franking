/*! A per-message placement overrides only its own lane. */

use crate::app::App;
use crate::db::{message_routes, sender_routes};

use super::triage::envelope;

#[test]
fn a_placed_message_moves_lane_without_changing_its_sender() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    sender_routes::set(
        &conn,
        "work",
        "alice@example.org",
        sender_routes::Route::Inbox,
    )
    .unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.triage_lane = Some(sender_routes::Route::Inbox);
    let mail = envelope("work", "7", "alice@example.org");
    app.handle_envelopes_loaded(vec![mail.clone()]);
    assert_eq!(app.envelopes.len(), 1);
    // Place only this message in Receipts; the sender route is untouched.
    message_routes::set(
        app.db.as_ref().unwrap(),
        &mail,
        Some(sender_routes::Route::Receipts),
    )
    .unwrap();
    app.handle_envelopes_loaded(vec![mail.clone()]);
    assert!(app.envelopes.is_empty(), "it leaves the Inbox lane");
    assert_eq!(
        sender_routes::get(app.db.as_ref().unwrap(), "work", "alice@example.org").unwrap(),
        sender_routes::Route::Inbox
    );
    app.triage_lane = Some(sender_routes::Route::Receipts);
    app.handle_envelopes_loaded(vec![mail]);
    assert_eq!(app.envelopes.len(), 1, "and appears in Receipts");
}
