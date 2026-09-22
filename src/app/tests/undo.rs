/*! App unit tests: undo. */

#[test]
fn deleting_records_a_move_back_from_trash() {
    use crate::app::undo::UndoOp;
    use crate::mail::types::{Envelope, Folder, FolderRole, Sender};

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
            name: "Deleted Items".to_string(),
            desc: None,
            role: FolderRole::Trash,
            subscribed: None,
        },
    ];
    app.envelopes = vec![Envelope {
        id: "7".to_string(),
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

    assert_eq!(app.trash_folder().as_deref(), Some("Deleted Items"));
    app.delete();

    match app.pending_undo.as_ref().expect("recorded undo") {
        UndoOp::Move { from, to, ids } => {
            assert_eq!(from, "INBOX");
            assert_eq!(to, "Deleted Items");
            assert_eq!(ids, &vec!["7".to_string()]);
        }
        other => panic!("unexpected undo: {other:?}"),
    }
}

#[test]
fn deleting_inside_trash_is_not_undoable() {
    use crate::mail::types::{Envelope, Folder, FolderRole, Sender};

    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "Trash".to_string();
    app.folders = vec![Folder {
        name: "Trash".to_string(),
        desc: None,
        role: FolderRole::Trash,
        subscribed: None,
    }];
    app.envelopes = vec![Envelope {
        id: "7".to_string(),
        flags: Vec::new(),
        subject: "Subject".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("Trash".to_string()),
    }];
    app.envelope_state.select(Some(0));

    app.delete();
    assert!(app.pending_undo.is_none());
}

#[test]
fn undo_without_a_pending_action_reports_status() {
    use super::super::App;

    let mut app = App::new(None);
    app.undo();
    assert!(app.status_message.contains("Nothing to undo"));
}

#[test]
fn autosaves_a_dirty_message_and_clears_it_on_discard() {
    use crate::compose::{ComposeMode, ComposeState};
    use crate::keys::View;

    use super::super::autosave;
    use super::super::App;

    let dir = std::env::temp_dir().join(format!(
        "sfmail-autosave-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));

    let mut app = App::new(None);
    app.autosave_dir = dir.clone();
    let mut state = ComposeState::new(ComposeMode::New, None);
    state.to = "bob@example.com".to_string();
    state.subject = "Autosaved".to_string();
    state.dirty = true;
    app.compose_state = Some(state);
    app.view = View::Compose;

    app.autosave_ticks = 500;
    app.tick();
    let saved = autosave::read(&dir).expect("autosave file");
    assert!(saved.contains("To: bob@example.com"));
    assert!(saved.contains("Subject: Autosaved"));

    // Discarding asks for confirmation, then clears the autosave.
    app.compose_discard();
    assert!(app.compose_state.as_ref().expect("compose").confirm_discard);
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(autosave::read(&dir).is_none());

    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(app.view, View::EnvelopeList);
}

#[test]
fn recovers_an_autosaved_message_at_startup() {
    use crate::keys::View;

    use super::super::autosave;
    use super::super::App;

    let dir = std::env::temp_dir().join(format!(
        "sfmail-recover-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    autosave::write(&dir, "To: bob@example.com\nSubject: Recover me\n\nbody").unwrap();

    let mut app = App::new(None);
    app.autosave_dir = dir.clone();
    assert!(app.recover_autosave());
    assert_eq!(app.view, View::Compose);
    let state = app.compose_state.as_ref().expect("compose");
    assert_eq!(state.to, "bob@example.com");
    assert_eq!(state.subject, "Recover me");

    let _ = std::fs::remove_dir_all(&dir);
}

fn threaded_envelope(
    id: &str,
    message_id: &str,
    parent: Option<&str>,
) -> crate::mail::types::Envelope {
    crate::mail::types::Envelope {
        id: id.to_string(),
        flags: Vec::new(),
        subject: format!("Subject {id}"),
        sender: crate::mail::types::Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: Some(message_id.to_string()),
        in_reply_to: parent.map(str::to_string),
        account: None,
        folder: Some("INBOX".to_string()),
    }
}

#[test]
fn collapses_and_expands_a_thread() {
    use super::super::App;

    let mut app = App::new(None);
    app.threaded = true;
    app.envelopes = vec![
        threaded_envelope("1", "a@example.com", None),
        threaded_envelope("2", "b@example.com", Some("a@example.com")),
        threaded_envelope("3", "c@example.com", Some("b@example.com")),
        threaded_envelope("4", "d@example.com", None),
    ];
    app.envelope_state.select(Some(0));

    app.collapse_thread();
    assert_eq!(app.envelopes.len(), 2, "replies hidden");
    assert_eq!(app.envelopes[1].id, "4");
    assert!(app.collapsed_threads.contains_key("a@example.com"));

    // Collapsing from a reply folds the whole thread.
    app.envelope_state.select(Some(0));
    app.expand_thread();
    assert_eq!(app.envelopes.len(), 4);
    assert!(app.collapsed_threads.is_empty());
}

#[test]
fn collapsing_off_threading_reports_and_hides_nothing() {
    use super::super::App;

    let mut app = App::new(None);
    app.threaded = false;
    app.envelopes = vec![threaded_envelope("1", "a@example.com", None)];
    app.envelope_state.select(Some(0));
    app.collapse_thread();
    assert_eq!(app.envelopes.len(), 1);
    assert!(app.status_message.contains("Threading is off"));
}

#[test]
fn expanding_without_a_collapsed_thread_reports_status() {
    use super::super::App;

    let mut app = App::new(None);
    app.envelopes = vec![threaded_envelope("1", "a@example.com", None)];
    app.envelope_state.select(Some(0));
    app.expand_thread();
    assert!(app.status_message.contains("No collapsed thread"));
}

#[test]
fn emptying_a_folder_requires_two_presses() {
    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "Trash".to_string();

    app.empty_folder();
    assert_eq!(app.pending_empty_folder.as_deref(), Some("Trash"));
    assert!(app.status_message.contains("Press E again"));

    // Cancelling by moving away clears the confirmation.
    app.pending_empty_folder = None;
    app.empty_folder();
    assert!(app.pending_empty_folder.is_some());
}
