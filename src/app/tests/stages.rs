/*! Workflow stages assign conversations and filter the list. */

use crate::app::App;
use crate::db::stages;
use crate::keys::View;

use super::triage::envelope;

fn mail(id: &str, message_id: &str) -> crate::mail::types::Envelope {
    let mut envelope = envelope("work", id, "alice@example.org");
    envelope.message_id = Some(message_id.into());
    envelope
}

#[test]
fn legacy_stage_storage_does_not_filter_server_folder_mail() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    stages::upsert(&conn, "work", "Doing").unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.handle_envelopes_loaded(vec![mail("1", "one@x"), mail("2", "two@x")]);
    app.open_stages();
    assert_eq!(app.view, View::StageBoard);
    assert_eq!(app.stages.items.len(), 1);
    assert_eq!(app.stages.target, vec!["one@x".to_string()]);

    app.stages_assign();
    assert_eq!(
        stages::assignments_for_account(app.db.as_ref().unwrap(), "work")
            .unwrap()
            .get("one@x"),
        Some(&"Doing".to_string())
    );

    // A stored stage selection must not filter ordinary server mail.
    app.stage_filter = Some("Doing".into());
    app.handle_envelopes_loaded(vec![mail("1", "one@x"), mail("2", "two@x")]);
    assert_eq!(app.envelopes.len(), 2, "legacy stages do not filter mail");
    assert_eq!(app.envelopes[0].message_id.as_deref(), Some("one@x"));

    // Closing the board returns to the list.
    app.close_stages();
    assert_eq!(app.view, View::EnvelopeList);
}
