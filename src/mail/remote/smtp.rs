/*! SMTP transport construction (TLS, credentials, OAuth2). */

use std::time::Duration;

use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::SmtpTransport;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::oauth;
use crate::mail::session::Security;

use super::model::ImapSmtpService;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

impl ImapSmtpService {
    pub(super) fn smtp_transport(&self) -> MailResult<SmtpTransport> {
        let host = self
            .account
            .smtp_host
            .as_deref()
            .ok_or_else(|| MailError::config_invalid("SMTP host is missing"))?;
        let port = self
            .account
            .smtp_port
            .ok_or_else(|| MailError::config_invalid("SMTP port is missing"))?;
        let security = Security::normalize(self.account.smtp_security.as_deref(), "tls");
        let username = self.username()?.to_string();

        let mut builder = SmtpTransport::builder_dangerous(host)
            .port(port)
            .timeout(Some(NETWORK_TIMEOUT));

        let tls = match security {
            Security::Tls => Tls::Wrapper(
                TlsParameters::new(host.to_string())
                    .map_err(|err| MailError::tls_failure(err.to_string()))?,
            ),
            Security::StartTls => Tls::Required(
                TlsParameters::new(host.to_string())
                    .map_err(|err| MailError::tls_failure(err.to_string()))?,
            ),
            Security::Plain => Tls::None,
        };
        builder = builder.tls(tls);

        match self.account.auth_mode.as_deref().unwrap_or("password") {
            "password" | "app_password" => {
                let secret_id = self
                    .account
                    .keyring_smtp_secret_id
                    .as_deref()
                    .ok_or_else(|| MailError::config_invalid("SMTP secret reference is missing"))?;
                let secret = self.pool.credentials().lookup(secret_id, &username)?;
                builder = builder.credentials(Credentials::new(username, secret));
            }
            "oauth2" => {
                let access_token = oauth::ensure_access_token(&self.account.name, &username)?;
                builder = builder
                    .credentials(Credentials::new(username, access_token))
                    .authentication(vec![Mechanism::Xoauth2]);
            }
            other => {
                return Err(MailError::unsupported_feature(format!(
                    "unsupported auth mode: {other}"
                )));
            }
        }

        Ok(builder.build())
    }
}
