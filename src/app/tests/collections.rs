/*! Named collections of conversations. */

use crate::app::App;
use crate::db::collections;
use crate::keys::View;

use super::triage::envelope;

fn mail(id: &str, message_id: &str) -> crate::mail::types::Envelope {
    let mut envelope = envelope("work", id, "alice@example.org");
    envelope.message_id = Some(message_id.into());
    envelope
}

#[test]
fn a_conversation_joins_collections_and_filters() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    collections::upsert(&conn, "work", "Projects").unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.handle_envelopes_loaded(vec![mail("1", "one@x"), mail("2", "two@x")]);
    app.open_collections();
    assert_eq!(app.view, View::CollectionBoard);
    assert_eq!(app.collections.target, vec!["one@x".to_string()]);

    app.collections_toggle();
    assert_eq!(
        collections::membership_for_account(app.db.as_ref().unwrap(), "work")
            .unwrap()
            .get("one@x")
            .map(Vec::len),
        Some(1)
    );

    app.collection_filter = Some("Projects".into());
    app.handle_envelopes_loaded(vec![mail("1", "one@x"), mail("2", "two@x")]);
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].message_id.as_deref(), Some("one@x"));
}
