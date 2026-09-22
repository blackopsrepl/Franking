/*! Credential resolution for IMAP sessions. */

use std::process::{Command, Stdio};

use crate::mail::errors::{MailError, MailResult};

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
