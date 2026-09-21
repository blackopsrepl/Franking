/*! OpenPGP verification and decryption.
Uses rpgp for cleartext-signed and inline-encrypted messages, and for detached
signatures. Keyrings are loaded from armored/binary key files. */

use std::path::Path;

use mail_parser::{MessageParser, MimeHeaders, PartType};
use pgp::composed::{
    CleartextSignedMessage, Deserializable, Message, SignedPublicKey, SignedSecretKey,
};
use pgp::types::{KeyDetails, Password};

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

/// Fingerprints of public keys that validly signed a cleartext message.
pub fn verify_cleartext(armored: &str, keys: &[SignedPublicKey]) -> Vec<String> {
    let Ok((message, _)) = CleartextSignedMessage::from_string(armored) else {
        return Vec::new();
    };
    keys.iter()
        .filter(|key| message.verify(*key).is_ok())
        .map(|key| key.fingerprint().to_string())
        .collect()
}

/// Fingerprints of public keys that validly produced a detached signature.
pub fn verify_detached(content: &[u8], signature: &[u8], keys: &[SignedPublicKey]) -> Vec<String> {
    let Ok((signature, _)) = pgp::composed::DetachedSignature::from_reader_single(signature) else {
        return Vec::new();
    };
    keys.iter()
        .filter(|key| signature.verify(*key, content).is_ok())
        .map(|key| key.fingerprint().to_string())
        .collect()
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
        let Ok(message) = Message::from_bytes(std::io::Cursor::new(ciphertext)) else {
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

/// Verify a PGP/MIME (multipart/signed) message, returning signer fingerprints.
pub fn verify_mime(raw: &[u8], keys: &[SignedPublicKey]) -> Option<Vec<String>> {
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
            .is_some_and(|subtype| subtype.eq_ignore_ascii_case("signed"))
    {
        return None;
    }
    let PartType::Multipart(ids) = &root.body else {
        return None;
    };
    if ids.len() < 2 {
        return None;
    }
    let signed = message.part(ids[0])?;
    let signature_part = message.part(ids[1])?;
    let raw_message = message.raw_message.as_ref();
    let start = signed.offset_header as usize;
    let end = signed.offset_end as usize;
    let signed_bytes = raw_message.get(start..end)?;
    let signature_bytes = signature_part.contents();

    let mut fingerprints = verify_detached(signed_bytes, signature_bytes, keys);
    if fingerprints.is_empty() {
        // Some producers sign the LF-normalized form.
        fingerprints = verify_detached(&normalize_crlf(signed_bytes), signature_bytes, keys);
    }
    Some(fingerprints)
}

fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    text.replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
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

#[cfg(test)]
mod tests;
