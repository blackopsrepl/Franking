/*! Local conversation merges. */

use crate::app::App;
use crate::db::thread_merges;

use super::triage::envelope;

fn mail(id: &str, message_id: &str) -> crate::mail::types::Envelope {
    let mut envelope = envelope("work", id, "alice@example.org");
    envelope.message_id = Some(message_id.into());
    envelope
}

#[test]
fn two_conversations_merge_and_share_a_thread_key() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let first = mail("1", "a@x");
    let second = mail("2", "b@x");
    app.handle_envelopes_loaded(vec![first.clone(), second.clone()]);

    app.envelope_state.select(Some(0));
    app.merge_conversation();
    assert!(
        app.pending_merge.is_some(),
        "the first press marks the root"
    );
    app.envelope_state.select(Some(1));
    app.merge_conversation();

    assert_eq!(
        thread_merges::roots_for_account(app.db.as_ref().unwrap(), "work")
            .unwrap()
            .get("b@x"),
        Some(&"a@x".to_string())
    );

    // After a reload the merged pair shares one thread key.
    app.handle_envelopes_loaded(vec![first, second]);
    assert_eq!(app.merge_roots.get("b@x"), Some(&"a@x".to_string()));
    assert!(app.thread_root_keys().iter().all(|key| key == "a@x"));
}
