/*! OpenPGP verification and decryption.
Uses rpgp for cleartext-signed and inline-encrypted messages, and for detached
signatures. Keyrings are loaded from armored/binary key files. */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mail_parser::{MessageParser, MimeHeaders, PartType};
use pgp::composed::{Deserializable, Message, SignedPublicKey, SignedSecretKey};
use pgp::composed::{EncryptionCaps, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder};
use pgp::crypto::ecc_curve::ECCCurve;
use pgp::types::Password;
use rand::thread_rng;

pub mod keys;
pub mod verify;
pub use keys::{delete_key, export_public_key, import_public_key, list_keys, KeyInfo};
pub use verify::{verify_cleartext, verify_detached, verify_mime};

/// Inline OpenPGP found in a message body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlinePgp {
    Signed,
    Encrypted,
}

/// Detect an inline armored OpenPGP block in a body.
pub fn detect_inline(body: &str) -> Option<InlinePgp> {
    if body.contains("-----BEGIN PGP SIGNED MESSAGE-----") {
        Some(InlinePgp::Signed)
    } else if body.contains("-----BEGIN PGP MESSAGE-----") {
        Some(InlinePgp::Encrypted)
    } else {
        None
    }
}

/// Decrypt a PGP/MIME (multipart/encrypted) message with a secret key.
pub fn decrypt_mime(raw: &[u8], keys: &[SignedSecretKey], passphrase: &str) -> Option<Vec<u8>> {
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    let message = parser.parse(raw)?;
    let root = message.part(0)?;
    let content_type = root.content_type()?;
    if !content_type.c_type.eq_ignore_ascii_case("multipart")
        || !content_type
            .c_subtype
            .as_deref()
            .is_some_and(|subtype| subtype.eq_ignore_ascii_case("encrypted"))
    {
        return None;
    }
    let PartType::Multipart(ids) = &root.body else {
        return None;
    };
    if ids.len() < 2 {
        return None;
    }
    let encrypted_part = message.part(ids[1])?;
    let ciphertext = encrypted_part.contents();
    let password: Password = passphrase.into();

    for key in keys {
        let message = parse_pgp_message(ciphertext)?;
        let Ok(mut decrypted) = message.decrypt(&password, key) else {
            continue;
        };
        if decrypted.is_compressed() {
            match decrypted.decompress() {
                Ok(decompressed) => decrypted = decompressed,
                Err(_) => continue,
            }
        }
        if let Ok(data) = decrypted.as_data_vec() {
            return Some(data);
        }
    }
    None
}

/// Parse an OpenPGP message that may be armored or binary.
fn parse_pgp_message(bytes: &[u8]) -> Option<Message<'_>> {
    if let Ok((message, _)) = Message::from_armor(bytes) {
        return Some(message);
    }
    Message::from_bytes(std::io::Cursor::new(bytes)).ok()
}

/// Decrypt an inline armored OpenPGP message with the first usable secret key.
pub fn decrypt_inline(
    armored: &str,
    keys: &[SignedSecretKey],
    passphrase: &str,
) -> Option<Vec<u8>> {
    let password: Password = passphrase.into();
    for key in keys {
        let Ok((message, _)) = Message::from_armor(armored.as_bytes()) else {
            return None;
        };
        let Ok(mut decrypted) = message.decrypt(&password, key) else {
            continue;
        };
        if decrypted.is_compressed() {
            match decrypted.decompress() {
                Ok(decompressed) => decrypted = decompressed,
                Err(_) => continue,
            }
        }
        if let Ok(data) = decrypted.as_data_vec() {
            return Some(data);
        }
    }
    None
}

/// Public and secret keys available for verification and decryption.
#[derive(Default)]
pub struct Keyring {
    pub public: Vec<SignedPublicKey>,
    pub secret: Vec<SignedSecretKey>,
}

impl Keyring {
    pub fn is_empty(&self) -> bool {
        self.public.is_empty() && self.secret.is_empty()
    }

    /// Load every key file in `dir`. Files ending in `.pub`/`.asc` are treated
    /// as public keys, `.sec` as secret keys; both armored and binary parse.
    pub fn load(dir: &Path) -> Self {
        let mut keyring = Keyring::default();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return keyring;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name.contains(".sec") {
                if let Ok((key, _)) = SignedSecretKey::from_reader_single(&bytes[..]) {
                    keyring.secret.push(key);
                }
            } else if let Ok((key, _)) = SignedPublicKey::from_reader_single(&bytes[..]) {
                keyring.public.push(key);
            }
        }
        keyring
    }
}

/// Default directory for PGP/S/MIME key material.
pub fn default_keys_dir() -> PathBuf {
    // An explicit directory wins, so an operator (or a test) can point the
    // keyring somewhere other than the user's data directory.
    if let Some(dir) = std::env::var_os("SOLVERFORGE_MAIL_KEYS_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solverforge")
        .join("mail")
        .join("keys")
}

/// Resolve a passphrase for encrypted secret keys from the environment or a
/// `passphrase` file in the keys directory.
pub fn resolve_passphrase() -> String {
    if let Ok(value) = std::env::var("SOLVERFORGE_PGP_PASSPHRASE") {
        return value;
    }
    std::fs::read_to_string(default_keys_dir().join("passphrase"))
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

/// Generate an Ed25519 key pair with signing and encryption subkeys.
pub fn generate_keypair(uid: &str) -> Result<(SignedSecretKey, SignedPublicKey)> {
    let mut signing = SubkeyParamsBuilder::default();
    signing
        .key_type(KeyType::Ed25519Legacy)
        .can_sign(true)
        .can_encrypt(EncryptionCaps::None)
        .can_authenticate(false);
    let mut encryption = SubkeyParamsBuilder::default();
    encryption
        .key_type(KeyType::ECDH(ECCCurve::Curve25519Legacy))
        .can_sign(false)
        .can_encrypt(EncryptionCaps::All)
        .can_authenticate(false);

    let params = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_certify(true)
        .can_sign(false)
        .can_encrypt(EncryptionCaps::None)
        .primary_user_id(uid.into())
        .passphrase(None)
        .subkey(signing.build().context("signing subkey")?)
        .subkey(encryption.build().context("encryption subkey")?)
        .build()
        .context("secret key parameters")?;

    let secret = params.generate(thread_rng()).context("generate key")?;
    let public = SignedPublicKey::from(secret.clone());
    Ok((secret, public))
}

/// Write a key pair as armored files into `dir`.
pub fn write_keypair(
    dir: &Path,
    name: &str,
    secret: &SignedSecretKey,
    public: &SignedPublicKey,
) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let secret_armor = secret.to_armored_string(Default::default())?;
    let public_armor = public.to_armored_string(Default::default())?;
    std::fs::write(dir.join(format!("{name}.sec.asc")), secret_armor)?;
    std::fs::write(dir.join(format!("{name}.pub.asc")), public_armor)?;
    Ok(())
}

#[cfg(test)]
mod tests;
