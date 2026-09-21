/*! Remote backend unit tests. */

use super::roles::{pick_drafts_folder, pick_sent_folder, pick_trash_folder};
use super::search::search_criteria;
use super::template::parse_template_message;

#[test]
fn template_parser_splits_headers_and_body() {
    let parsed = parse_template_message("To: a@example.com\nSubject: Hi\n\nHello");
    assert_eq!(parsed.header("to"), Some("a@example.com"));
    assert_eq!(parsed.header("subject"), Some("Hi"));
    assert_eq!(parsed.body, "Hello");
}

#[test]
fn search_criteria_translate_the_app_query_grammar() {
    assert_eq!(search_criteria(None), "ALL");
    assert_eq!(search_criteria(Some("   ")), "ALL");
    assert_eq!(search_criteria(Some("flag seen")), "SEEN");
    assert_eq!(search_criteria(Some("not flag seen")), "UNSEEN");
    assert_eq!(search_criteria(Some("flag flagged")), "FLAGGED");
    assert_eq!(
        search_criteria(Some("subject quarterly")),
        "SUBJECT \"quarterly\""
    );
    assert_eq!(
        search_criteria(Some("from alice and not flag seen")),
        "FROM \"alice\" UNSEEN"
    );
    assert_eq!(search_criteria(Some("revenue")), "TEXT \"revenue\"");
    assert_eq!(
        search_criteria(Some("subject \"quoted\"")),
        "SUBJECT \"\\\"quoted\\\"\""
    );
}

#[test]
fn sent_folder_prefers_special_use_over_names() {
    let folders = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Gesendet".to_string(), vec!["\\Sent".to_string()]),
        ("Sent".to_string(), Vec::new()),
    ];
    assert_eq!(pick_sent_folder(&folders).as_deref(), Some("Gesendet"));

    let fallback = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Sent Items".to_string(), Vec::new()),
    ];
    assert_eq!(pick_sent_folder(&fallback).as_deref(), Some("Sent Items"));
    assert!(pick_sent_folder(&[("INBOX".to_string(), Vec::new())]).is_none());
}

#[test]
fn trash_folder_prefers_special_use_over_names() {
    let folders = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Papierkorb".to_string(), vec!["\\Trash".to_string()]),
        ("Trash".to_string(), Vec::new()),
    ];
    assert_eq!(pick_trash_folder(&folders).as_deref(), Some("Papierkorb"));

    let fallback = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Deleted Items".to_string(), Vec::new()),
    ];
    assert_eq!(
        pick_trash_folder(&fallback).as_deref(),
        Some("Deleted Items")
    );
    assert!(pick_trash_folder(&[("INBOX".to_string(), Vec::new())]).is_none());
}

#[test]
fn drafts_folder_prefers_special_use_over_names() {
    let folders = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Entwuerfe".to_string(), vec!["\\Drafts".to_string()]),
        ("Drafts".to_string(), Vec::new()),
    ];
    assert_eq!(pick_drafts_folder(&folders).as_deref(), Some("Entwuerfe"));

    let fallback = vec![
        ("INBOX".to_string(), Vec::new()),
        ("Drafts".to_string(), Vec::new()),
    ];
    assert_eq!(pick_drafts_folder(&fallback).as_deref(), Some("Drafts"));
    assert!(pick_drafts_folder(&[("INBOX".to_string(), Vec::new())]).is_none());
}
