/*! ManageSieve operations for the router.
Sieve is an IMAP-account capability reached through the account's credentials,
so it lives on the router rather than on a transport backend. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::sieve::{SieveClient, SieveConfig, SieveScript, SieveSecurity};

use super::router::RouterMailService;

/// Default ManageSieve port (RFC 5804).
const DEFAULT_SIEVE_PORT: u16 = 4190;

/// Host, port, and security for an account's Sieve server.
///
/// Explicit settings win; otherwise the account's IMAP host, port 4190, and
/// STARTTLS are assumed, with a plaintext IMAP account staying unencrypted.
pub(super) fn resolve_target(
    record: &crate::mail::account_store::AccountRecord,
) -> MailResult<(String, u16, SieveSecurity)> {
    let host = record
        .sieve_host
        .clone()
        .or_else(|| record.imap_host.clone())
        .ok_or_else(|| MailError::config_invalid("IMAP host is missing"))?;
    let port = record
        .sieve_port
        .or_else(|| {
            std::env::var("SOLVERFORGE_SIEVE_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(DEFAULT_SIEVE_PORT);
    let security = match record.sieve_security.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("tls") => SieveSecurity::Tls,
        Some(value) if value.eq_ignore_ascii_case("plain") => SieveSecurity::Plain,
        Some(_) => SieveSecurity::StartTls,
        // Port 4190 conventionally uses STARTTLS; only a plaintext IMAP
        // account (local test servers) keeps an unencrypted connection.
        None if record
            .imap_security
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("plain")) =>
        {
            SieveSecurity::Plain
        }
        None => SieveSecurity::StartTls,
    };
    Ok((host, port, security))
}

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
    let (host, port, security) = resolve_target(&record)?;
    let username = record
        .username
        .clone()
        .ok_or_else(|| MailError::config_invalid("account username is missing"))?;
    let secret_id = record
        .keyring_imap_secret_id
        .clone()
        .ok_or_else(|| MailError::config_invalid("IMAP secret reference is missing"))?;
    let password = router.pool.credentials().lookup(&secret_id, &username)?;

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

#[cfg(test)]
mod tests {
    use super::resolve_target;
    use crate::mail::account_store::AccountRecord;
    use crate::mail::sieve::SieveSecurity;

    fn record() -> AccountRecord {
        AccountRecord {
            name: "work".to_string(),
            backend_kind: "imap".to_string(),
            provider_kind: "generic".to_string(),
            enabled: true,
            is_default: false,
            maildir_path: None,
            imap_host: Some("imap.example.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: Some(465),
            smtp_security: Some("tls".to_string()),
            sieve_host: None,
            sieve_port: None,
            sieve_security: None,
            auth_mode: Some("password".to_string()),
            username: Some("alice@example.com".to_string()),
            keyring_imap_secret_id: Some("work-imap".to_string()),
            keyring_smtp_secret_id: Some("work-smtp".to_string()),
        }
    }

    #[test]
    fn falls_back_to_the_imap_host_and_starttls() {
        let (host, port, security) = resolve_target(&record()).unwrap();
        assert_eq!(host, "imap.example.com");
        assert_eq!(port, 4190);
        assert_eq!(security, SieveSecurity::StartTls);
    }

    #[test]
    fn explicit_sieve_settings_win() {
        let mut record = record();
        record.sieve_host = Some("sieve.example.com".to_string());
        record.sieve_port = Some(14190);
        record.sieve_security = Some("tls".to_string());

        let (host, port, security) = resolve_target(&record).unwrap();
        assert_eq!(host, "sieve.example.com");
        assert_eq!(port, 14190);
        assert_eq!(security, SieveSecurity::Tls);
    }

    #[test]
    fn plain_imap_keeps_sieve_plaintext() {
        let mut record = record();
        record.imap_security = Some("plain".to_string());
        let (_, _, security) = resolve_target(&record).unwrap();
        assert_eq!(security, SieveSecurity::Plain);
    }
}
