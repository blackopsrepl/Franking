/*! Muted conversations. */

use crate::app::App;
use crate::db::conversations;
use crate::mail::types::Envelope;

use super::envelope;

fn conversation(account: &str, id: &str, message_id: &str, in_reply_to: Option<&str>) -> Envelope {
    let mut mail = envelope(account, id, "alice@example.org");
    mail.message_id = Some(message_id.into());
    mail.in_reply_to = in_reply_to.map(str::to_string);
    mail
}

#[test]
fn muting_a_conversation_marks_it_without_changing_order() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let root = conversation("work", "1", "root@x", None);
    let reply = conversation("work", "2", "reply@x", Some("root@x"));
    app.handle_envelopes_loaded(vec![root.clone(), reply.clone()]);
    let anchors = conversations::anchors(&root);
    conversations::set_muted(app.db.as_ref().unwrap(), "work", &anchors, true).unwrap();
    let other = conversation("work", "3", "other@x", None);
    app.handle_envelopes_loaded(vec![root, reply, other]);
    assert!(app.muted_ids.contains("1"));
    assert!(
        app.muted_ids.contains("2"),
        "the reply joins the conversation"
    );
    assert_eq!(
        app.envelopes
            .iter()
            .map(|mail| mail.id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2", "3"],
        "mute leaves the ordinary order intact"
    );
}
