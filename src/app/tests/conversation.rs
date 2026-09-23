/*! Quiet conversations and resurfacing. */

use crate::app::App;
use crate::db::conversations;
use crate::mail::types::Envelope;

use super::triage::envelope;

fn conversation(account: &str, id: &str, message_id: &str, in_reply_to: Option<&str>) -> Envelope {
    let mut mail = envelope(account, id, "alice@example.org");
    mail.message_id = Some(message_id.into());
    mail.in_reply_to = in_reply_to.map(str::to_string);
    mail
}

#[test]
fn quieting_a_conversation_orders_it_last_and_marks_it() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let root = conversation("work", "1", "root@x", None);
    let reply = conversation("work", "2", "reply@x", Some("root@x"));
    app.handle_envelopes_loaded(vec![root.clone(), reply.clone()]);
    let anchors = conversations::anchors(&root);
    conversations::set_muted(app.db.as_ref().unwrap(), "work", &anchors, true).unwrap();
    app.handle_envelopes_loaded(vec![root, reply]);
    assert!(app.muted_ids.contains("1"));
    assert!(
        app.muted_ids.contains("2"),
        "the reply joins the conversation"
    );
    assert_eq!(
        app.envelopes.last().unwrap().id,
        "2",
        "quieted mail sorts last"
    );
}

#[test]
fn a_resurfaced_conversation_sorts_first() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let older = conversation("work", "1", "root@x", None);
    let newer = conversation("work", "2", "other@x", None);
    let anchors = conversations::anchors(&older);
    conversations::set_resurface(
        app.db.as_ref().unwrap(),
        "work",
        &anchors,
        Some("2020-01-01T00:00:00Z"),
    )
    .unwrap();
    app.handle_envelopes_loaded(vec![newer, older]);
    assert!(app.resurfaced_ids.contains("1"));
    assert_eq!(app.envelopes.first().unwrap().id, "1");
}

#[test]
fn clearing_a_resurface_delay_restores_order() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let mail = conversation("work", "1", "root@x", None);
    let anchors = conversations::anchors(&mail);
    conversations::set_resurface(
        app.db.as_ref().unwrap(),
        "work",
        &anchors,
        Some("2020-01-01T00:00:00Z"),
    )
    .unwrap();
    conversations::set_resurface(app.db.as_ref().unwrap(), "work", &anchors, None).unwrap();
    app.handle_envelopes_loaded(vec![mail]);
    assert!(app.resurfaced_ids.is_empty());
}
