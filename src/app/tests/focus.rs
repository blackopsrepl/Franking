/*! Sequential reply queue over Reply later. */

use crate::app::App;
use crate::db::message_markers::{self, Marker};
use crate::keys::View;

use super::triage::envelope;

#[test]
fn focus_reply_steps_through_the_response_queue() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let first = envelope("work", "1", "alice@example.org");
    let second = envelope("work", "2", "bob@example.org");
    message_markers::set(&conn, &first, Marker::ReplyLater, true).unwrap();
    message_markers::set(&conn, &second, Marker::ReplyLater, true).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.open_focus_reply();
    assert_eq!(app.view, View::FocusReply);
    assert_eq!(app.focus.as_ref().unwrap().queue.len(), 2);

    // Completing the current message drops it from the queue and its marker.
    let current = app.focus.as_ref().unwrap().queue[0].clone();
    app.focus_done();
    let queue = &app.focus.as_ref().unwrap().queue;
    assert_eq!(queue.len(), 1);
    assert!(!queue.iter().any(|envelope| envelope.id == current.id));
    assert!(!message_markers::has(app.db.as_ref().unwrap(), &current, Marker::ReplyLater).unwrap());

    app.close_focus();
    assert_eq!(app.view, View::EnvelopeList);
    assert!(app.focus.is_none());
}

#[test]
fn focus_reply_with_an_empty_queue_stays_put() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.open_focus_reply();
    assert_eq!(app.view, View::EnvelopeList);
    assert!(app.status_message.contains("empty"));
}
