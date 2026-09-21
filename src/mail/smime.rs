/*! S/MIME (PKCS#7) verification and decryption.
Uses OpenSSL for CMS SignedData and EnvelopedData. Certificates and private
keys are loaded from PEM/DER files. */

use std::path::Path;

use mail_parser::{MessageParser, MimeHeaders, PartType};
use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::PKey;
use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::X509;

/// Trusted certificates plus recipient key pairs for decryption.
#[derive(Default)]
pub struct SmimeKeyring {
    pub certs: Vec<X509>,
    pub pairs: Vec<(PKey<openssl::pkey::Private>, X509)>,
}

impl SmimeKeyring {
    pub fn is_empty(&self) -> bool {
        self.certs.is_empty() && self.pairs.is_empty()
    }

    /// Load certificates (`*.crt`/`*.pem`/`*.der`) and keys (`*.key`) from `dir`,
    /// pairing a key with the certificate that shares its file stem.
    pub fn load(dir: &Path) -> Self {
        let mut certs: Vec<(String, X509)> = Vec::new();
        let mut keys: Vec<(String, PKey<openssl::pkey::Private>)> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                if let Some(stem) = name.strip_suffix(".key") {
                    if let Ok(key) = PKey::private_key_from_pem(&bytes) {
                        keys.push((stem.to_string(), key));
                    }
                } else if let Some(cert) = parse_cert(&bytes) {
                    let stem = name
                        .rsplit_once('.')
                        .map(|(stem, _)| stem.to_string())
                        .unwrap_or_else(|| name.clone());
                    certs.push((stem, cert));
                }
            }
        }

        let pairs = keys
            .into_iter()
            .filter_map(|(stem, key)| {
                certs
                    .iter()
                    .find(|(cert_stem, _)| cert_stem == &stem)
                    .map(|(_, cert)| (key, cert.clone()))
            })
            .collect();

        SmimeKeyring {
            certs: certs.into_iter().map(|(_, cert)| cert).collect(),
            pairs,
        }
    }
}

fn parse_cert(bytes: &[u8]) -> Option<X509> {
    X509::from_pem(bytes)
        .ok()
        .or_else(|| X509::from_der(bytes).ok())
}

fn parse_pkcs7(bytes: &[u8]) -> Option<Pkcs7> {
    Pkcs7::from_der(bytes)
        .ok()
        .or_else(|| Pkcs7::from_pem(bytes).ok())
}

/// Verify a CMS SignedData blob and return its content on success.
pub fn verify_signed_data(der: &[u8], trusted: &[X509]) -> Option<Vec<u8>> {
    let pkcs7 = parse_pkcs7(der)?;
    let mut builder = X509StoreBuilder::new().ok()?;
    for cert in trusted {
        let _ = builder.add_cert(cert.clone());
    }
    let store = builder.build();
    let certs = Stack::new().ok()?;
    let mut out = Vec::new();
    pkcs7
        .verify(&certs, &store, None, Some(&mut out), Pkcs7Flags::BINARY)
        .ok()?;
    Some(out)
}

/// Verify an S/MIME multipart/signed message, returning the signed content.
pub fn verify_mime(raw: &[u8], trusted: &[X509]) -> Option<Vec<u8>> {
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
    let signature = message.part(ids[1])?.contents();
    let raw_message = message.raw_message.as_ref();
    let signed_bytes =
        raw_message.get(signed.offset_header as usize..signed.offset_end as usize)?;

    verify_detached(signed_bytes, signature, trusted)
        .or_else(|| verify_detached(&normalize_crlf(signed_bytes), signature, trusted))
}

fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
}

/// Verify a detached CMS signature over `content`.
pub fn verify_detached(content: &[u8], signature: &[u8], trusted: &[X509]) -> Option<Vec<u8>> {
    let pkcs7 = parse_pkcs7(signature)?;
    let mut builder = X509StoreBuilder::new().ok()?;
    for cert in trusted {
        let _ = builder.add_cert(cert.clone());
    }
    let store = builder.build();
    let certs = Stack::new().ok()?;
    pkcs7
        .verify(&certs, &store, Some(content), None, Pkcs7Flags::BINARY)
        .ok()
        .map(|_| content.to_vec())
}

/// Decrypt a CMS EnvelopedData blob with the first matching key pair.
pub fn decrypt_enveloped(der: &[u8], keyring: &SmimeKeyring) -> Option<Vec<u8>> {
    let pkcs7 = parse_pkcs7(der)?;
    for (key, cert) in &keyring.pairs {
        if let Ok(data) = pkcs7.decrypt(key, cert, Pkcs7Flags::BINARY) {
            return Some(data);
        }
    }
    None
}

#[cfg(test)]
mod tests;
