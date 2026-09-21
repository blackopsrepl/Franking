/*! ManageSieve operations for the router.
Sieve is an IMAP-account capability reached through the account's credentials,
so it lives on the router rather than on a transport backend. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::sieve::{SieveClient, SieveConfig, SieveScript, SieveSecurity};

use super::router::RouterMailService;

/// Default ManageSieve port (RFC 5804).
const DEFAULT_SIEVE_PORT: u16 = 4190;

pub(super) fn scripts(
    router: &RouterMailService,
    account: Option<&str>,
) -> MailResult<Vec<SieveScript>> {
    with_client(router, account, |client| {
        client.scripts().map_err(sieve_error)
    })
}

pub(super) fn script(
    router: &RouterMailService,
    account: Option<&str>,
    name: &str,
) -> MailResult<String> {
    with_client(router, account, |client| {
        client.get_script(name).map_err(sieve_error)
    })
}

pub(super) fn save_script(
    router: &RouterMailService,
    account: Option<&str>,
    name: &str,
    body: &str,
) -> MailResult<()> {
    with_client(router, account, |client| {
        client.put_script(name, body).map_err(sieve_error)
    })
}

pub(super) fn set_active(
    router: &RouterMailService,
    account: Option<&str>,
    name: Option<&str>,
) -> MailResult<()> {
    with_client(router, account, |client| {
        client.set_active(name).map_err(sieve_error)
    })
}

pub(super) fn delete_script(
    router: &RouterMailService,
    account: Option<&str>,
    name: &str,
) -> MailResult<()> {
    with_client(router, account, |client| {
        client.delete_script(name).map_err(sieve_error)
    })
}

/// Build the connection parameters for an account's Sieve server.
pub(super) fn config(router: &RouterMailService, account: Option<&str>) -> MailResult<SieveConfig> {
    let record = router.choose_account(account)?;
    let host = record
        .imap_host
        .clone()
        .ok_or_else(|| MailError::config_invalid("IMAP host is missing"))?;
    let username = record
        .username
        .clone()
        .ok_or_else(|| MailError::config_invalid("account username is missing"))?;
    let secret_id = record
        .keyring_imap_secret_id
        .clone()
        .ok_or_else(|| MailError::config_invalid("IMAP secret reference is missing"))?;
    let password = router.pool.credentials().lookup(&secret_id, &username)?;
    let port = std::env::var("SOLVERFORGE_SIEVE_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SIEVE_PORT);
    // Port 4190 conventionally uses STARTTLS; a plaintext IMAP account (local
    // test servers) uses an unencrypted connection.
    let security = if record
        .imap_security
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("plain"))
    {
        SieveSecurity::Plain
    } else {
        SieveSecurity::StartTls
    };

    Ok(SieveConfig {
        host,
        port,
        username,
        password,
        security,
    })
}

fn with_client<T>(
    router: &RouterMailService,
    account: Option<&str>,
    command: impl FnOnce(&mut SieveClient) -> MailResult<T>,
) -> MailResult<T> {
    let config = config(router, account)?;
    let mut client = SieveClient::connect(&config).map_err(sieve_error)?;
    let result = command(&mut client);
    client.logout();
    result
}

fn sieve_error(error: anyhow::Error) -> MailError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("connect") {
        MailError::connection_dropped(format!("Sieve: {message}"))
    } else {
        MailError::other(format!("Sieve: {message}"))
    }
}
