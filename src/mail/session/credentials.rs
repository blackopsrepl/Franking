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

/// The one keyring lookup every caller uses. A missing item is reported as a
/// missing secret naming what was looked for, never as an unavailable keyring.
pub fn lookup_secret(service: &str, username: &str) -> MailResult<String> {
    let output = Command::new("secret-tool")
        .args([
            "lookup",
            "service",
            service,
            "username",
            username,
            "application",
            crate::brand::KEYRING_APPLICATION,
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

    // secret-tool exits 1 with nothing on stderr when no item matches; any
    // real keyring failure explains itself on stderr.
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() && !stderr.is_empty() {
        return Err(MailError::keyring_unavailable(stderr));
    }

    let secret = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if secret.is_empty() {
        Err(MailError::secret_missing(format!(
            "no {service} secret is stored for {username}; run franking --setup"
        )))
    } else {
        Ok(secret)
    }
}
