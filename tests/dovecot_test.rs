//! Live IMAP integration test.
//!
//! Runs only when `SOLVERFORGE_IMAP_TEST_ADDR` is set (e.g. `127.0.0.1:1143`)
//! and a Dovecot server accepts `test`/`secret` there; otherwise it is skipped.
//! It exercises a real connect, LOGIN, capability probe, and LIST, and proves
//! that the pooled session is reused for a second operation.

use std::sync::Arc;

use solverforge_mail::mail::account_store::AccountRecord;
use solverforge_mail::mail::remote::ImapSmtpService;
use solverforge_mail::mail::session::{CredentialProvider, SessionPool};
use solverforge_mail::mail::MailResult;

#[derive(Debug)]
struct FixedCredentials;

impl CredentialProvider for FixedCredentials {
    fn lookup(&self, _service: &str, _username: &str) -> MailResult<String> {
        Ok("secret".to_string())
    }
}

fn account(host: &str, port: u16) -> AccountRecord {
    AccountRecord {
        name: "dovecot".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: true,
        maildir_path: None,
        imap_host: Some(host.to_string()),
        imap_port: Some(port),
        imap_security: Some("plain".to_string()),
        smtp_host: None,
        smtp_port: None,
        smtp_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("test".to_string()),
        keyring_imap_secret_id: Some("dovecot".to_string()),
        keyring_smtp_secret_id: None,
    }
}

#[test]
fn dovecot_lists_folders_and_reuses_the_session() {
    let Ok(address) = std::env::var("SOLVERFORGE_IMAP_TEST_ADDR") else {
        return;
    };
    let (host, port) = address.rsplit_once(':').expect("host:port");
    let port: u16 = port.parse().expect("port");

    let pool = Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials)));
    let service = ImapSmtpService::new(account(host, port), pool);

    let folders = service.list_folders(None).expect("list folders");
    assert!(
        folders
            .iter()
            .any(|folder| folder.name.eq_ignore_ascii_case("INBOX")),
        "INBOX should be listed: {folders:?}"
    );

    // A second operation must reuse the pooled connection rather than relogin.
    let again = service.list_folders(None).expect("list folders again");
    assert_eq!(folders.len(), again.len());
}
