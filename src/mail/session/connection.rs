/*! Connection establishment, authentication, IDLE, and error mapping. */

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use imap::extensions::idle::{SetReadTimeout, WaitOutcome};
use imap::Authenticator;
use native_tls::TlsStream;

use crate::mail::account_store::AccountRecord;
use crate::mail::errors::{MailError, MailResult};
use crate::mail::oauth;

use super::capabilities::Capabilities;
use super::credentials::CredentialProvider;
use super::transport::{connect_tcp, tls_connector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleOutcome {
    TimedOut,
    MailboxChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    Tls,
    StartTls,
    Plain,
}

impl Security {
    pub fn normalize(value: Option<&str>, default: &str) -> Self {
        match value
            .unwrap_or(default)
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "starttls" => Security::StartTls,
            "plain" | "none" => Security::Plain,
            _ => Security::Tls,
        }
    }
}

/// A live, authenticated IMAP connection. The stream type differs between
/// implicit TLS and plaintext/STARTTLS, so the variants carry the session.
#[derive(Debug)]
pub enum ConnectedImapSession {
    Plain(imap::Session<TcpStream>),
    Tls(imap::Session<TlsStream<TcpStream>>),
}

/// A connection plus the capabilities probed at connect time.
#[derive(Debug)]
pub struct ImapSession {
    pub connection: ConnectedImapSession,
    pub capabilities: Capabilities,
}

impl ImapSession {
    /// Open and authenticate a session, probing capabilities once.
    pub fn connect(
        account: &AccountRecord,
        credentials: &dyn CredentialProvider,
    ) -> MailResult<Self> {
        let mut connection = connect(account, credentials)?;
        let capabilities = probe_capabilities(&mut connection)?;
        Ok(Self {
            connection,
            capabilities,
        })
    }

