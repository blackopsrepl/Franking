use super::*;
use crate::mail::pgp;
use crate::mail::pgp_sign;

fn options(sign: bool, encrypt: bool) -> SendOptions {
    SendOptions {
        sign,
        encrypt,
        ..SendOptions::default()
    }
}

fn sample_message() -> Vec<u8> {
    [
        "From: Alice <alice@example.com>",
        "To: Bob <bob@example.com>",
        "Subject: Quarterly report",
        "Date: Mon, 21 Sep 2026 10:00:00 +0000",
        "Message-ID: <report@example.com>",
        "Content-Type: text/plain; charset=utf-8",
        "",
        "Please review the attached numbers.",
    ]
    .join("\r\n")
    .into_bytes()
}

fn keyring(uid: &str) -> (pgp::Keyring, ::pgp::composed::SignedSecretKey) {
    let (secret, public) = pgp::generate_keypair(uid).expect("generate keypair");
    let keyring = pgp::Keyring {
        public: vec![public],
        secret: vec![secret.clone()],
    };
    (keyring, secret)
}

#[test]
fn splits_envelope_headers_from_the_mime_entity() {
    let message = split_message(&sample_message());
    assert!(message.envelope.iter().any(|h| h.starts_with("From:")));
    assert!(message.envelope.iter().any(|h| h.starts_with("To:")));
    assert!(!message
        .mime_headers
        .iter()
        .any(|h| h.to_ascii_lowercase().starts_with("from:")));
    assert_eq!(message.recipients(), vec!["bob@example.com"]);
}

#[test]
fn extracts_addresses_from_display_names_and_lists() {
    assert_eq!(
        extract_addresses("Alice <alice@example.com>, Bob <BOB@example.com>"),
        vec!["alice@example.com", "bob@example.com"]
    );
    assert_eq!(
        extract_addresses("carol@example.com"),
        vec!["carol@example.com"]
    );
    assert!(extract_addresses("undisclosed-recipients:;").is_empty());
}

#[test]
fn crlf_normalization_is_idempotent() {
    assert_eq!(crlf("a\nb"), "a\r\nb");
    assert_eq!(crlf("a\r\nb"), "a\r\nb");
    assert_eq!(crlf("a\r\nb\nc"), "a\r\nb\r\nc");
}

#[test]
fn signed_message_verifies_against_its_signer() {
    let (keys, _) = keyring("Alice <alice@example.com>");
    let wrapped = wrap(&sample_message(), &options(true, false), &keys).expect("wrap");
    assert!(contains(&wrapped, b"multipart/signed"));

    let fingerprints = pgp::verify_mime(&wrapped, &keys.public).expect("mime");
    assert_eq!(fingerprints.len(), 1);
}

#[test]
fn encrypted_message_decrypts_to_the_original_entity() {
    let (keys, secret) = keyring("Bob <bob@example.com>");
    let wrapped = wrap(&sample_message(), &options(false, true), &keys).expect("wrap");
    assert!(contains(&wrapped, b"multipart/encrypted"));

    let decrypted = pgp::decrypt_mime(&wrapped, &[secret], "").expect("decrypt");
    let text = String::from_utf8_lossy(&decrypted);
    assert!(text.contains("Please review the attached numbers."));
    assert!(text.contains("Content-Type: text/plain"));
}

#[test]
fn signed_then_encrypted_message_round_trips() {
    let (keys, secret) = keyring("Bob <bob@example.com>");
    let wrapped = wrap(&sample_message(), &options(true, true), &keys).expect("wrap");
    assert!(contains(&wrapped, b"multipart/encrypted"));

    let decrypted = pgp::decrypt_mime(&wrapped, &[secret], "").expect("decrypt");
    assert!(contains(&decrypted, b"multipart/signed"));
}

#[test]
fn encryption_requires_a_recipient_key() {
    let (keys, _) = keyring("Alice <alice@example.com>");
    let error = wrap(&sample_message(), &options(false, true), &keys)
        .expect_err("no key for bob@example.com");
    assert!(error.to_string().contains("bob@example.com"));
}

#[test]
fn signing_requires_a_secret_key() {
    let keys = pgp::Keyring::default();
    let error = wrap(&sample_message(), &options(true, false), &keys).expect_err("empty keyring");
    assert!(error.to_string().contains("no secret key"));
}

#[test]
fn boundary_avoids_content_collisions() {
    let content = b"=-sfmail-signed-0000000000000000";
    let boundary = unique_boundary(content, "signed");
    assert!(!contains(content, boundary.as_bytes()));
}

#[test]
fn detached_signature_is_accepted_by_the_inbound_path() {
    let (keys, secret) = keyring("Alice <alice@example.com>");
    let entity = entity_bytes(&split_message(&sample_message()));
    let armored = pgp_sign::sign_detached(&entity, &secret, "").expect("sign");
    let fingerprints = pgp::verify_detached(&entity, armored.as_bytes(), &keys.public);
    assert_eq!(fingerprints.len(), 1);
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn multipart_inner_message_survives_signing() {
    let (keys, _) = keyring("Alice <alice@example.com>");
    let inner = [
        "From: Alice <alice@example.com>",
        "To: Bob <bob@example.com>",
        "Subject: With attachment",
        "Content-Type: multipart/mixed; boundary=\"mix\"",
        "",
        "--mix",
        "Content-Type: text/plain; charset=utf-8",
        "",
        "see attached",
        "--mix",
        "Content-Type: application/octet-stream",
        "Content-Transfer-Encoding: base64",
        "",
        "AAECAwQ=",
        "--mix--",
    ]
    .join("\r\n")
    .into_bytes();

    let wrapped = wrap(&inner, &options(true, false), &keys).expect("wrap");
    let fingerprints = pgp::verify_mime(&wrapped, &keys.public).expect("mime");
    assert_eq!(fingerprints.len(), 1);

    let (part, _) = split_signed_for_test(&wrapped);
    let text = String::from_utf8_lossy(&part);
    assert!(text.contains("multipart/mixed"));
    assert!(text.contains("AAECAwQ="));
}

fn split_signed_for_test(wrapped: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let text = String::from_utf8_lossy(wrapped);
    let boundary = text
        .split("boundary=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("boundary");
    let marker = format!("--{boundary}\r\n");
    let first = text.find(&marker).expect("first part") + marker.len();
    let stop = text[first..]
        .find(&format!("\r\n--{boundary}\r\n"))
        .expect("part end")
        + first;
    (wrapped[first..stop].to_vec(), Vec::new())
}
