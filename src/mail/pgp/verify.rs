/*! OpenPGP signature verification.
Keys frequently sign with a subkey rather than the primary key, so every
candidate verifying key on a certificate is tried. */

use mail_parser::{MessageParser, MimeHeaders, PartType};
use pgp::composed::{CleartextSignedMessage, Deserializable, DetachedSignature, SignedPublicKey};
use pgp::types::KeyDetails;

/// Fingerprints of public keys that validly signed a cleartext message.
pub fn verify_cleartext(armored: &str, keys: &[SignedPublicKey]) -> Vec<String> {
    let Ok((message, _)) = CleartextSignedMessage::from_string(armored) else {
        return Vec::new();
    };
    keys.iter()
        .filter(|key| cleartext_verifies(&message, key))
        .map(|key| key.fingerprint().to_string())
        .collect()
}

/// Fingerprints of public keys that validly produced a detached signature.
pub fn verify_detached(content: &[u8], signature: &[u8], keys: &[SignedPublicKey]) -> Vec<String> {
    let Ok((signature, _)) = DetachedSignature::from_reader_single(signature) else {
        return Vec::new();
    };
    keys.iter()
        .filter(|key| detached_verifies(&signature, key, content))
        .map(|key| key.fingerprint().to_string())
        .collect()
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

fn cleartext_verifies(message: &CleartextSignedMessage, key: &SignedPublicKey) -> bool {
    if message.verify(&key.primary_key).is_ok() {
        return true;
    }
    key.public_subkeys
        .iter()
        .any(|subkey| message.verify(&subkey.key).is_ok())
}

fn detached_verifies(signature: &DetachedSignature, key: &SignedPublicKey, content: &[u8]) -> bool {
    if signature.verify(&key.primary_key, content).is_ok() {
        return true;
    }
    key.public_subkeys
        .iter()
        .any(|subkey| signature.verify(&subkey.key, content).is_ok())
}

fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    text.replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
}
