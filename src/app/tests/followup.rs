/*! Follow-up queues keep independent state per source account. */

use crate::app::App;
use crate::db::message_markers::{self, Marker};

use super::triage::envelope;

#[test]
fn reply_and_saved_lanes_show_marked_mail_across_accounts() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    app.current_folder = "All Inboxes".into();
    app.handle_envelopes_loaded(vec![
        envelope("personal", "1", "alice@example.org"),
        envelope("work", "2", "bob@example.org"),
    ]);
    app.toggle_marker(Marker::ReplyLater);
    let marked = app.selected_envelope().unwrap().clone();
    app.toggle_marker(Marker::Saved);
    assert!(message_markers::has(app.db.as_ref().unwrap(), &marked, Marker::Saved).unwrap());
    app.open_followup(Marker::ReplyLater);
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].account, marked.account);
    app.toggle_marker(Marker::ReplyLater);
    assert!(app.envelopes.is_empty());
    app.open_followup(Marker::Saved);
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].sender_display(), marked.sender_display());
}

#[test]
fn reply_retains_the_original_source_for_completion() {
    let mut app = App::new(Some("personal".into()));
    app.current_folder = "All Inboxes".into();
    app.handle_envelopes_loaded(vec![envelope("work", "7", "alice@example.org")]);
    app.reply(false);
    let pending = app.pending_reply_marker.as_ref().unwrap();
    assert_eq!(pending.account.as_deref(), Some("work"));
    assert_eq!(pending.folder.as_deref(), Some("INBOX"));
}

#[test]
fn completing_a_reply_clears_only_its_own_queue_entry() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    app.current_folder = "All Inboxes".into();
    let work = envelope("work", "7", "alice@example.org");
    let personal = envelope("personal", "7", "alice@example.org");
    message_markers::set(app.db.as_ref().unwrap(), &work, Marker::ReplyLater, true).unwrap();
    message_markers::set(
        app.db.as_ref().unwrap(),
        &personal,
        Marker::ReplyLater,
        true,
    )
    .unwrap();
    message_markers::set(app.db.as_ref().unwrap(), &work, Marker::Saved, true).unwrap();
    app.handle_envelopes_loaded(vec![work.clone()]);
    app.reply(false);
    app.complete_reply_marker().unwrap();
    assert!(!message_markers::has(app.db.as_ref().unwrap(), &work, Marker::ReplyLater).unwrap());
    assert!(message_markers::has(app.db.as_ref().unwrap(), &work, Marker::Saved).unwrap());
    assert!(message_markers::has(app.db.as_ref().unwrap(), &personal, Marker::ReplyLater).unwrap());
}
