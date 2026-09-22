/*! App unit tests: compose toggles, attachments, folders. */

#[test]
fn compose_toggles_signing_and_encryption() {
    use crate::compose::{ComposeMode, ComposeState, FocusedField};

    use super::super::App;

    let mut app = App::new(None);
    app.compose_state = Some(ComposeState::new(ComposeMode::New, None));

    let focused = |app: &mut App, field: FocusedField| {
        app.compose_state.as_mut().expect("compose").focused = field;
        app.compose_enter_insert();
    };

    focused(&mut app, FocusedField::Sign);
    assert!(app.compose_state.as_ref().expect("compose").sign);

    focused(&mut app, FocusedField::Encrypt);
    assert!(app.compose_state.as_ref().expect("compose").encrypt);
    focused(&mut app, FocusedField::Encrypt);
    assert!(!app.compose_state.as_ref().expect("compose").encrypt);
}

#[test]
fn attachment_list_opens_navigates_and_closes() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAECAwQ=\r\n--m--\r\n";
    let document = crate::mail::mime::parse_message(raw).unwrap();

    let mut app = App::new(None);
    app.message_content = Some(document);

    app.open_attachments();
    assert_eq!(app.view, View::AttachmentList);
    assert_eq!(app.attachment_index, 0);

    app.attachment_next();
    assert_eq!(app.attachment_index, 0, "single attachment clamps");
    app.attachment_prev();
    assert_eq!(app.attachment_index, 0);

    app.close_attachments();
    assert_eq!(app.view, View::MessageView);
}

#[test]
fn attachment_actions_without_a_message_report_status() {
    use super::super::App;

    let mut app = App::new(None);
    app.open_attachments();
    assert_eq!(app.view, crate::keys::View::EnvelopeList);

    app.save_selected_attachment();
    assert!(app.status_message.contains("No attachment is selected"));
}

#[test]
fn folder_prompts_open_with_the_expected_input() {
    use crate::keys::View;

    use super::super::folders::FolderPromptKind;
    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "Archive".to_string();

    app.folder_prompt_new();
    assert_eq!(app.view, View::FolderPrompt);
    let prompt = app.folder_prompt.as_ref().expect("prompt");
    assert_eq!(prompt.kind, FolderPromptKind::Create);
    assert!(prompt.input.is_empty());

    app.folder_prompt_rename();
    let prompt = app.folder_prompt.as_ref().expect("prompt");
    assert_eq!(prompt.kind, FolderPromptKind::Rename);
    assert_eq!(prompt.input, "Archive");

    app.folder_prompt_delete();
    assert_eq!(
        app.folder_prompt.as_ref().expect("prompt").kind,
        FolderPromptKind::Delete
    );
}

#[test]
fn folder_prompt_edits_and_cancels() {
    use crate::keys::View;

    use super::super::App;

    let mut app = App::new(None);
    app.folder_prompt_new();
    for c in "Receipts".chars() {
        app.folder_prompt_input(c);
    }
    app.folder_prompt_backspace();
    assert_eq!(app.folder_prompt.as_ref().expect("prompt").input, "Receipt");

    app.cancel_folder_prompt();
    assert_eq!(app.view, View::FolderList);
    assert!(app.folder_prompt.is_none());
}

#[test]
fn deleting_a_folder_requires_confirmation() {
    use super::super::App;

    let mut app = App::new(None);
    app.current_folder = "Old".to_string();
    app.folder_prompt_delete();

    app.folder_prompt_input('n');
    app.folder_prompt_input('o');
    app.submit_folder_prompt();
    assert!(app.status_message.contains("not deleted"));
    assert!(!app.pending_folder_refresh);
}

#[test]
fn selection_toggles_and_targets_the_cursor_row() {
    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let mut app = App::new(None);
    let envelope = |id: &str| Envelope {
        id: id.to_string(),
        flags: Vec::new(),
        subject: "Subject".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    };
    app.envelopes = vec![envelope("1"), envelope("2")];
    app.envelope_state.select(Some(0));

    app.toggle_select();
    assert_eq!(app.selected.len(), 1);
    assert!(app.selected.contains("1"));
    assert_eq!(app.target_ids(), vec!["1".to_string(), "2".to_string()]);

    // Return to the first row and deselect it.
    app.envelope_state.select(Some(0));
    app.toggle_select();
    assert_eq!(app.selected.len(), 0);
    assert!(!app.selected.contains("1"));

    app.selected.insert("2".to_string());
    app.clear_selection();
    assert_eq!(app.selected.len(), 0);
}

#[test]
fn space_key_selects_the_cursor_row() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let mut app = App::new(None);
    app.envelopes = vec![Envelope {
        id: "1".to_string(),
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

    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(app.selected.contains("1"));

    app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE));
    assert!(app.selected.is_empty());
}

#[test]
fn header_toggle_switches_between_summary_and_raw_headers() {
    use super::super::App;

    let raw = b"From: alice@example.com\r\nTo: bob@example.com\r\nSubject: Raw\r\nX-Custom: value\r\n\r\nbody\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.message_content = Some(document);

    assert!(!app.show_all_headers);
    app.toggle_headers();
    assert!(app.show_all_headers);

    let headers = app.all_headers();
    assert!(headers.iter().any(|h| h == "X-Custom: value"));
    assert!(!headers.iter().any(|h| h.contains("body")));
}

#[test]
fn message_file_stem_prefers_the_message_id() {
    use super::super::App;

    let raw = b"From: alice@example.com\r\nMessage-ID: <abc.123@example.com>\r\nSubject: Hi\r\n\r\nbody\r\n";
    let mut app = App::new(None);
    app.message_content = Some(crate::mail::mime::parse_message(raw).unwrap());
    assert_eq!(app.message_file_stem(), "abc.123@example.com");

    app.message_content = None;
    app.envelopes = vec![crate::mail::types::Envelope {
        id: "42".to_string(),
        flags: Vec::new(),
        subject: "Hi".to_string(),
        sender: crate::mail::types::Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));
    assert_eq!(app.message_file_stem(), "42");
}

#[test]
fn saving_without_a_message_reports_an_error() {
    use super::super::App;

    let mut app = App::new(None);
    app.save_message();
    assert!(app.status_is_error);
    assert!(app.status_message.contains("No message source"));
}

#[test]
fn save_message_writes_the_source_and_reports_it() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::keys::View;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nMessage-ID: <save-test@example.com>\r\nSubject: Save\r\n\r\nbody\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    assert!(!app.status_is_error, "status: {}", app.status_message);
    assert!(
        app.status_message.starts_with("Saved"),
        "{}",
        app.status_message
    );

    let path = app.status_message.trim_start_matches("Saved ").to_string();
    assert!(std::path::Path::new(&path).exists(), "missing {path}");
    let _ = std::fs::remove_file(&path);
}
