//! Shared helpers for the IMAP session integration tests.

use solverforge_mail::mail::account_store::AccountRecord;
use solverforge_mail::mail::session::{map_imap_error, ConnectedImapSession, CredentialProvider};
use solverforge_mail::mail::MailResult;

#[derive(Debug)]
pub(crate) struct FixedCredentials;

impl CredentialProvider for FixedCredentials {
    fn lookup(&self, _service: &str, _username: &str) -> MailResult<String> {
        Ok("secret".to_string())
    }
}

pub(crate) fn account(port: u16) -> AccountRecord {
    AccountRecord {
        name: "work".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: false,
        maildir_path: None,
        imap_host: Some("127.0.0.1".to_string()),
        imap_port: Some(port),
        imap_security: Some("plain".to_string()),
        smtp_host: None,
        smtp_port: None,
        smtp_security: None,
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("alice".to_string()),
        keyring_imap_secret_id: Some("service".to_string()),
        keyring_smtp_secret_id: None,
    }
}

pub(crate) fn select_inbox(connection: &mut ConnectedImapSession) -> MailResult<()> {
    match connection {
        ConnectedImapSession::Plain(session) => {
            session.select("INBOX").map_err(map_imap_error)?;
        }
        ConnectedImapSession::Tls(session) => {
            session.select("INBOX").map_err(map_imap_error)?;
        }
    }
    Ok(())
}
