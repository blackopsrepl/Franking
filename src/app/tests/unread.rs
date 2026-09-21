/*! App unit tests: unread navigation. */

#[test]
fn jumps_between_unread_messages_with_wrap_around() {
    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let envelope = |id: &str, seen: bool| Envelope {
        id: id.to_string(),
        flags: if seen {
            vec!["Seen".to_string()]
        } else {
            Vec::new()
        },
        subject: format!("Subject {id}"),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    };

    let mut app = App::new(None);
    app.envelopes = vec![
        envelope("1", true),
        envelope("2", false),
        envelope("3", true),
        envelope("4", false),
    ];
    app.envelope_state.select(Some(0));

    app.jump_unread(true);
    assert_eq!(app.selected_envelope_id(), Some("2"));
    app.jump_unread(true);
    assert_eq!(app.selected_envelope_id(), Some("4"));
    app.jump_unread(true);
    assert_eq!(app.selected_envelope_id(), Some("2"), "wraps forward");
    app.jump_unread(false);
    assert_eq!(app.selected_envelope_id(), Some("4"), "wraps backward");
}

#[test]
fn reports_when_everything_is_read() {
    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let mut app = App::new(None);
    app.envelopes = vec![Envelope {
        id: "1".to_string(),
        flags: vec!["Seen".to_string()],
        subject: "Read".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: None,
    }];
    app.envelope_state.select(Some(0));
    app.jump_unread(true);
    assert!(app.status_message.contains("No unread messages"));
}
