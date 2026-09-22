/*! Leaving a compose session. */

#[test]
fn escape_leaves_a_reply_from_a_header_field() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::super::App;

    let mut app = App::new(None);
    app.compose_state = Some(crate::compose::ComposeState::new(
        crate::compose::ComposeMode::Reply,
        None,
    ));
    app.view = crate::keys::View::Compose;

    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    // A pristine reply closes without a confirmation prompt.
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view, crate::keys::View::EnvelopeList, "compose closed");
    assert!(app.compose_state.is_none());
}

#[test]
fn escape_in_the_body_asks_to_discard() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::compose::{ComposeMode, ComposeState, FocusedField};

    use super::super::App;

    let mut app = App::new(None);
    let mut state = ComposeState::new(ComposeMode::Reply, None);
    // A reply starts in the body, with the quoted text already in it.
    state.focused = FocusedField::Body;
    state.dirty = true;
    app.compose_state = Some(state);
    app.view = crate::keys::View::Compose;

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(
        app.compose_state
            .as_ref()
            .is_some_and(|cs| cs.confirm_discard),
        "the discard confirmation is offered instead of doing nothing"
    );
}
