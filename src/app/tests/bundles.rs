/*! Sender bundles collapse a sender to one row. */

use crate::app::App;
use crate::db::bundles;

use super::triage::envelope;

#[test]
fn a_bundled_sender_collapses_to_one_row_and_expands() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    bundles::set_bundled(&conn, "work", "bot@example.org", true).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.envelopes = vec![
        envelope("work", "1", "bot@example.org"),
        envelope("work", "2", "bot@example.org"),
        envelope("work", "3", "alice@example.org"),
    ];
    app.load_bundled_senders(&["work".to_string()]);
    app.apply_bundles();
    assert_eq!(app.envelopes.len(), 2, "two bot rows become one");
    let rep = app
        .envelopes
        .iter()
        .find(|envelope| envelope.sender_display() == "bot@example.org")
        .unwrap()
        .id
        .clone();
    assert_eq!(app.bundled_reps.get(&rep), Some(&2));

    // Expanding the bundle shows every row again after a reload.
    app.expanded_bundles
        .insert(("work".to_string(), "bot@example.org".to_string()));
    app.envelopes = vec![
        envelope("work", "1", "bot@example.org"),
        envelope("work", "2", "bot@example.org"),
        envelope("work", "3", "alice@example.org"),
    ];
    app.apply_bundles();
    assert_eq!(app.envelopes.len(), 3);
}
