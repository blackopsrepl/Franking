/*! Maildir backend unit tests. */

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mail::mime;
use crate::mail::service::MailService;

use super::model::MaildirService;
use super::template::attachment_payloads;

fn temp_maildir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("solverforge-maildir-test-{unique}"))
}

#[test]
fn maildir_round_trip_supports_read_flag_move_and_send() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    let folders = service.list_folders(Some("test")).unwrap();
    assert_eq!(folders[0].name, "INBOX");

    let inbox = service
        .list_envelopes(Some("test"), "INBOX", 1, 50, None)
        .unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(!inbox[0].is_seen());

    let id = inbox[0].id.clone();
    let body = service.read_message(Some("test"), "INBOX", &id).unwrap();
    assert!(body.contains("Project update"));

    let inbox_after_read = service
        .list_envelopes(Some("test"), "INBOX", 1, 50, Some("flag seen"))
        .unwrap();
    assert_eq!(inbox_after_read.len(), 1);

    service
        .flag_add(Some("test"), "INBOX", &id, "flagged")
        .unwrap();
    let flagged = service
        .list_envelopes(Some("test"), "INBOX", 1, 50, None)
        .unwrap();
    assert!(flagged[0].is_flagged());

    service
        .move_message(Some("test"), "INBOX", "Trash", &id)
        .unwrap();
    let trash = service
        .list_envelopes(Some("test"), "Trash", 1, 50, None)
        .unwrap();
    assert_eq!(trash.len(), 1);

    let template = service.template_write(Some("test")).unwrap();
    assert_eq!(template, "\n");
    service
        .template_send(
            Some("test"),
            "To: bob@example.com\nSubject: Test send\n\nHello from SolverForge Mail",
            &Default::default(),
        )
        .unwrap();
    let sent = service
        .list_envelopes(Some("test"), "Sent", 1, 50, None)
        .unwrap();
    assert_eq!(sent.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn reply_template_carries_threading_headers() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    let raw = "From: alice@example.com\nTo: bob@example.com\nSubject: Project\nMessage-ID: <child@example.com>\nReferences: <root@example.com>\nDate: 2026-04-13 09:00:00+00:00\n\nbody";
    fs::write(root.join("new").join("threading-message"), raw).unwrap();

    let inbox = service
        .list_envelopes(Some("test"), "INBOX", 1, 50, None)
        .unwrap();
    let id = inbox
        .iter()
        .find(|envelope| envelope.subject == "Project")
        .expect("seeded message should be listed")
        .id
        .clone();

    let template = service
        .template_reply(Some("test"), "INBOX", &id, false)
        .unwrap();

    assert!(
        template.contains("In-Reply-To: <child@example.com>"),
        "{template}"
    );
    assert!(
        template.contains("References: <root@example.com> <child@example.com>"),
        "{template}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn sent_message_receives_a_message_id() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    service
        .template_send(
            Some("test"),
            "To: bob@example.com\nSubject: Hi\n\nhello",
            &Default::default(),
        )
        .unwrap();

    let sent = service
        .list_envelopes(Some("test"), "Sent", 1, 50, None)
        .unwrap();
    let raw = service
        .read_message_raw(Some("test"), "Sent", &sent[0].id)
        .unwrap();
    let raw = String::from_utf8_lossy(&raw);

    assert!(raw.contains("Message-ID: <"), "{raw}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn save_draft_writes_to_the_drafts_folder() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    service
        .save_draft(
            Some("test"),
            "To: bob@example.com\nSubject: Draft subject\n\nwork in progress",
        )
        .unwrap();

    let drafts = service
        .list_envelopes(Some("test"), "Drafts", 1, 50, None)
        .unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].subject, "Draft subject");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_unread_counts_unseen_messages() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    assert_eq!(service.folder_unread(Some("test"), "INBOX").unwrap(), 1);

    let inbox = service
        .list_envelopes(Some("test"), "INBOX", 1, 50, None)
        .unwrap();
    service
        .flag_add(Some("test"), "INBOX", &inbox[0].id, "seen")
        .unwrap();
    assert_eq!(service.folder_unread(Some("test"), "INBOX").unwrap(), 0);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn extracts_attachment_payloads_from_a_message() {
    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nbody\r\n--b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\n\r\nPDFDATA\r\n--b--";
    let document = mime::parse_message(raw).unwrap();

    let payloads = attachment_payloads(&document);
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].0, "report.pdf");
    assert_eq!(payloads[0].1, b"PDFDATA");
}

#[test]
fn sent_message_with_attachment_is_multipart() {
    let root = temp_maildir();
    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();

    let attachment = root.join("note.txt");
    fs::write(&attachment, b"hello attachment").unwrap();

    let template = format!(
        "To: bob@example.com\nSubject: With file\nAttachment: {}\n\nsee attached",
        attachment.display()
    );
    service
        .template_send(Some("test"), &template, &Default::default())
        .unwrap();

    let sent = service
        .list_envelopes(Some("test"), "Sent", 1, 50, None)
        .unwrap();
    assert_eq!(sent.len(), 1);

    let raw = service
        .read_message_raw(Some("test"), "Sent", &sent[0].id)
        .unwrap();
    let document = mime::parse_message(&raw).unwrap();
    assert_eq!(document.attachments.len(), 1);
    assert_eq!(
        document.attachments[0].file_name.as_deref(),
        Some("note.txt")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn empties_a_local_folder() {
    use crate::mail::service::MailService;

    let root = std::env::temp_dir().join(format!(
        "sfmail-empty-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let service = MaildirService::new("test", &root).with_default(true);
    service.ensure_ready().unwrap();
    service
        .template_send(
            Some("test"),
            "To: bob@example.com\nSubject: Hi\n\nhello",
            &Default::default(),
        )
        .expect("send");

    assert_eq!(
        service
            .list_envelopes(Some("test"), "Sent", 1, 50, None)
            .unwrap()
            .len(),
        1
    );
    let summary = service.empty_folder(Some("test"), "Sent").expect("empty");
    assert!(summary.contains("Emptied Sent"), "{summary}");
    assert!(service
        .list_envelopes(Some("test"), "Sent", 1, 50, None)
        .unwrap()
        .is_empty());

    let _ = std::fs::remove_dir_all(&root);
}
