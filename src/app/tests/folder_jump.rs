/*! App unit tests: folder incremental search. */

#[test]
fn typing_in_the_sidebar_jumps_to_matching_folders() {
    use crate::keys::View;
    use crate::mail::types::{Folder, FolderRole};

    use super::super::App;

    let folder = |name: &str| Folder {
        name: name.to_string(),
        desc: None,
        role: FolderRole::Other,
    };

    let mut app = App::new(None);
    app.view = View::FolderList;
    app.folders = vec![
        folder("INBOX"),
        folder("Sent"),
        folder("Archive"),
        folder("Archive 2024"),
    ];

    app.folder_jump_input('a');
    assert_eq!(app.folders[app.folder_index].name, "Archive");
    assert!(app.status_message.contains("Archive"));

    // A longer prefix narrows the match.
    for c in "rchive 2".chars() {
        app.folder_jump_input(c);
    }
    assert_eq!(app.folders[app.folder_index].name, "Archive 2024");

    // Removing a character widens it again.
    app.folder_jump_backspace();
    assert_eq!(app.folders[app.folder_index].name, "Archive 2024");

    app.folder_jump_clear();
    assert!(app.folder_jump.is_empty());
    assert_eq!(app.view, View::FolderList, "stays in the sidebar");

    // Clearing an empty query leaves the sidebar.
    app.folder_jump_clear();
    assert_eq!(app.view, View::EnvelopeList);
}

#[test]
fn a_folder_query_with_no_match_reports_it() {
    use crate::mail::types::{Folder, FolderRole};

    use super::super::App;

    let mut app = App::new(None);
    app.view = crate::keys::View::FolderList;
    app.folders = vec![Folder {
        name: "INBOX".to_string(),
        desc: None,
        role: FolderRole::Inbox,
    }];
    app.folder_jump_input('z');
    assert!(app.status_message.contains("No folder starts with"));
}
