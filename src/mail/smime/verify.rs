/*! CMS SignedData verification and certificate trust handling. */

use std::path::Path;

use anyhow::{Context, Result};
use mail_parser::{MessageParser, MimeHeaders, PartType};
use openssl::hash::MessageDigest;
use openssl::pkcs7::Pkcs7Flags;
use openssl::ssl::SslFiletype;
use openssl::stack::Stack;
use openssl::x509::store::{X509Lookup, X509Store, X509StoreBuilder};
use openssl::x509::verify::X509VerifyFlags;
use openssl::x509::{X509NameRef, X509Ref, X509StoreContext, X509};

use super::{parse_pkcs7, SmimeSigner, SmimeVerification, TrustStore};

/// `X509_V_ERR_CERT_REVOKED`, which the crate does not name.
const REVOKED_CODE: i32 = 23;

/// How a signer certificate relates to the trust store.
enum SignerTrust {
    Trusted,
    Untrusted,
    /// A configured revocation list revokes the certificate.
    Revoked,
}

/// Verify a CMS SignedData blob and return its content on success.
pub fn verify_signed_data(der: &[u8], trusted: &TrustStore) -> Option<Vec<u8>> {
    verify_detailed(der, None, trusted)?.content
}

/// Verify a CMS SignedData blob, reporting the signers and their trust.
///
/// The signer identities are reported even when verification fails, so the app
/// can offer to trust a certificate it has not seen before.
pub fn verify_detailed(
    der: &[u8],
    content: Option<&[u8]>,
    trusted: &TrustStore,
) -> Option<SmimeVerification> {
    let pkcs7 = parse_pkcs7(der)?;
    // Signature validity is checked without revocation, so a revoked
    // certificate is reported as revoked rather than as a broken signature.
    let store = trust_store(trusted, false)?;
    let revocation_store = trust_store(trusted, true)?;
    let embedded = Stack::new().ok()?;
    let signers = pkcs7.signers(&embedded, Pkcs7Flags::empty()).ok()?;

    let mut out = Vec::new();
    let verified = pkcs7
        .verify(
            &embedded,
            &store,
            content,
            Some(&mut out),
            Pkcs7Flags::BINARY,
        )
        .is_ok();

    let described = signers
        .iter()
        .map(|cert| describe_signer(cert, &store, &revocation_store, verified))
        .collect();

    Some(SmimeVerification {
        content: verified.then_some(out),
        signers: described,
    })
}

/// Verify a detached CMS signature over `content`.
pub fn verify_detached(content: &[u8], signature: &[u8], trusted: &TrustStore) -> Option<Vec<u8>> {
    verify_detailed(signature, Some(content), trusted)?.content
}

/// Verify a detached CMS signature and report signer details.
pub fn verify_detached_detailed(
    content: &[u8],
    signature: &[u8],
    trusted: &TrustStore,
) -> Option<SmimeVerification> {
    verify_detailed(signature, Some(content), trusted)
}

/// Verify an S/MIME multipart/signed message, returning the signed content.
pub fn verify_mime(raw: &[u8], trusted: &TrustStore) -> Option<Vec<u8>> {
    verify_mime_detailed(raw, trusted)?.content
}

/// Verify an S/MIME multipart/signed message with signer details.
pub fn verify_mime_detailed(raw: &[u8], trusted: &TrustStore) -> Option<SmimeVerification> {
    let signature = mime_signature(raw)?;
    let mut result = verify_detailed(&signature.cms, Some(&signature.signed), trusted)?;
    if result.content.is_none() {
        let normalized = normalize_crlf(&signature.signed);
        if let Some(alternative) = verify_detailed(&signature.cms, Some(&normalized), trusted) {
            if alternative.content.is_some() {
                result = alternative;
            }
        }
    }
    Some(result)
}

/// Detached CMS blob and the exact signed bytes from a multipart/signed message.
pub struct MimeSignature {
    pub cms: Vec<u8>,
    pub signed: Vec<u8>,
}

/// Extract the detached signature and signed part from a multipart/signed body.
pub fn mime_signature(raw: &[u8]) -> Option<MimeSignature> {
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
    let cms = message.part(ids[1])?.contents().to_vec();
    let raw_message = message.raw_message.as_ref();
    let signed = raw_message
        .get(signed.offset_header as usize..signed.offset_end as usize)?
        .to_vec();
    Some(MimeSignature { cms, signed })
}

