/*! App unit tests: move and copy pickers. */

#[test]
fn copy_prompt_targets_the_same_picker_without_undo() {
    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "INBOX".to_string();
    app.envelopes = vec![Envelope {
        id: "9".to_string(),
        flags: Vec::new(),
        subject: "Subject".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));

    app.enter_copy_prompt();
    assert!(app.move_is_copy);
    assert_eq!(app.view, crate::keys::View::MovePrompt);

    app.enter_move_prompt();
    assert!(!app.move_is_copy);
}
