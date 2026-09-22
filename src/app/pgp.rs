/*! PGP verification and decryption for a loaded message. */

use crate::mail::model::{BodyDocument, MessageDocument};
use crate::mail::pgp::{self, InlinePgp};
use crate::mail::security::Protection;

/// Verify or decrypt PGP on a message, returning a status line.
pub(super) fn process_pgp(message: &mut MessageDocument, passphrase: &str) -> Option<String> {
    let keyring = pgp::Keyring::load(&keys_dir());

    if message.protection() == Some(Protection::PgpSigned) {
        if let Some(raw) = message.raw.as_deref() {
            let fingerprints = pgp::verify_mime(raw, &keyring.public).unwrap_or_default();
            return Some(if fingerprints.is_empty() {
                "PGP/MIME signature could not be verified".to_string()
            } else {
                format!("PGP/MIME signature valid: {}", fingerprints.join(", "))
            });
        }
    }

    if message.protection() == Some(Protection::PgpEncrypted) {
        if let Some(raw) = message.raw.as_deref() {
            return Some(match pgp::decrypt_mime(raw, &keyring.secret, passphrase) {
                Some(data) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    message.body = BodyDocument::from_plain(&text);
                    message.plain_body = Some(text);
                    "PGP/MIME encrypted message decrypted".to_string()
                }
                None => "PGP/MIME encrypted (no usable secret key or wrong passphrase)".to_string(),
            });
        }
    }

    let body = message.plain_body.clone()?;
    let kind = pgp::detect_inline(&body)?;

    match kind {
        InlinePgp::Signed => {
            let fingerprints = pgp::verify_cleartext(&body, &keyring.public);
            Some(if fingerprints.is_empty() {
                "PGP signature could not be verified".to_string()
            } else {
                format!("PGP signature valid: {}", fingerprints.join(", "))
            })
        }
        InlinePgp::Encrypted => match pgp::decrypt_inline(&body, &keyring.secret, passphrase) {
            Some(data) => {
                let text = String::from_utf8_lossy(&data).to_string();
                message.body = BodyDocument::from_plain(&text);
                message.plain_body = Some(text);
                Some("PGP encrypted message decrypted".to_string())
            }
            None => Some("PGP encrypted (no usable secret key or wrong passphrase)".to_string()),
        },
    }
}

pub(crate) fn keys_dir() -> std::path::PathBuf {
    // One definition of where key material lives, shared with the send path.
    pgp::default_keys_dir()
}
