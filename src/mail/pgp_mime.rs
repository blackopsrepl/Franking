/*! Outbound PGP/MIME assembly (RFC 3156).
Wraps a complete RFC 5322 message as `multipart/signed`, `multipart/encrypted`,
or sign-then-encrypt, matching the structure the inbound verifier expects. */

use anyhow::{anyhow, Context, Result};

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
        let signed_entity = entity_with(&signed.content_type, &signed.body);
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

/// A message split into envelope headers and an inner MIME entity.
struct SplitMessage {
    /// Headers that belong on the outer message (From, To, Subject, …).
    envelope: Vec<String>,
    /// MIME headers of the inner entity (Content-Type, CTE, MIME-Version).
    mime_headers: Vec<String>,
    /// Body bytes of the inner entity.
    body: Vec<u8>,
}

impl SplitMessage {
    /// Recipient email addresses from To/Cc/Bcc, in header order.
    fn recipients(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        for header in &self.envelope {
            let Some((name, value)) = header.split_once(':') else {
                continue;
            };
            if !matches!(
                name.trim().to_ascii_lowercase().as_str(),
                "to" | "cc" | "bcc"
            ) {
                continue;
            }
            addresses.extend(extract_addresses(value));
        }
        addresses
    }
}

/// A MIME entity produced by signing or encrypting.
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
    let ciphertext =
        pgp_sign::encrypt_to_keys(entity, &keys).context("encrypt outgoing message")?;

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

/// Split a raw message at the first blank line.
fn split_message(raw: &[u8]) -> SplitMessage {
    let (head, body) = split_headers(raw);
    let text = String::from_utf8_lossy(head).replace("\r\n", "\n");
    let mut envelope = Vec::new();
    let mut mime_headers = Vec::new();
    for line in unfold(&text) {
        let name = line
            .split_once(':')
            .map(|(name, _)| name.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(
            name.as_str(),
            "content-type" | "content-transfer-encoding" | "mime-version"
        ) {
            mime_headers.push(line);
        } else {
            envelope.push(line);
        }
    }
    SplitMessage {
        envelope,
        mime_headers,
        body: body.to_vec(),
    }
}

/// The inner MIME entity, with CRLF line endings and its own MIME headers.
fn entity_bytes(message: &SplitMessage) -> Vec<u8> {
    let mut mime_headers = message.mime_headers.clone();
    if !mime_headers
        .iter()
        .any(|line| line.to_ascii_lowercase().starts_with("content-type:"))
    {
        mime_headers.push("Content-Type: text/plain; charset=utf-8".to_string());
    }
    entity_with(&mime_headers.join("\r\n"), &message.body)
}

/// Join MIME headers and a body into a CRLF entity, ensuring a trailing CRLF.
fn entity_with(mime_headers: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(crlf(mime_headers).as_bytes());
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(&crlf_bytes(body));
    out.push(b'\r');
    out.push(b'\n');
    out
}

/// Assemble the outer message: envelope headers plus the protected payload.
fn assemble(envelope: &[String], content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut headers = envelope.to_vec();
    headers.push("MIME-Version: 1.0".to_string());
    headers.push(format!("Content-Type: {content_type}"));
    let mut out = crlf(&headers.join("\r\n")).into_bytes();
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(body);
    out
}

/// Address-shaped substrings from a header value, lowercased.
fn extract_addresses(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .filter_map(|chunk| {
            let start = chunk.rfind('<').map(|i| i + 1).unwrap_or(0);
            let end = chunk.rfind('>').unwrap_or(chunk.len());
            let candidate = chunk.get(start..end.max(start))?.trim();
            candidate.contains('@').then(|| candidate.to_lowercase())
        })
        .collect()
}

/// Unfold folded header lines (continuation lines start with whitespace).
fn unfold(head: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in head.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(last) = lines.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
                continue;
            }
        }
        lines.push(line.trim_end().to_string());
    }
    lines
}

/// Split raw bytes at the blank line separating headers from body.
fn split_headers(raw: &[u8]) -> (&[u8], &[u8]) {
    for (index, window) in raw.windows(4).enumerate() {
        if window == b"\r\n\r\n" {
            return (&raw[..index], &raw[index + 4..]);
        }
    }
    for (index, window) in raw.windows(2).enumerate() {
        if window == b"\n\n" {
            return (&raw[..index], &raw[index + 2..]);
        }
    }
    (raw, &[])
}

/// Normalize lone LF to CRLF without doubling existing CRLF.
fn crlf(text: &str) -> String {
    String::from_utf8_lossy(&crlf_bytes(text.as_bytes())).into_owned()
}

fn crlf_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 16);
    let mut previous_cr = false;
    for &byte in bytes {
        if byte == b'\n' && !previous_cr {
            out.push(b'\r');
        }
        out.push(byte);
        previous_cr = byte == b'\r';
    }
    out
}

/// A boundary that does not appear in `content`.
fn unique_boundary(content: &[u8], kind: &str) -> String {
    loop {
        let boundary = format!("=-sfmail-{kind}-{:016x}", rand::random::<u64>());
        if !content
            .windows(boundary.len())
            .any(|window| window == boundary.as_bytes())
        {
            return boundary;
        }
    }
}

#[cfg(test)]
mod tests;
