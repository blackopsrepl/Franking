/*! Bulk reply and the reading stream. */

use crate::app::App;
use crate::db::sender_routes;

use super::triage::envelope;

#[test]
fn bulk_reply_addresses_every_selected_sender_once() {
    let mut app = App::new(Some("work".into()));
    app.envelopes = vec![
        envelope("work", "1", "alice@example.org"),
        envelope("work", "2", "alice@example.org"),
        envelope("work", "3", "bob@example.org"),
    ];
    app.selected.insert("1".into());
    app.selected.insert("3".into());
    app.bulk_reply();
    assert_eq!(
        app.pending_bulk_to.as_deref(),
        Some("alice@example.org, bob@example.org")
    );
    assert!(app.compose_state.is_some());
    assert!(app.selected.is_empty());
}

#[test]
fn legacy_reading_lane_does_not_implicitly_select_every_message() {
    let mut app = App::new(Some("work".into()));
    app.triage_lane = Some(sender_routes::Route::Reading);
    app.envelopes = vec![
        envelope("work", "1", "alice@example.org"),
        envelope("work", "2", "bob@example.org"),
    ];
    app.envelope_state.select(Some(0));
    app.open_read_together();
    assert!(app.status_message.contains("Reading 1 messages"));
}
