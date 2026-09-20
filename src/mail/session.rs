/*! IMAP session ownership: connection establishment, capability probing, and
IDLE support.
One place owns how a connection is opened (TLS / STARTTLS / plain), how the
account authenticates (password, app password, or XOAUTH2), and what the
server advertises. Callers that need a connection reuse this instead of
reimplementing the handshake, so capability-aware behavior can be added in one
spot. */

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use imap::extensions::idle::{SetReadTimeout, WaitOutcome};
use imap::Authenticator;
use native_tls::{TlsConnector, TlsStream};

use super::account_store::AccountRecord;
use super::errors::{MailError, MailResult};
use super::oauth;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

/// Resolves credentials for an account. The production implementation reads
/// the OS keyring; tests inject a fixed provider so sessions can be exercised
/// without a keyring or secret store.
pub trait CredentialProvider: std::fmt::Debug + Send + Sync {
    fn lookup(&self, service: &str, username: &str) -> MailResult<String>;
}

#[derive(Debug, Default)]
pub struct KeyringCredentials;

impl CredentialProvider for KeyringCredentials {
    fn lookup(&self, service: &str, username: &str) -> MailResult<String> {
        lookup_secret(service, username)
    }
}

/// Capabilities advertised by the server, decoded into the flags this client
/// actually branches on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub names: Vec<String>,
    pub imap4rev1: bool,
    pub idle: bool,
    pub move_: bool,
    pub uidplus: bool,
    pub qresync: bool,
    pub condstore: bool,
    pub special_use: bool,
    pub sort: bool,
    pub thread: bool,
    pub utf8_accept: bool,
    pub compress_deflate: bool,
    pub auth_plain: bool,
    pub auth_xoauth2: bool,
}

impl Capabilities {
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names = names
            .into_iter()
            .map(|name| name.as_ref().to_string())
            .collect::<Vec<_>>();
        let has = |capability: &str| {
            names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(capability))
        };
        Self {
            imap4rev1: has("IMAP4rev1"),
            idle: has("IDLE"),
            move_: has("MOVE"),
            uidplus: has("UIDPLUS"),
            qresync: has("QRESYNC"),
            condstore: has("CONDSTORE"),
            special_use: has("SPECIAL-USE"),
            sort: has("SORT"),
            thread: has("THREAD"),
            utf8_accept: has("UTF8=ACCEPT"),
            compress_deflate: has("COMPRESS=DEFLATE"),
            auth_plain: has("AUTH=PLAIN"),
            auth_xoauth2: has("AUTH=XOAUTH2"),
            names,
        }
    }

    pub fn from_imap(capabilities: &imap::types::Capabilities) -> Self {
        let names = capabilities
            .iter()
            .map(|capability| match capability {
                imap_proto::types::Capability::Imap4rev1 => "IMAP4rev1".to_string(),
                imap_proto::types::Capability::Auth(mechanism) => format!("AUTH={mechanism}"),
                imap_proto::types::Capability::Atom(name) => name.to_string(),
            })
            .collect::<Vec<_>>();
        Self::from_names(names)
    }

    pub fn supports(&self, capability: &str) -> bool {
        self.names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(capability))
    }
}

/// Result of an IDLE wait.
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

