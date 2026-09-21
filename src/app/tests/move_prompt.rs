/*! App unit tests: move-to-folder picker. */

#[test]
fn move_picker_filters_and_highlights_folders() {
    use crate::mail::types::{Folder, FolderRole};

    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "INBOX".to_string();
    app.folders = vec![
        Folder {
            name: "INBOX".to_string(),
            desc: None,
            role: FolderRole::Inbox,
            subscribed: None,
        },
        Folder {
            name: "Archive".to_string(),
            desc: None,
            role: FolderRole::Archive,
            subscribed: None,
        },
        Folder {
            name: "Archive 2024".to_string(),
            desc: None,
            role: FolderRole::Archive,
            subscribed: None,
        },
        Folder {
            name: "Trash".to_string(),
            desc: None,
            role: FolderRole::Trash,
            subscribed: None,
        },
    ];

    // All folders except the current one.
    assert_eq!(app.move_candidates().len(), 3);
    assert_eq!(app.resolved_move_target().as_deref(), Some("Archive"));

    app.move_next_candidate();
    assert_eq!(app.resolved_move_target().as_deref(), Some("Archive 2024"));

    // Typing filters the list and resets the highlight.
    app.view = crate::keys::View::MovePrompt;
    for c in "tra".chars() {
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::NONE,
        ));
    }
    assert_eq!(app.move_candidates(), vec!["Trash".to_string()]);
    assert_eq!(app.resolved_move_target().as_deref(), Some("Trash"));
}

#[test]
fn move_prompt_without_matches_falls_back_to_typed_text() {
    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "INBOX".to_string();
    app.move_target = "New Folder".to_string();
    assert_eq!(app.resolved_move_target().as_deref(), Some("New Folder"));

    app.move_target.clear();
    assert!(app.resolved_move_target().is_none());
}
