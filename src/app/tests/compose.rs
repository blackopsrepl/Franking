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

#[test]
fn tab_in_the_attach_prompt_opens_the_file_picker() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::compose::{ComposeMode, ComposeState};
    use crate::keys::View;

    use super::super::App;

    let mut app = App::new(None);
    app.compose_state = Some(ComposeState::new(ComposeMode::New, None));
    app.view = View::Compose;
    app.compose_state.as_mut().unwrap().attach_input = Some(String::new());

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.view, View::FilePicker);
    assert!(app.file_picker.is_some());
    assert!(app.compose_state.is_some(), "compose state survives");

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.view, View::Compose);
}

#[test]
fn picking_a_file_attaches_it() {
    use crate::compose::{ComposeMode, ComposeState};
    use crate::file_picker::FilePickerState;

    use super::super::App;

    let root = std::env::temp_dir().join(format!(
        "sfmail-pick-attach-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("report.pdf"), b"pdf").unwrap();

    let mut app = App::new(None);
    let mut state = ComposeState::new(ComposeMode::New, None);
    state.attach_input = Some(String::new());
    app.compose_state = Some(state);
    app.file_picker = Some(FilePickerState::open(Some(root.clone())));

    app.file_picker_enter();
    let cs = app.compose_state.as_ref().expect("compose");
    assert_eq!(
        cs.attachments,
        vec![root.join("report.pdf").display().to_string()]
    );
    assert!(cs.dirty);
    assert!(cs.attach_input.is_none());
    assert!(app.file_picker.is_none());

    let _ = std::fs::remove_dir_all(&root);
}
