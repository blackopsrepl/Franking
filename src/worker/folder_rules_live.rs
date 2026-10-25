/*! Real-server proof for the explicit future-mail rule boundary. */

use super::{apply_with, folder_rules};
use crate::mail::{
    account_store::AccountRecord,
    remote::{next, ImapSmtpService},
    session::{CredentialProvider, SessionPool},
    store::{set_sync_state, SyncState},
    MailResult,
};
use std::sync::Arc;

#[derive(Debug)]
struct TestCredentials;
impl CredentialProvider for TestCredentials {
    fn lookup(&self, _: &str, _: &str) -> MailResult<String> {
        Ok("password".into())
    }
}

#[test]
fn explicit_rules_move_future_mail_on_a_real_imap_server() {
    let Ok(address) = std::env::var("FRANKING_IMAP_TEST_ADDR") else {
        return;
    };
    let (host, port) = address.rsplit_once(':').expect("host:port");
    let account = AccountRecord {
        name: "rules-live".into(),
        backend_kind: "imap".into(),
        provider_kind: "generic".into(),
        enabled: true,
        is_default: true,
        maildir_path: None,
        imap_host: Some(host.into()),
        imap_port: Some(port.parse().unwrap()),
        imap_security: Some("plain".into()),
        smtp_host: None,
        smtp_port: None,
        smtp_security: None,
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("password".into()),
        username: Some("test".into()),
        keyring_imap_secret_id: Some("rules-test".into()),
        keyring_smtp_secret_id: None,
    };
    let pool = Arc::new(SessionPool::with_credentials(Arc::new(TestCredentials)));
    let service = ImapSmtpService::new(account.clone(), pool.clone());
    let destination = format!("Franking Rules {}", std::process::id());
    service.create_folder(None, &destination).unwrap();
    let sender = format!("rules-{}@example.org", std::process::id());
    let append = |subject: &str| {
        let raw = format!(
            "From: {sender}\r\nTo: test@example.org\r\nSubject: {subject}\r\n\r\nrule verification"
        );
        pool.with_client(&account, |client| {
            next::append(client, "INBOX", vec![], raw.as_bytes()).map(|_| ())
        })
        .unwrap();
    };
    append("Before explicit rule");
    let cursor = service.folder_sync_cursor(None, "INBOX").unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    set_sync_state(
        &conn,
        &SyncState {
            account: account.name.clone(),
            folder: "INBOX".into(),
            uid_validity: cursor.0,
            uid_next: cursor.1,
            ..Default::default()
        },
    )
    .unwrap();
    folder_rules::save(&conn, &account.name, &sender, &destination).unwrap();
    append("After explicit rule");
    let rows = service
        .list_envelopes(None, "INBOX", 1, 2000, None)
        .unwrap();
    let visible = apply_with(&conn, rows, |account, folder, target, uid| {
        service.move_message(Some(account), folder, target, uid)
    })
    .unwrap();
    assert!(visible.iter().any(|e| e.subject == "Before explicit rule"));
    assert!(!visible.iter().any(|e| e.subject == "After explicit rule"));
    let moved = service
        .list_envelopes(None, &destination, 1, 50, None)
        .unwrap();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].subject, "After explicit rule");
    let inbox = service
        .list_envelopes(None, "INBOX", 1, 2000, None)
        .unwrap();
    assert!(inbox.iter().any(|e| e.subject == "Before explicit rule"));
    assert!(!inbox.iter().any(|e| e.subject == "After explicit rule"));
    folder_rules::delete(&conn, &account.name, &sender).unwrap();
    append("After removing rule");
    let rows = service
        .list_envelopes(None, "INBOX", 1, 2000, None)
        .unwrap();
    let visible = apply_with(&conn, rows, |_, _, _, _| {
        panic!("deleted rule must not move mail")
    })
    .unwrap();
    assert!(visible.iter().any(|e| e.subject == "After removing rule"));
    service.delete_folder(None, &destination).unwrap();
}
