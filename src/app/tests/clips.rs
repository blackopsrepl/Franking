/*! Text clips: save an excerpt and retrieve it later. */

use crate::app::App;
use crate::db::clips;

#[test]
fn a_clip_is_captured_from_a_message_and_deleted_without_touching_it() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let raw = b"Subject: Report\r\nFrom: Alice <alice@example.org>\r\n\r\nConfirmation 1234";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.envelopes = vec![crate::mail::types::Envelope {
        id: "7".into(),
        flags: Vec::new(),
        subject: "Report".into(),
        sender: crate::mail::types::Sender::Plain("alice@example.org".into()),
        date: String::new(),
        message_id: Some("seven@x".into()),
        in_reply_to: None,
        account: Some("work".into()),
        folder: Some("INBOX".into()),
    }];
    app.envelope_state.select(Some(0));
    app.message_content = Some(document);
    app.view = crate::keys::View::MessageView;

    app.open_clip_prompt();
    assert_eq!(app.view, crate::keys::View::ClipPrompt);
    assert_eq!(app.clips.input, "Confirmation 1234");
    app.submit_clip();
    assert_eq!(app.view, crate::keys::View::Clips);
    assert_eq!(app.clips.items.len(), 1);

    let id = app.clips.items[0].id;
    app.delete_clip();
    app.delete_clip();
    assert!(clips::list(app.db.as_ref().unwrap()).unwrap().is_empty());
    let _ = id;
}
