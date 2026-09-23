/*! Read several selected messages in one scroll. */

use crate::app::App;
use crate::keys::View;
use crate::mail::types::{Envelope, Sender};

fn envelope(account: &str, id: &str) -> Envelope {
    Envelope {
        id: id.into(),
        flags: Vec::new(),
        subject: format!("Subject {id}"),
        sender: Sender::Plain("alice@example.org".into()),
        date: "2026-09-23".into(),
        message_id: None,
        in_reply_to: None,
        account: Some(account.into()),
        folder: Some("INBOX".into()),
    }
}

#[test]
fn read_together_renders_each_selected_message_with_its_identity() {
    let mut app = App::new(Some("work".into()));
    app.current_folder = "All Inboxes".into();
    app.handle_envelopes_loaded(vec![envelope("work", "1"), envelope("personal", "2")]);
    // Select both rows and open them together.
    app.selected.insert("1".into());
    app.selected.insert("2".into());
    let first =
        crate::mail::mime::parse_message(b"Subject: One\r\nFrom: a@x\r\n\r\nfirst body").unwrap();
    let second =
        crate::mail::mime::parse_message(b"Subject: Two\r\nFrom: b@x\r\n\r\nsecond body").unwrap();
    app.handle_read_together(vec![first, second]);
    assert_eq!(app.view, View::ReadTogether);
    let rendered = app.render_read_together(70);
    assert!(rendered.contains("[1/2]"));
    assert!(rendered.contains("[2/2]"));
    assert!(rendered.contains("first body"));
    assert!(rendered.contains("second body"));
    app.scroll(1);
    assert_eq!(app.read_together_scroll, 1);
    app.close_read_together();
    assert_eq!(app.view, View::EnvelopeList);
    assert!(app.read_together.is_none());
}
