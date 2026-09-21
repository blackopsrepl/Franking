//! Outbound S/MIME round trips: wrap, then verify or decrypt.
//!
//! Every case builds its key material once and reuses it, since each fixture
//! call generates a fresh key.

use openssl::pkey::{PKey, Private};
use openssl::x509::X509;

use crate::mail::mime_wrap::{entity_bytes, split_message};
use crate::mail::service::SendOptions;
use crate::mail::smime::{self, SmimeKeyring};
use crate::mail::smime_mime::wrap;
use crate::mail::smime_sign;
use crate::mail::test_certs::{certificate, certificate_for};

/// Alice (sender) has a certificate and key; Bob (recipient) has only a
/// certificate, which is what encryption needs.
struct Fixture {
    keyring: SmimeKeyring,
    alice_key: PKey<Private>,
    alice_cert: X509,
    bob_key: PKey<Private>,
    bob_cert: X509,
}

impl Fixture {
    fn new() -> Self {
        let (alice_key, alice_cert) = certificate();
        let (bob_key, bob_cert) = certificate_for("bob@example.com", "Bob");
        Self {
            keyring: SmimeKeyring {
                crl_paths: Vec::new(),
                certs: vec![alice_cert.clone(), bob_cert.clone()],
                pairs: vec![(alice_key.clone(), alice_cert.clone())],
            },
            alice_key,
            alice_cert,
            bob_key,
            bob_cert,
        }
    }

    /// The recipient-side keyring, holding only Bob's pair.
    fn bob_keyring(&self) -> SmimeKeyring {
        SmimeKeyring {
            crl_paths: Vec::new(),
            certs: vec![self.bob_cert.clone()],
            pairs: vec![(self.bob_key.clone(), self.bob_cert.clone())],
        }
    }
}

fn options(sign: bool, encrypt: bool) -> SendOptions {
    SendOptions {
        smime_sign: sign,
        smime_encrypt: encrypt,
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

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn signed_message_verifies_and_carries_the_entity() {
    let fixture = Fixture::new();
    let wrapped = wrap(&sample_message(), &options(true, false), &fixture.keyring).expect("wrap");
    assert!(contains(&wrapped, b"multipart/signed"));
    assert!(contains(&wrapped, b"application/pkcs7-signature"));

    let verification =
        smime::verify_mime_detailed(&wrapped, &fixture.keyring.trust()).expect("verify");
    let signed = verification.content.expect("verified content");
    let text = String::from_utf8_lossy(&signed);
    assert!(text.contains("Content-Type: text/plain; charset=utf-8"));
    assert!(text.contains("Please review the attached numbers."));
}

#[test]
fn encrypted_message_decrypts_to_the_original_entity() {
    let fixture = Fixture::new();
    let wrapped = wrap(&sample_message(), &options(false, true), &fixture.keyring).expect("wrap");
    assert!(contains(&wrapped, b"application/pkcs7-mime"));
    assert!(contains(&wrapped, b"smime-type=enveloped-data"));

    let decrypted = smime::decrypt_enveloped(&payload(&wrapped), &fixture.bob_keyring())
        .expect("the recipient's pair decrypts the message");
    let text = String::from_utf8_lossy(&decrypted);
    assert!(text.contains("Please review the attached numbers."));
}

#[test]
fn signed_then_encrypted_message_round_trips() {
    let fixture = Fixture::new();
    let wrapped = wrap(&sample_message(), &options(true, true), &fixture.keyring).expect("wrap");
    assert!(contains(&wrapped, b"application/pkcs7-mime"));

    let entity =
        smime::decrypt_enveloped(&payload(&wrapped), &fixture.bob_keyring()).expect("decrypt");
    assert!(contains(&entity, b"multipart/signed"));

    let verification =
        smime::verify_mime_detailed(&entity_with_headers(&entity), &fixture.keyring.trust());
    let Some(verification) = verification else {
        panic!(
            "the inner signed entity should parse: {}",
            String::from_utf8_lossy(&entity)
        );
    };
    let signers = verification.signers.len();
    let content = verification.content.map(|c| c.len());
    assert!(
        content.is_some(),
        "inner signature verified: signers={signers}, content={content:?}"
    );
}

#[test]
fn encryption_requires_a_recipient_certificate() {
    let fixture = Fixture::new();
    let mut without_bob = fixture.keyring;
    without_bob.certs = Vec::new();
    let error = wrap(&sample_message(), &options(false, true), &without_bob)
        .expect_err("no certificate for bob@example.com");
    assert!(error.to_string().contains("bob@example.com"));
}

#[test]
fn signing_requires_a_certificate_and_key_pair() {
    let keys = SmimeKeyring::default();
    let error = wrap(&sample_message(), &options(true, false), &keys).expect_err("empty keyring");
    assert!(error.to_string().contains("no certificate and key pair"));
}

#[test]
fn detached_signature_over_the_entity_verifies() {
    let fixture = Fixture::new();
    let entity = entity_bytes(&split_message(&sample_message()));
    let der =
        smime_sign::sign_detached(&entity, &fixture.alice_key, &fixture.alice_cert).expect("sign");

    let verification =
        smime::verify_detached_detailed(&entity, &der, &fixture.keyring.trust()).expect("verify");
    assert_eq!(
        verification.content.as_deref(),
        Some(entity.as_slice()),
        "the signed bytes come back unchanged"
    );
}

#[test]
fn payload_is_folded_base64_and_decodes_to_cms() {
    let fixture = Fixture::new();
    let wrapped = wrap(&sample_message(), &options(false, true), &fixture.keyring).expect("wrap");
    let text = String::from_utf8_lossy(&wrapped);
    let body = text.split("\r\n\r\n").nth(1).expect("enveloped body");
    for line in body.lines().filter(|line| !line.is_empty()) {
        assert!(
            line.len() <= 76,
            "base64 line is folded at 76 columns: {line:?}"
        );
    }
    assert!(openssl::pkcs7::Pkcs7::from_der(&payload(&wrapped)).is_ok());
}

/// The base64 CMS payload of an enveloped message, folded whitespace removed.
fn payload(wrapped: &[u8]) -> Vec<u8> {
    use base64::Engine;

    let text = String::from_utf8_lossy(wrapped);
    let body = text.split("\r\n\r\n").nth(1).expect("enveloped body");
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(compact)
        .expect("base64 payload")
}

/// Give a bare entity the envelope headers a message would carry.
fn entity_with_headers(entity: &[u8]) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(
        b"From: Alice <alice@example.com>\r\nTo: Bob <bob@example.com>\r\nSubject: Re: Quarterly report\r\n",
    );
    message.extend_from_slice(entity);
    message
}
