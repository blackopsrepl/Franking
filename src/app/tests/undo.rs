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
        },
        Folder {
            name: "Deleted Items".to_string(),
            desc: None,
            role: FolderRole::Trash,
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
