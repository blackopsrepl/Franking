//! Identity form and compose editor interaction tests.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use solverforge_mail::app::App;
use solverforge_mail::compose::{ComposeMode, ComposeState, FocusedField};

#[test]
fn tab_to_save_saves_identity_edit_form() {
    use rusqlite::Connection;
    use solverforge_mail::identity_edit::{IdentityEditState, IdentityField};
    use solverforge_mail::keys::View;

    // Set up an in-memory DB with the full schema.
    let conn = Connection::open_in_memory().unwrap();
    solverforge_mail::db::init_for_test(&conn).unwrap();

    let mut app = App::new(Some("testaccount".to_string()));
    app.db = Some(conn);
    app.view = View::IdentityEdit;

    // Pre-fill a valid form.
    let mut state = IdentityEditState::new("testaccount");
    state.name = "Work".to_string();
    state.email = "work@example.com".to_string();
    app.identity_edit_state = Some(state);

    // Tab through fields to reach the Save button:
    // Name → SenderName → Email → Signature → SentFolder → IsDefault → Save
    let tab = KeyEvent {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    for _ in 0..6 {
        app.handle_key(tab);
    }

    // Verify focus is now on Save.
    assert_eq!(
        app.identity_edit_state.as_ref().unwrap().focused,
        IdentityField::Save,
        "focus should be on Save after the fields and toggles"
    );

    // Press Enter to activate Save.
    let enter = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    app.handle_key(enter);

    // The modal should be gone and we should be back on IdentityList.
    assert!(
        app.identity_edit_state.is_none(),
        "modal should close after save"
    );
    assert_eq!(app.view, View::IdentityList);
    assert_eq!(app.identities.len(), 1);
    assert_eq!(app.identities[0].email, "work@example.com");
}

#[test]
fn compose_typing_updates_to_field_directly() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, None);
    compose.focused = FocusedField::To;
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert_eq!(cs.to, "a");
    assert!(cs.dirty);
}

#[test]
fn compose_typing_updates_subject_directly() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, None);
    compose.focused = FocusedField::Subject;
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert_eq!(cs.subject, "H");
}

#[test]
fn compose_confirm_discard_blocks_ctrl_passthrough_on_from_field() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::identities::Identity;
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, Some("test".to_string()));
    compose.focused = FocusedField::From;
    compose.confirm_discard = true;
    compose.from_identities = vec![
        Identity {
            id: 1,
            account: "test".to_string(),
            name: Some("Work".to_string()),
            display_name: Some("Work".to_string()),
            email: "work@example.com".to_string(),
            signature: None,
            sent_folder: None,
            is_default: true,
        },
        Identity {
            id: 2,
            account: "test".to_string(),
            name: Some("Alt".to_string()),
            display_name: Some("Alt".to_string()),
            email: "alt@example.com".to_string(),
            signature: None,
            sent_folder: None,
            is_default: false,
        },
    ];
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert!(cs.confirm_discard);
    assert_eq!(cs.from_idx, None);
}

#[test]
fn compose_body_escape_does_not_mark_pristine_draft_dirty() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, None);
    compose.focused = FocusedField::Body;
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert!(!cs.dirty);
    assert!(!cs.confirm_discard);
}

#[test]
fn compose_body_navigation_does_not_mark_pristine_draft_dirty() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, None);
    compose.focused = FocusedField::Body;
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert!(!cs.dirty);
    assert!(!cs.confirm_discard);
}

#[test]
fn compose_tab_out_of_body_clears_active_search() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use solverforge_mail::keys::View;

    let mut app = App::new(None);
    app.view = View::Compose;

    let mut compose = ComposeState::new(ComposeMode::New, None);
    compose.focused = FocusedField::Body;
    app.compose_state = Some(compose);

    app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    let cs = app
        .compose_state
        .as_ref()
        .expect("compose state should exist");
    assert_eq!(cs.focused, FocusedField::Send);
    assert!(!cs.body.is_search_active());
    assert_eq!(cs.body.search_query(), "");
}
