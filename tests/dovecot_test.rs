//! Live IMAP integration test.
//!
//! Runs only when `SOLVERFORGE_IMAP_TEST_ADDR` is set, pointing at a Dovecot
//! test container started per the official docs (rootless image, non-privileged
//! port, password via env):
//!
//! ```text
//! printf 'auth_allow_cleartext = yes\n' > 99-test.conf
//! podman run -d --name sfm-dovecot -p 1143:31143 -e USER_PASSWORD=password \
//!   -v $PWD/99-test.conf:/etc/dovecot/conf.d/99-test.conf:Z dovecot/dovecot:latest
//! SOLVERFORGE_IMAP_TEST_ADDR=127.0.0.1:1143 cargo test --test dovecot_test
//! ```
//!
//! Any username authenticates with that password; the drop-in allows cleartext
//! auth for the non-TLS test port. Skipped otherwise.

use std::sync::Arc;

use solverforge_mail::mail::account_store::AccountRecord;
use solverforge_mail::mail::mime;
use solverforge_mail::mail::remote::ImapSmtpService;
use solverforge_mail::mail::session::{
    map_imap_error, ConnectedImapSession, CredentialProvider, SessionPool,
};
use solverforge_mail::mail::MailResult;

#[derive(Debug)]
struct FixedCredentials;

impl CredentialProvider for FixedCredentials {
    fn lookup(&self, _service: &str, _username: &str) -> MailResult<String> {
        Ok("password".to_string())
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
fn dovecot_append_list_read_and_flag() {
    let Ok(address) = std::env::var("SOLVERFORGE_IMAP_TEST_ADDR") else {
        return;
    };
    let (host, port) = address.rsplit_once(':').expect("host:port");
    let port: u16 = port.parse().expect("port");
    let account = account(host, port);

    let pool = Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials)));
    let service = ImapSmtpService::new(account.clone(), pool.clone());

    let raw = b"From: alice@example.com\r\nTo: test@example.com\r\nSubject: Dovecot probe\r\nMessage-ID: <probe@example.com>\r\nDate: 2026-04-13 09:00:00+00:00\r\n\r\nhello from dovecot";
    pool.with_connection(&account, |connection| match connection {
        ConnectedImapSession::Plain(session) => session
            .append("INBOX", raw.as_slice())
            .map_err(map_imap_error),
        ConnectedImapSession::Tls(session) => session
            .append("INBOX", raw.as_slice())
            .map_err(map_imap_error),
    })
    .expect("append");

    let envelopes = service
        .list_envelopes(None, "INBOX", 1, 50, None)
        .expect("list");
    let probe = envelopes
        .iter()
        .find(|envelope| envelope.subject == "Dovecot probe")
        .expect("appended message should be listed");

    let (uid_validity, uid_next) = service
        .folder_sync_cursor(None, "INBOX")
        .expect("sync cursor");
    assert!(uid_validity.is_some(), "UIDVALIDITY should be reported");
    assert!(uid_next.is_some(), "UIDNEXT should be reported");

    let bytes = service
        .read_message_raw(None, "INBOX", &probe.id)
        .expect("read");
    let document = mime::parse_message(&bytes).expect("parse");
    assert_eq!(document.subject(), "Dovecot probe");

    let cached = service.sync_folder(None, "INBOX").expect("sync folder");
    assert!(!cached.is_empty(), "sync_folder should return envelopes");

    let resume = service
        .draft_template(None, "INBOX", &probe.id)
        .expect("draft template");
    assert!(resume.contains("Subject: Dovecot probe"));
    assert!(resume.contains("To: test@example.com"));

    service
        .flag_add(None, "INBOX", &probe.id, "seen")
        .expect("flag");
}

#[test]
fn dovecot_threads_messages() {
    let Ok(address) = std::env::var("SOLVERFORGE_IMAP_TEST_ADDR") else {
        return;
    };
    let (host, port) = address.rsplit_once(':').expect("host:port");
    let port: u16 = port.parse().expect("port");
    let account = account(host, port);

    let pool = Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials)));
    let service = ImapSmtpService::new(account.clone(), pool.clone());

    let parent = b"From: alice@example.com\r\nTo: test@example.com\r\nSubject: Thread root\r\nMessage-ID: <root@example.com>\r\n\r\nroot";
    let child = b"From: bob@example.com\r\nTo: test@example.com\r\nSubject: Re: Thread root\r\nMessage-ID: <child@example.com>\r\nReferences: <root@example.com>\r\nIn-Reply-To: <root@example.com>\r\n\r\nreply";
    for raw in [parent.as_slice(), child.as_slice()] {
        pool.with_connection(&account, |connection| match connection {
            ConnectedImapSession::Plain(session) => {
                session.append("INBOX", raw).map_err(map_imap_error)
            }
            ConnectedImapSession::Tls(session) => {
                session.append("INBOX", raw).map_err(map_imap_error)
            }
        })
        .expect("append");
    }

    let threaded = service
        .list_envelopes_threaded(None, "INBOX", None)
        .expect("threaded list");
    assert!(threaded
        .iter()
        .any(|envelope| envelope.subject == "Thread root"));
    assert!(threaded
        .iter()
        .any(|envelope| envelope.subject == "Re: Thread root"));
}
