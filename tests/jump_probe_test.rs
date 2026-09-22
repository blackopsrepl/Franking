//! Does the list selection move the way the keys say it should?
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use solverforge_mail::app::App;
use solverforge_mail::keys::View;
use solverforge_mail::mail::types::{Envelope, Sender};

fn envelope(id: &str, subject: &str) -> Envelope {
    Envelope {
        id: id.to_string(),
        flags: Vec::new(),
        subject: subject.to_string(),
        sender: Sender::Plain("a@example.com".to_string()),
        date: format!("2026-04-1{id} 09:00:00+00:00"),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }
}

#[test]
fn jump_top_selects_the_first_row() {
    let mut app = App::new(None);
    app.view = View::EnvelopeList;
    app.envelopes = vec![
        envelope("1", "first"),
        envelope("2", "second"),
        envelope("3", "third"),
    ];
    app.envelope_state.select(Some(2));

    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.envelope_state.selected(), Some(0), "g jumps to the top");

    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(
        app.envelope_state.selected(),
        Some(2),
        "G jumps to the bottom"
    );

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(
        app.envelope_state.selected(),
        Some(2),
        "j clamps at the end"
    );
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.envelope_state.selected(), Some(1), "k moves up");
}