/// Store a certificate in `dir` as `<sha256>.crt`, marking it trusted.
pub fn trust_certificate(dir: &Path, der: &[u8]) -> Result<std::path::PathBuf> {
    let cert = X509::from_der(der).context("parse certificate")?;
    let fingerprint = hex(&cert.digest(MessageDigest::sha256())?);
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!("{fingerprint}.crt"));
    std::fs::write(&path, der).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Render a distinguished name as `CN=Alice, emailAddress=alice@example.com`.
pub fn name_to_string(name: &X509NameRef) -> String {
    let mut parts = Vec::new();
    for entry in name.entries() {
        let key = entry.object().nid().short_name().unwrap_or("?");
        let value = entry
            .data()
            .as_utf8()
            .map(|value| value.to_string())
            .unwrap_or_default();
        parts.push(format!("{key}={value}"));
    }
    parts.join(", ")
}

/// Build a trust store from the certificates, optionally consulting the
/// revocation lists.
///
/// Revocation is a separate query rather than part of signature verification:
/// with `CRL_CHECK` a revoked certificate makes the whole `Pkcs7::verify` fail,
/// which would report a revoked signer as a broken signature.
fn trust_store(trusted: &TrustStore, with_revocation: bool) -> Option<X509Store> {
    let mut builder = X509StoreBuilder::new().ok()?;
    for cert in &trusted.certs {
        let _ = builder.add_cert(cert.clone());
    }
    if with_revocation && !trusted.crl_paths.is_empty() {
        // A CRL file is loaded through a file lookup before the flag is set.
        let lookup = builder.add_lookup(X509Lookup::file()).ok()?;
        for path in &trusted.crl_paths {
            lookup.load_crl_file(path, SslFiletype::PEM).ok()?;
        }
        builder.set_flags(X509VerifyFlags::CRL_CHECK).ok()?;
    }
    Some(builder.build())
}

/// Describe one signer certificate, checking it against the trust store.
fn describe_signer(
    cert: &X509Ref,
    store: &X509Store,
    revocation_store: &X509Store,
    verified: bool,
) -> SmimeSigner {
    let trust = if verified {
        certificate_trust(cert, store)
    } else {
        SignerTrust::Untrusted
    };
    let revoked = matches!(
        certificate_trust(cert, revocation_store),
        SignerTrust::Revoked
    );
    SmimeSigner {
        subject: name_to_string(cert.subject_name()),
        issuer: name_to_string(cert.issuer_name()),
        fingerprint: cert
            .digest(MessageDigest::sha256())
            .map(|bytes| hex(&bytes))
            .unwrap_or_default(),
        // A revoked certificate is never trusted, however valid the signature.
        trusted: matches!(trust, SignerTrust::Trusted) && !revoked,
        revoked,
        not_before: cert.not_before().to_string(),
        not_after: cert.not_after().to_string(),
        der: cert.to_der().unwrap_or_default(),
    }
}

/// Whether a signer certificate chains to the store, and whether it is revoked.
fn certificate_trust(cert: &X509Ref, store: &X509Store) -> SignerTrust {
    let Ok(certs) = Stack::new() else {
        return SignerTrust::Untrusted;
    };
    let Ok(mut context) = X509StoreContext::new() else {
        return SignerTrust::Untrusted;
    };
    context
        .init(store, cert, &certs, |ctx| {
            // `verify_cert` reports a failed verification as `Ok(false)` with
            // the reason in the context, not as an error.
            match ctx.verify_cert() {
                Ok(true) => Ok(SignerTrust::Trusted),
                Ok(false) if ctx.error().as_raw() == REVOKED_CODE => Ok(SignerTrust::Revoked),
                Ok(false) => Ok(SignerTrust::Untrusted),
                // X509_V_ERR_CERT_REVOKED; the crate exposes no named constant.
                Err(_) if ctx.error().as_raw() == REVOKED_CODE => Ok(SignerTrust::Revoked),
                Err(error) => Err(error),
            }
        })
        .unwrap_or(SignerTrust::Untrusted)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
}
