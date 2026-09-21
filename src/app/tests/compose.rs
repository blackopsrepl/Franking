/*! App unit tests: compose attachment management. */

#[test]
fn compose_attachment_list_removes_entries() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::compose::{ComposeMode, ComposeState, FocusedField};
    use crate::keys::View;

    use super::super::App;

    let mut app = App::new(None);
    let mut state = ComposeState::new(ComposeMode::New, None);
    state.attachments = vec!["/tmp/a.txt".to_string(), "/tmp/b.pdf".to_string()];
    app.compose_state = Some(state);
    app.view = View::Compose;

    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);

    // The Files button opens the overlay.
    app.compose_state.as_mut().unwrap().focused = FocusedField::Files;
    app.compose_enter_insert();
    assert!(app.compose_state.as_ref().unwrap().attach_list_open);

    // Move down and remove the second entry.
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.compose_state.as_ref().unwrap().attach_index, 1);
    app.handle_key(key(KeyCode::Enter));

    let state = app.compose_state.as_ref().unwrap();
    assert_eq!(state.attachments, vec!["/tmp/a.txt".to_string()]);
    assert!(state.dirty);
    assert!(state.attach_list_open);

    // Removing the last entry closes the overlay.
    app.handle_key(key(KeyCode::Char('d')));
    let state = app.compose_state.as_ref().unwrap();
    assert!(state.attachments.is_empty());
    assert!(!state.attach_list_open);

    // Esc closes the overlay without touching the list.
    app.handle_key(key(KeyCode::Esc));
    assert!(app.compose_state.is_some());
}
