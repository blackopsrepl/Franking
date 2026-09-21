/*! App unit tests: archiving. */

#[test]
fn archive_targets_the_archive_role_or_name() {
    use crate::mail::types::{Folder, FolderRole};

    use super::super::App;

    let folder = |name: &str, role: FolderRole| Folder {
        name: name.to_string(),
        desc: None,
        role,
    };

    let mut app = App::new(None);
    assert!(app.archive_folder().is_none());

    app.folders = vec![
        folder("INBOX", FolderRole::Inbox),
        folder("All Mail", FolderRole::Archive),
    ];
    assert_eq!(app.archive_folder().as_deref(), Some("All Mail"));

    app.folders = vec![
        folder("INBOX", FolderRole::Inbox),
        folder("Archive", FolderRole::Other),
    ];
    assert_eq!(app.archive_folder().as_deref(), Some("Archive"));
}

#[test]
fn archiving_reports_when_there_is_no_archive_folder() {
    use super::super::App;

    let mut app = App::new(None);
    app.archive();
    assert!(app.status_message.contains("no archive folder"));
}
