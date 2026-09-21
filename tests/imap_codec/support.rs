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

/// Create the test mailbox when it does not exist, then wait until the server
/// reports it.
///
/// Dovecot answers existence from a per-user mailbox index, so a CREATE from
/// one connection is not always visible to the next connection immediately;
/// without this check a following APPEND fails with TRYCREATE on a fresh
/// container.
pub(crate) fn ensure_mailbox(account: &AccountRecord) {
    use std::time::{Duration, Instant};

    let pool = SessionPool::with_credentials(std::sync::Arc::new(FixedCredentials));
    pool.with_client(account, |client| {
        next::select(client, "INBOX")?;
        // Whichever reason CREATE reports, the folder must end up listed.
        let created = next::create_folder(client, FOLDER).err();

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let listed = next::list_folders(client)?;
            if listed.iter().any(|folder| folder.name == FOLDER) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(solverforge_mail::mail::MailError::other(format!(
                    "the test mailbox {FOLDER} was not created: {created:?}"
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })
    .expect("test mailbox");
}

/// Seed two messages so a listing has something to compare.
pub(crate) fn seed(account: &AccountRecord) {
    let pool = SessionPool::with_credentials(std::sync::Arc::new(FixedCredentials));
    let raw = b"From: alice@example.com\r\nTo: test@example.com\r\nSubject: Codec probe\r\nMessage-ID: <codec-probe@example.com>\r\nDate: 2026-04-13 09:00:00+00:00\r\n\r\nhello from the codec test";
    // Both appends go through one connection, which also ensures the mailbox
    // exists on it: Dovecot's per-user mailbox index lags between sessions, so
    // a mailbox created elsewhere is not always appendable from here yet.
    pool.with_client(account, |client| {
        let _ = next::create_folder(client, FOLDER);
        for _ in 0..2 {
            next::append(client, FOLDER, vec![], raw)?;
        }
        Ok(())
    })
    .expect("seed append");
}
