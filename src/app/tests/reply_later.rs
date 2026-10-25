/*! Reply later keeps independent state per source account. */

use crate::app::{App, LocalView};
use crate::db::message_markers;

use super::envelope;

#[test]
fn reply_later_shows_marked_mail_across_accounts() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    app.current_folder = "All Inboxes".into();
    app.handle_envelopes_loaded(vec![
        envelope("personal", "1", "alice@example.org"),
        envelope("work", "2", "bob@example.org"),
    ]);
    app.toggle_reply_later();
    let marked = app.selected_envelope().unwrap().clone();
    app.open_reply_later();
    assert_eq!(app.local_view, Some(LocalView::ReplyLater));
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].account, marked.account);
    app.toggle_reply_later();
    assert!(app.envelopes.is_empty());
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
fn completing_a_reply_clears_only_its_own_entry() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("personal".into()));
    app.db = Some(conn);
    app.current_folder = "All Inboxes".into();
    let work = envelope("work", "7", "alice@example.org");
    let personal = envelope("personal", "7", "alice@example.org");
    let conn = app.db.as_ref().unwrap();
    message_markers::set_reply_later(conn, &work, true).unwrap();
    message_markers::set_reply_later(conn, &personal, true).unwrap();
    app.handle_envelopes_loaded(vec![work.clone()]);
    app.reply(false);
    app.complete_reply_marker().unwrap();
    let conn = app.db.as_ref().unwrap();
    assert!(!message_markers::is_reply_later(conn, &work).unwrap());
    assert!(message_markers::is_reply_later(conn, &personal).unwrap());
}
