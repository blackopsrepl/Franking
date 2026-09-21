/*! Connect and authenticate the app-owned IMAP client. */

use crate::mail::account_store::AccountRecord;
use crate::mail::errors::{MailError, MailResult};
use crate::mail::oauth;

use super::client::ImapClient;
use super::credentials::CredentialProvider;
use super::transport::{connect_tcp, tls_connector, ReadWrite};
use super::Security;

/// Open and authenticate an app-owned IMAP client.
///
/// Mirrors [`connect`] for transport setup, but returns the codec-based client
/// whose reading policy tolerates untagged responses the grammar does not know.
pub fn open_imap_client(
    account: &AccountRecord,
    credentials: &dyn CredentialProvider,
) -> MailResult<ImapClient<Box<dyn ReadWrite>>> {
    let host = account
        .imap_host
        .as_deref()
        .ok_or_else(|| MailError::config_invalid("IMAP host is missing"))?;
    let port = account
        .imap_port
        .ok_or_else(|| MailError::config_invalid("IMAP port is missing"))?;
    let security = Security::normalize(account.imap_security.as_deref(), "tls");
    let username = account
        .username
        .as_deref()
        .ok_or_else(|| MailError::config_invalid("account username is missing"))?
        .to_string();

    let secret = match account.auth_mode.as_deref().unwrap_or("password") {
        "password" | "app_password" => {
            let secret_id = account
                .keyring_imap_secret_id
                .as_deref()
                .ok_or_else(|| MailError::config_invalid("IMAP secret reference is missing"))?;
            Some(credentials.lookup(secret_id, &username)?)
        }
        "oauth2" => None,
        other => {
            return Err(MailError::unsupported_feature(format!(
                "unsupported auth mode: {other}"
            )))
        }
    };

    let mut client: ImapClient<Box<dyn ReadWrite>> = match security {
        Security::Tls => {
            let stream = connect_tcp(host, port)?;
            let stream = tls_connector()?
                .connect(host, stream)
                .map_err(|err| MailError::tls_failure(err.to_string()))?;
            ImapClient::new(Box::new(stream) as Box<dyn ReadWrite>)
        }
        Security::StartTls => {
            let mut client = ImapClient::new(connect_tcp(host, port)?);
            client
                .starttls()
                .map_err(|err| MailError::tls_failure(err.to_string()))?;
            let stream = client.into_inner();
            let stream = tls_connector()?
                .connect(host, stream)
                .map_err(|err| MailError::tls_failure(err.to_string()))?;
            ImapClient::new(Box::new(stream) as Box<dyn ReadWrite>)
        }
        Security::Plain => {
            ImapClient::new(Box::new(connect_tcp(host, port)?) as Box<dyn ReadWrite>)
        }
    };

    match secret {
        Some(secret) => client
            .login(&username, &secret)
            .map_err(|err| MailError::imap_auth_rejected(err.to_string()))?,
        None => {
            let access_token = oauth::ensure_access_token(&account.name, &username)?;
            client
                .authenticate_xoauth2(&username, &access_token)
                .map_err(|err| MailError::imap_auth_rejected(err.to_string()))?;
        }
    }

    Ok(client)
}
