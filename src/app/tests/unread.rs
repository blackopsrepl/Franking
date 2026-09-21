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

#[test]
fn cycles_the_message_list_ordering() {
    use crate::app::App;
    use crate::mail::sort::{SortKey, SortOrder};
    use crate::mail::types::{Envelope, Sender};

    let envelope = |id: &str, sender: &str, date: &str| Envelope {
        id: id.to_string(),
        flags: Vec::new(),
        subject: format!("Subject {id}"),
        sender: Sender::Plain(sender.to_string()),
        date: date.to_string(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: None,
    };

    let mut app = App::new(None);
    app.envelopes = vec![
        envelope("1", "carol@example.com", "2026-01-01"),
        envelope("2", "alice@example.com", "2026-01-03"),
    ];

    assert_eq!(app.sort_order, SortOrder::default());
    app.cycle_sort_order();
    assert_eq!(app.sort_order.key, SortKey::Date);
    assert!(!app.sort_order.descending);
    let ids: Vec<&str> = app.envelopes.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["1", "2"], "oldest first");

    app.cycle_sort_order();
    assert_eq!(app.sort_order.key, SortKey::Sender);
    assert!(app.sort_order.descending, "each key starts descending");
    let ids: Vec<&str> = app.envelopes.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["1", "2"], "carol before alice when descending");
    assert!(app.status_message.contains("sender"));

    app.cycle_sort_order();
    assert_eq!(app.sort_order.key, SortKey::Sender);
    assert!(!app.sort_order.descending);
    let ids: Vec<&str> = app.envelopes.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["2", "1"], "alice before carol when ascending");

    // Threaded view keeps the server order.
    app.threaded = true;
    let before = app
        .envelopes
        .iter()
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    app.cycle_sort_order();
    let after = app
        .envelopes
        .iter()
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    assert!(app.status_message.contains("threaded view"));
}
