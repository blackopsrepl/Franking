//! Shared helpers for the live app-owned-client tests.

use std::sync::{Mutex, MutexGuard, OnceLock};

use solverforge_mail::mail::account_store::AccountRecord;
use solverforge_mail::mail::remote::next;
use solverforge_mail::mail::session::{CredentialProvider, SessionPool};
use solverforge_mail::mail::MailResult;

#[derive(Debug)]
pub(crate) struct FixedCredentials;

impl CredentialProvider for FixedCredentials {
    fn lookup(&self, _service: &str, _username: &str) -> MailResult<String> {
        Ok("password".to_string())
    }
}

/// The live tests share one Dovecot INBOX, so mutations must not interleave.
pub(crate) fn mailbox_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn account(host: &str, port: u16) -> AccountRecord {
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
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("test".to_string()),
        keyring_imap_secret_id: Some("dovecot".to_string()),
        keyring_smtp_secret_id: None,
    }
}

pub(crate) fn test_address() -> Option<(String, u16)> {
    let address = std::env::var("SOLVERFORGE_IMAP_TEST_ADDR").ok()?;
    let (host, port) = address.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

/// The live tests use their own mailbox: `cargo test` runs each file in a
/// separate process, so a shared INBOX would let them mutate each other.
pub(crate) const FOLDER: &str = "codec-tests";

/// Create the test mailbox when it does not exist yet.
pub(crate) fn ensure_mailbox(account: &AccountRecord) {
    let pool = SessionPool::with_credentials(std::sync::Arc::new(FixedCredentials));
    pool.with_client(account, |client| {
        match next::create_folder(client, FOLDER) {
            Ok(()) => Ok(()),
            Err(error) if error.is_transport() => Err(error),
            // The mailbox already exists, which is the common case.
            Err(_) => Ok(()),
        }
    })
    .expect("test mailbox");
}

/// Seed two messages so a listing has something to compare.
pub(crate) fn seed(account: &AccountRecord) {
    let pool = SessionPool::with_credentials(std::sync::Arc::new(FixedCredentials));
    let raw = b"From: alice@example.com\r\nTo: test@example.com\r\nSubject: Codec probe\r\nMessage-ID: <codec-probe@example.com>\r\nDate: 2026-04-13 09:00:00+00:00\r\n\r\nhello from the codec test";
    for _ in 0..2 {
        pool.with_client(account, |client| {
            next::append(client, FOLDER, vec![], raw).map(|_| ())
        })
        .expect("seed append");
    }
}
