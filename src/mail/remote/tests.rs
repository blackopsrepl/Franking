/*! Remote backend unit tests. */

use super::send::pick_role_folder;
use super::template::parse_template_message;
use crate::mail::types::{Folder, FolderRole};

fn folder(name: &str, role: FolderRole) -> Folder {
    Folder {
        name: name.to_string(),
        desc: None,
        role,
    }
}

#[test]
fn template_parser_splits_headers_and_body() {
    let parsed = parse_template_message("To: a@example.com\nSubject: Hi\n\nHello");
    assert_eq!(parsed.header("to"), Some("a@example.com"));
    assert_eq!(parsed.header("subject"), Some("Hi"));
    assert_eq!(parsed.body, "Hello");
}

#[test]
fn roles_prefer_special_use_attributes_over_names() {
    let folders = vec![
        folder("INBOX", FolderRole::Inbox),
        folder("Gesendet", FolderRole::Sent),
        folder("Sent", FolderRole::Other),
    ];
    assert_eq!(
        pick_role_folder(&folders, FolderRole::Sent).as_deref(),
        Some("Gesendet")
    );
}

#[test]
fn roles_fall_back_to_historical_and_localized_names() {
    let folders = vec![
        folder("INBOX", FolderRole::Inbox),
        folder("Sent Items", FolderRole::Other),
        folder("Papierkorb", FolderRole::Trash),
        folder("Entwuerfe", FolderRole::Drafts),
    ];
    assert_eq!(
        pick_role_folder(&folders, FolderRole::Sent).as_deref(),
        Some("Sent Items"),
        "name fallback finds senders without the RFC 6154 attribute"
    );
    assert_eq!(
        pick_role_folder(&folders, FolderRole::Trash).as_deref(),
        Some("Papierkorb")
    );
    assert_eq!(
        pick_role_folder(&folders, FolderRole::Drafts).as_deref(),
        Some("Entwuerfe")
    );
    assert!(pick_role_folder(&[folder("INBOX", FolderRole::Inbox)], FolderRole::Sent).is_none());
}