fn idle_wait(connection: &mut ConnectedImapSession, timeout: Duration) -> MailResult<IdleOutcome> {
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
#[derive(Debug)]
pub struct SessionPool {
    credentials: Arc<dyn CredentialProvider>,
    sessions: Mutex<HashMap<String, ImapSession>>,
    watchers: Mutex<HashMap<String, ImapSession>>,
}

impl Default for SessionPool {
    fn default() -> Self {
        Self::with_credentials(Arc::new(KeyringCredentials))
    }
}

impl SessionPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_credentials(credentials: Arc<dyn CredentialProvider>) -> Self {
        Self {
            credentials,
            sessions: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn credentials(&self) -> &Arc<dyn CredentialProvider> {
        &self.credentials
    }

    pub fn with_connection<T>(
        &self,
        account: &AccountRecord,
        mut operation: impl FnMut(&mut ConnectedImapSession) -> MailResult<T>,
    ) -> MailResult<T> {
        let mut attempts = 0;
        loop {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| MailError::other("session pool lock was poisoned"))?;

            if !sessions.contains_key(&account.name) {
                sessions.insert(
                    account.name.clone(),
                    ImapSession::connect(account, self.credentials.as_ref())?,
                );
            }

            let session = sessions
                .get_mut(&account.name)
                .expect("session inserted immediately above");

            match operation(session.connection()) {
                Ok(value) => return Ok(value),
                Err(error) if error.is_transport() => {
                    sessions.remove(&account.name);
                    drop(sessions);
                    if attempts == 0 {
                        attempts += 1;
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub fn idle_wait(
        &self,
        account: &AccountRecord,
        folder: &str,
        timeout: Duration,
    ) -> MailResult<IdleOutcome> {
        // A dedicated connection is used for IDLE so a long wait never blocks
        // the operation sessions for the same account.
        let mut watchers = self
            .watchers
            .lock()
            .map_err(|_| MailError::other("session pool lock was poisoned"))?;

        if !watchers.contains_key(&account.name) {
            watchers.insert(
                account.name.clone(),
                ImapSession::connect(account, self.credentials.as_ref())?,
            );
        }
        let session = watchers
            .get_mut(&account.name)
            .expect("session inserted immediately above");

        let result = (|| {
            select_mailbox(session.connection(), folder)?;
            idle_wait(session.connection(), timeout)
        })();

        if let Err(error) = &result {
            if error.is_transport() {
                watchers.remove(&account.name);
            }
        }

        result
    }

    pub fn invalidate(&self, account: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(account);
        }
    }
}

fn select_mailbox(connection: &mut ConnectedImapSession, folder: &str) -> MailResult<()> {
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

pub fn connect_tcp(host: &str, port: u16) -> MailResult<TcpStream> {
    let stream = TcpStream::connect((host, port)).map_err(|err| {
        if err.kind() == std::io::ErrorKind::TimedOut {
            MailError::transport_timeout(err.to_string())
        } else {
            MailError::io(err.to_string())
        }
    })?;
    stream
        .set_read_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|err| MailError::io(err.to_string()))?;
    stream
        .set_write_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|err| MailError::io(err.to_string()))?;
    Ok(stream)
}

fn tls_connector() -> MailResult<TlsConnector> {
    TlsConnector::builder()
        .build()
        .map_err(|err| MailError::tls_failure(err.to_string()))
}

/// Look up a secret from the OS keyring via `secret-tool`.
pub fn lookup_secret(service: &str, username: &str) -> MailResult<String> {
    let output = Command::new("secret-tool")
        .args([
            "lookup",
            "service",
            service,
            "username",
            username,
            "application",
            "solverforge-mail",
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                MailError::keyring_unavailable(
                    "secret-tool is not installed or not available in PATH",
                )
            } else {
                MailError::keyring_unavailable(err.to_string())
            }
        })?;

    if !output.status.success() {
        return Err(MailError::keyring_unavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let secret = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if secret.is_empty() {
        Err(MailError::secret_missing(format!(
            "no secret found for service {service}"
        )))
    } else {
        Ok(secret)
    }
}

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

#[cfg(test)]
mod tests {
    use super::{Capabilities, Security};

    #[test]
    fn capabilities_map_known_extensions() {
        let capabilities = Capabilities::from_names([
            "IMAP4rev1",
            "IDLE",
            "MOVE",
            "UIDPLUS",
            "CONDSTORE",
            "SPECIAL-USE",
            "SORT",
            "THREAD",
            "UTF8=ACCEPT",
            "COMPRESS=DEFLATE",
            "AUTH=PLAIN",
            "AUTH=XOAUTH2",
        ]);

        assert!(capabilities.imap4rev1);
        assert!(capabilities.idle);
        assert!(capabilities.move_);
        assert!(capabilities.uidplus);
        assert!(capabilities.condstore);
        assert!(capabilities.special_use);
        assert!(capabilities.sort);
        assert!(capabilities.thread);
        assert!(capabilities.utf8_accept);
        assert!(capabilities.compress_deflate);
        assert!(capabilities.auth_plain);
        assert!(capabilities.auth_xoauth2);
        assert!(!capabilities.qresync);
        assert!(capabilities.supports("idle"));
    }

    #[test]
    fn security_normalizes_aliases() {
        assert_eq!(Security::normalize(Some("imaps"), "plain"), Security::Tls);
        assert_eq!(Security::normalize(Some("ssl"), "plain"), Security::Tls);
        assert_eq!(
            Security::normalize(Some("starttls"), "tls"),
            Security::StartTls
        );
        assert_eq!(Security::normalize(Some("none"), "tls"), Security::Plain);
        assert_eq!(Security::normalize(None, "tls"), Security::Tls);
    }
}
