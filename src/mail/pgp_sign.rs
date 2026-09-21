/*! OpenPGP signing and encryption for outbound messages.
Builds the two artifacts PGP/MIME needs: an armored detached signature over
the signed part, and an armored encrypted message for the recipients. */

use anyhow::{Context, Result};
use pgp::composed::{DetachedSignature, MessageBuilder, SignedPublicKey, SignedSecretKey};
use pgp::crypto::hash::HashAlgorithm;
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::{KeyDetails, Password};
use rand::thread_rng;

/// Armored detached signature (`SignatureType::Binary`) over `content`.
pub fn sign_detached(content: &[u8], key: &SignedSecretKey, passphrase: &str) -> Result<String> {
    let password: Password = passphrase.into();
    let signature = match signing_subkey(key) {
        Some(subkey) => DetachedSignature::sign_binary_data(
            thread_rng(),
            &subkey.key,
            &password,
            HashAlgorithm::Sha256,
            content,
        ),
        None => DetachedSignature::sign_binary_data(
            thread_rng(),
            &key.primary_key,
            &password,
            HashAlgorithm::Sha256,
            content,
        ),
    }
    .context("sign detached data")?;
    signature
        .to_armored_string(Default::default())
        .context("armor signature")
}

/// Encryption-capable subkey, if the certificate carries one.
fn encryption_subkey(key: &SignedPublicKey) -> Option<&pgp::composed::SignedPublicSubKey> {
    key.public_subkeys
        .iter()
        .find(|subkey| subkey.key.algorithm().can_encrypt())
}

/// Signing-capable subkey, if the certificate carries one.
fn signing_subkey(key: &SignedSecretKey) -> Option<&pgp::composed::SignedSecretSubKey> {
    key.secret_subkeys
        .iter()
        .find(|subkey| subkey.key.algorithm().can_sign())
}

/// Armored OpenPGP message encrypting `content` to every supplied public key.
pub fn encrypt_to_keys(content: &[u8], keys: &[SignedPublicKey]) -> Result<String> {
    let mut rng = thread_rng();
    let mut builder = MessageBuilder::from_bytes("", content.to_vec())
        .seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
    for key in keys {
        match encryption_subkey(key) {
            Some(subkey) => builder.encrypt_to_key(&mut rng, subkey),
            None => builder.encrypt_to_key(&mut rng, key),
        }
        .context("encrypt to recipient key")?;
    }
    builder
        .to_armored_string(&mut rng, Default::default())
        .context("armor encrypted message")
}

/// Public keys whose user IDs mention any of the given email addresses.
///
/// Matching is a case-insensitive substring test against the full user ID, so
/// `Alice <alice@example.com>` matches the address `alice@example.com`.
pub fn recipient_keys(keys: &[SignedPublicKey], emails: &[String]) -> Vec<SignedPublicKey> {
    let wanted: Vec<String> = emails
        .iter()
        .map(|email| email.trim().to_lowercase())
        .filter(|email| !email.is_empty())
        .collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    keys.iter()
        .filter(|key| key_matches(key, &wanted))
        .cloned()
        .collect()
}

/// Email addresses and names carried in a key's user IDs.
pub fn key_emails(key: &SignedPublicKey) -> Vec<String> {
    key.details
        .users
        .iter()
        .map(|user| String::from_utf8_lossy(user.id.id()).to_string())
        .collect()
}

fn key_matches(key: &SignedPublicKey, wanted: &[String]) -> bool {
    key.details.users.iter().any(|user| {
        let id = String::from_utf8_lossy(user.id.id()).to_lowercase();
        wanted.iter().any(|email| id.contains(email.as_str()))
    })
}

#[cfg(test)]
mod tests;
