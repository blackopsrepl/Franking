/*! Outbound S/MIME assembly.
Wraps a complete RFC 5322 message as `multipart/signed` or as an enveloped
`application/pkcs7-mime`, matching the structure the inbound verifier expects. */

use anyhow::{anyhow, Context, Result};

use crate::mail::mime_wrap::{assemble, entity_bytes, entity_with, split_message, unique_boundary};
use crate::mail::service::SendOptions;
use crate::mail::smime::{self, SmimeKeyring};
use crate::mail::smime_sign;

/// Content type of the detached signature part in `multipart/signed`.
const PKCS7_SIGNATURE: &str = "application/pkcs7-signature";
/// Content type of an enveloped message.
const PKCS7_MIME: &str = "application/pkcs7-mime";

/// Wrap `raw` (a full RFC 5322 message) as S/MIME per `options`.
pub fn wrap(raw: &[u8], options: &SendOptions, keyring: &SmimeKeyring) -> Result<Vec<u8>> {
    let message = split_message(raw);
    let entity = entity_bytes(&message);

    let protected = if options.smime_sign && options.smime_encrypt {
        let signed = sign_entity(&entity, keyring)?;
        // The inner entity must carry its own Content-Type header, or the
        // decrypted bytes cannot be parsed back as signed data.
        let signed_entity = entity_with(&content_type_header(&signed.content_type), &signed.body);
        encrypt_entity(&signed_entity, &message.recipients(), keyring)?
    } else if options.smime_sign {
        sign_entity(&entity, keyring)?
    } else {
        encrypt_entity(&entity, &message.recipients(), keyring)?
    };

    Ok(assemble(
        &message.envelope,
        &protected.content_type,
        &protected.body,
    ))
}

/// A MIME entity produced by signing or encrypting.
struct Protected {
    content_type: String,
    body: Vec<u8>,
}

fn sign_entity(entity: &[u8], keyring: &SmimeKeyring) -> Result<Protected> {
    let (key, cert) = keyring
        .pairs
        .first()
        .ok_or_else(|| anyhow!("no certificate and key pair available to sign the message"))?;
    let signature =
        smime_sign::sign_detached(entity, key, cert).context("sign outgoing message")?;
    let encoded = base64_lines(&signature);

    let boundary = unique_boundary(entity, "smime-signed");
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(entity);
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Type: {PKCS7_SIGNATURE}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Transfer-Encoding: base64\r\n");
    body.extend_from_slice(b"Content-Disposition: attachment; filename=\"smime.p7s\"\r\n\r\n");
    body.extend_from_slice(encoded.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    Ok(Protected {
        content_type: format!(
            "multipart/signed; protocol=\"{PKCS7_SIGNATURE}\"; micalg=sha-256; boundary=\"{boundary}\""
        ),
        body,
    })
}

fn encrypt_entity(
    entity: &[u8],
    recipients: &[String],
    keyring: &SmimeKeyring,
) -> Result<Protected> {
    let certs = smime::recipient_certs(&keyring.certs, recipients);
    if certs.is_empty() {
        return Err(anyhow!(
            "no certificate found for {}",
            recipients.join(", ")
        ));
    }
    let ciphertext = smime_sign::encrypt(entity, &certs).context("encrypt outgoing message")?;

    Ok(Protected {
        content_type: format!("{PKCS7_MIME}; smime-type=enveloped-data; name=\"smime.p7m\""),
        body: base64_lines(&ciphertext).into_bytes(),
    })
}

/// A `Content-Type` header line for a content type value.
fn content_type_header(value: &str) -> String {
    format!("Content-Type: {value}")
}

/// Base64 with CRLF-separated 76-column lines, as S/MIME requires.
fn base64_lines(der: &[u8]) -> String {
    use base64::Engine;

    let encoded = base64::engine::general_purpose::STANDARD.encode(der);
    let mut out = String::with_capacity(encoded.len() + encoded.len() / 76 * 2);
    for (index, chunk) in encoded.as_bytes().chunks(76).enumerate() {
        if index > 0 {
            out.push_str("\r\n");
        }
        out.push_str(&String::from_utf8_lossy(chunk));
    }
    out.push_str("\r\n");
    out
}

#[cfg(test)]
mod tests;
