/*! S/MIME (PKCS#7) verification and decryption.
Uses OpenSSL for CMS SignedData and EnvelopedData. Certificates and private
keys are loaded from PEM/DER files. */

use std::path::Path;

use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::PKey;
use openssl::x509::X509;

mod verify;

pub use verify::{
    name_to_string, trust_certificate, verify_detached, verify_detached_detailed, verify_detailed,
    verify_mime, verify_mime_detailed, verify_signed_data,
};

/// Identity and trust state of one CMS signer certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmimeSigner {
    /// Subject distinguished name, e.g. `CN=Alice, emailAddress=alice@example.com`.
    pub subject: String,
    pub issuer: String,
    /// Lowercase SHA-256 fingerprint, no separators.
    pub fingerprint: String,
    /// Whether the certificate chains to the configured trust store.
    pub trusted: bool,
    pub not_before: String,
    pub not_after: String,
    /// DER bytes, so the signer can be added to the trust store.
    pub der: Vec<u8>,
}

/// Outcome of verifying a CMS SignedData blob.
#[derive(Debug, Clone)]
pub struct SmimeVerification {
    /// Signed content, present only when verification succeeded.
    pub content: Option<Vec<u8>>,
    pub signers: Vec<SmimeSigner>,
}

impl SmimeVerification {
    /// True when verification succeeded and every signer is trusted.
    pub fn is_trusted(&self) -> bool {
        self.content.is_some() && !self.signers.is_empty() && self.signers.iter().all(|s| s.trusted)
    }

    /// The signer certificate to offer for trust, when it is not yet trusted.
    pub fn untrusted_signer(&self) -> Option<&SmimeSigner> {
        self.signers.iter().find(|signer| !signer.trusted)
    }
}

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

/// Certificates covering `emails`, matched on the certificate's email
/// addresses. Recipients without a certificate are simply absent, so the
/// caller can report exactly who cannot be encrypted to.
pub fn recipient_certs(certs: &[X509], emails: &[String]) -> Vec<X509> {
    let mut wanted = emails
        .iter()
        .map(|email| email.to_lowercase())
        .collect::<Vec<_>>();
    wanted.sort();
    wanted.dedup();

    let mut selected = Vec::new();
    for cert in certs {
        let addresses = cert_emails(cert);
        if wanted
            .iter()
            .any(|wanted| addresses.iter().any(|address| address == wanted))
        {
            selected.push(cert.clone());
        }
    }
    selected
}

/// Lowercased email addresses carried by a certificate.
pub fn cert_emails(cert: &X509) -> Vec<String> {
    let mut addresses = Vec::new();
    for entry in cert.subject_name().entries() {
        if entry.object().nid().short_name().ok() == Some("emailAddress") {
            if let Ok(value) = entry.data().as_utf8() {
                addresses.push(value.to_string().to_lowercase());
            }
        }
    }
    if let Some(names) = cert.subject_alt_names() {
        for name in names {
            if let Some(email) = name.email() {
                addresses.push(email.to_lowercase());
            }
        }
    }
    addresses
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