    pub fn connection(&mut self) -> &mut ConnectedImapSession {
        &mut self.connection
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Block until the selected mailbox changes or the timeout expires.
    /// Callers must have a mailbox selected; IDLE only reports changes there.
    pub fn idle_wait(&mut self, timeout: Duration) -> MailResult<IdleOutcome> {
        idle_wait(&mut self.connection, timeout)
    }
}

/// Open and authenticate a connection without probing capabilities.
pub fn connect(
    account: &AccountRecord,
    credentials: &dyn CredentialProvider,
) -> MailResult<ConnectedImapSession> {
    let host = account
        .imap_host
        .as_deref()
        .ok_or_else(|| MailError::config_invalid("IMAP host is missing"))?;
    let port = account
        .imap_port
        .ok_or_else(|| MailError::config_invalid("IMAP port is missing"))?;
    let security = Security::normalize(account.imap_security.as_deref(), "tls");

    match security {
        Security::Tls => {
            let connector = tls_connector()?;
            let stream = connect_tcp(host, port)?;
            let stream = connector
                .connect(host, stream)
                .map_err(|err| MailError::tls_failure(err.to_string()))?;
            let mut client = imap::Client::new(stream);
            client.read_greeting().map_err(map_imap_error)?;
            login_client(client, account, credentials).map(ConnectedImapSession::Tls)
        }
        Security::StartTls => {
            let connector = tls_connector()?;
            let stream = connect_tcp(host, port)?;
            let mut client = imap::Client::new(stream);
            client.read_greeting().map_err(map_imap_error)?;
            let client = client.secure(host, &connector).map_err(map_imap_error)?;
            login_client(client, account, credentials).map(ConnectedImapSession::Tls)
        }
        Security::Plain => {
            let stream = connect_tcp(host, port)?;
            let mut client = imap::Client::new(stream);
            client.read_greeting().map_err(map_imap_error)?;
            login_client(client, account, credentials).map(ConnectedImapSession::Plain)
        }
    }
}

pub fn probe_capabilities(connection: &mut ConnectedImapSession) -> MailResult<Capabilities> {
    match connection {
        ConnectedImapSession::Plain(session) => {
            let capabilities = session.capabilities().map_err(map_imap_error)?;
            Ok(Capabilities::from_imap(&capabilities))
        }
        ConnectedImapSession::Tls(session) => {
            let capabilities = session.capabilities().map_err(map_imap_error)?;
            Ok(Capabilities::from_imap(&capabilities))
        }
    }
}

pub(super) fn idle_wait(
    connection: &mut ConnectedImapSession,
    timeout: Duration,
) -> MailResult<IdleOutcome> {
    match connection {
        ConnectedImapSession::Plain(session) => idle_on(session, timeout),
        ConnectedImapSession::Tls(session) => idle_on(session, timeout),
    }
}

fn idle_on<S>(session: &mut imap::Session<S>, timeout: Duration) -> MailResult<IdleOutcome>
where
    S: Read + Write + SetReadTimeout,
{
    let handle = session.idle().map_err(map_imap_error)?;
    match handle.wait_with_timeout(timeout).map_err(map_imap_error)? {
        WaitOutcome::TimedOut => Ok(IdleOutcome::TimedOut),
        WaitOutcome::MailboxChanged => Ok(IdleOutcome::MailboxChanged),
    }
}

/// A pool of authenticated sessions, one per account, reused across operations.
/// A transport failure discards the entry and retries the operation once on a
/// fresh connection.
pub(super) fn select_mailbox(
    connection: &mut ConnectedImapSession,
    folder: &str,
) -> MailResult<()> {
    match connection {
        ConnectedImapSession::Plain(session) => {
            session.select(folder).map_err(map_imap_error)?;
        }
        ConnectedImapSession::Tls(session) => {
            session.select(folder).map_err(map_imap_error)?;
        }
    }
    Ok(())
}

fn login_client<S: Read + Write>(
    client: imap::Client<S>,
    account: &AccountRecord,
    credentials: &dyn CredentialProvider,
) -> MailResult<imap::Session<S>> {
    let username = account
        .username
        .as_deref()
        .ok_or_else(|| MailError::config_invalid("account username is missing"))?
        .to_string();

    match account.auth_mode.as_deref().unwrap_or("password") {
        "password" | "app_password" => {
            let secret_id = account
                .keyring_imap_secret_id
                .as_deref()
                .ok_or_else(|| MailError::config_invalid("IMAP secret reference is missing"))?;
            let secret = credentials.lookup(secret_id, &username)?;
            client
                .login(username, secret)
                .map_err(|(err, _)| map_imap_error(err))
        }
        "oauth2" => {
            let access_token = oauth::ensure_access_token(&account.name, &username)?;
            let authenticator = XOAuth2Authenticator {
                username,
                access_token,
            };
            client
                .authenticate("XOAUTH2", &authenticator)
                .map_err(|(err, _)| map_imap_error(err))
        }
        other => Err(MailError::unsupported_feature(format!(
            "unsupported auth mode: {other}"
        ))),
    }
}

#[derive(Debug)]
struct XOAuth2Authenticator {
    username: String,
    access_token: String,
}

impl Authenticator for XOAuth2Authenticator {
    type Response = String;

    fn process(&self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.username, self.access_token
        )
    }
}

/// Look up a secret from the OS keyring via `secret-tool`.
pub fn map_imap_error(error: imap::error::Error) -> MailError {
    match error {
        imap::error::Error::No(detail) | imap::error::Error::Bad(detail)
            if looks_like_auth_failure(&detail) =>
        {
            MailError::imap_auth_rejected(detail)
        }
        imap::error::Error::No(detail) | imap::error::Error::Bad(detail) => {
            MailError::other(detail)
        }
        imap::error::Error::Tls(err) => MailError::tls_failure(err.to_string()),
        imap::error::Error::TlsHandshake(err) => MailError::tls_failure(err.to_string()),
        imap::error::Error::ConnectionLost => {
            MailError::connection_dropped("the IMAP server closed the connection")
        }
        imap::error::Error::Io(err) if err.kind() == std::io::ErrorKind::TimedOut => {
            MailError::transport_timeout(err.to_string())
        }
        imap::error::Error::Io(err) => MailError::io(err.to_string()),
        other => MailError::other(other.to_string()),
    }
}

pub fn looks_like_auth_failure(detail: &str) -> bool {
    let lowered = detail.to_ascii_lowercase();
    lowered.contains("auth")
        || lowered.contains("login failed")
        || lowered.contains("invalid credentials")
        || lowered.contains("username and password")
        || lowered.contains("535")
}
