/*! Private notes and display-only subject aliases. */

use crate::app::App;
use crate::db::annotations;
use crate::keys::View;

use super::triage::envelope;

#[test]
fn a_note_is_saved_and_cleared_through_the_prompt() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let mail = envelope("work", "7", "alice@example.org");
    app.handle_envelopes_loaded(vec![mail.clone()]);
    app.open_note_prompt();
    assert_eq!(app.view, View::MessageNote);
    for c in "call Tuesday".chars() {
        app.annotation_input(c);
    }
    app.submit_annotation();
    assert_eq!(
        annotations::note(app.db.as_ref().unwrap(), &mail)
            .unwrap()
            .as_deref(),
        Some("call Tuesday")
    );
    assert_eq!(app.view, View::EnvelopeList);
}

#[test]
fn a_subject_alias_is_applied_to_the_list_only() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let mut mail = envelope("work", "7", "alice@example.org");
    mail.message_id = Some("root@x".into());
    app.handle_envelopes_loaded(vec![mail.clone()]);
    app.open_subject_alias_prompt();
    assert_eq!(app.view, View::SubjectAlias);
    for c in "Q3 pricing".chars() {
        app.annotation_input(c);
    }
    app.submit_annotation();
    assert_eq!(
        annotations::alias(app.db.as_ref().unwrap(), "work", &["root@x".to_string()])
            .unwrap()
            .as_deref(),
        Some("Q3 pricing")
    );
    app.handle_envelopes_loaded(vec![mail.clone()]);
    assert_eq!(
        app.subject_aliases.get("7").map(String::as_str),
        Some("Q3 pricing")
    );
    // The server-visible subject is untouched.
    assert_eq!(mail.subject, "7");
}
