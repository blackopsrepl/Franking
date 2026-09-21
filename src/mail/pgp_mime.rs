/*! Outbound PGP/MIME assembly (RFC 3156).
Wraps a complete RFC 5322 message as `multipart/signed`, `multipart/encrypted`,
or sign-then-encrypt, matching the structure the inbound verifier expects. */

use anyhow::{anyhow, Context, Result};

use crate::mail::mime_wrap::{assemble, entity_bytes, entity_with, split_message, unique_boundary};
use crate::mail::pgp::Keyring;
use crate::mail::pgp_sign;
use crate::mail::service::SendOptions;

/// Content type of the version control part in `multipart/encrypted`.
const PGP_ENCRYPTED: &str = "application/pgp-encrypted";
/// Content type of the signature part in `multipart/signed`.
const PGP_SIGNATURE: &str = "application/pgp-signature";

/// Wrap `raw` (a full RFC 5322 message) as PGP/MIME.
pub fn wrap(raw: &[u8], options: &SendOptions, keyring: &Keyring) -> Result<Vec<u8>> {
    let message = split_message(raw);
    let entity = entity_bytes(&message);

    let protected = if options.sign && options.encrypt {
        let signed = sign_entity(&entity, keyring, options)?;
        // The inner entity must carry its own Content-Type header, or the
        // decrypted bytes cannot be parsed back as signed data.
        let signed_entity = entity_with(&content_type_header(&signed.content_type), &signed.body);
        encrypt_entity(&signed_entity, &message.recipients(), keyring)?
    } else if options.sign {
        sign_entity(&entity, keyring, options)?
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
/// A `Content-Type` header line for a content type value.
fn content_type_header(value: &str) -> String {
    format!("Content-Type: {value}")
}

struct Protected {
    content_type: String,
    body: Vec<u8>,
}

fn sign_entity(entity: &[u8], keyring: &Keyring, options: &SendOptions) -> Result<Protected> {
    let signer = keyring
        .secret
        .first()
        .ok_or_else(|| anyhow!("no secret key available to sign the message"))?;
    let signature = pgp_sign::sign_detached(entity, signer, &options.passphrase)
        .context("sign outgoing message")?;

    let boundary = unique_boundary(entity, "signed");
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(entity);
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Type: {PGP_SIGNATURE}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Transfer-Encoding: 7bit\r\n\r\n");
    body.extend_from_slice(signature.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    Ok(Protected {
        content_type: format!(
            "multipart/signed; micalg=pgp-sha256; protocol=\"{PGP_SIGNATURE}\"; boundary=\"{boundary}\""
        ),
        body,
    })
}

fn encrypt_entity(entity: &[u8], recipients: &[String], keyring: &Keyring) -> Result<Protected> {
    let keys = pgp_sign::recipient_keys(&keyring.public, recipients);
    if keys.is_empty() {
        return Err(anyhow!("no public key found for {}", recipients.join(", ")));
    }
    encrypt_to_keys(entity, &keys)
}

/// Encrypt the message to explicit keys, keeping the envelope headers.
///
/// Used for drafts, which are encrypted to the sender rather than to the
/// message's recipients: a draft is a private note until it is sent.
pub fn encrypt_for_emails(raw: &[u8], emails: &[String], keyring: &Keyring) -> Result<Vec<u8>> {
    let keys = pgp_sign::recipient_keys(&keyring.public, emails);
    if keys.is_empty() {
        return Err(anyhow!("no public key found for {}", emails.join(", ")));
    }
    let message = split_message(raw);
    let entity = entity_bytes(&message);
    let protected = encrypt_to_keys(&entity, &keys)?;
    Ok(assemble(
        &message.envelope,
        &protected.content_type,
        &protected.body,
    ))
}

fn encrypt_to_keys(entity: &[u8], keys: &[::pgp::composed::SignedPublicKey]) -> Result<Protected> {
    let ciphertext = pgp_sign::encrypt_to_keys(entity, keys).context("encrypt outgoing message")?;

    let boundary = unique_boundary(entity, "encrypted");
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Type: {PGP_ENCRYPTED}\r\n\r\n").as_bytes());
    body.extend_from_slice(b"Version: 1\r\n");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(ciphertext.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    Ok(Protected {
        content_type: format!(
            "multipart/encrypted; protocol=\"{PGP_ENCRYPTED}\"; boundary=\"{boundary}\""
        ),
        body,
    })
}

#[cfg(test)]
mod tests;
