use super::*;
use crate::mail::pgp;

fn keypair(uid: &str) -> (SignedSecretKey, SignedPublicKey) {
    pgp::generate_keypair(uid).expect("generate keypair")
}

#[test]
fn sign_and_verify_round_trip() {
    let (secret, public) = keypair("Alice <alice@example.com>");
    let content = b"Content-Type: text/plain\r\n\r\nhello\r\n";

    let armored = sign_detached(content, &secret, "").expect("sign");
    assert!(armored.contains("BEGIN PGP SIGNATURE"));

    let fingerprints = pgp::verify_detached(content, armored.as_bytes(), &[public]);
    assert_eq!(fingerprints.len(), 1);
}

#[test]
fn encrypt_and_decrypt_round_trip() {
    let (secret, public) = keypair("Alice <alice@example.com>");
    let body = b"From: alice@example.com\r\n\r\nsecret body\r\n";

    let armored = encrypt_to_keys(body, &[public]).expect("encrypt");
    assert!(armored.contains("BEGIN PGP MESSAGE"));

    let decrypted = pgp::decrypt_inline(&armored, &[secret], "").expect("decrypt");
    assert_eq!(decrypted, body);
}

#[test]
fn recipient_keys_match_by_email() {
    let (_, alice) = keypair("Alice <alice@example.com>");
    let (_, bob) = keypair("Bob <bob@example.com>");

    let selected = recipient_keys(&[alice, bob], &["alice@example.com".to_string()]);
    assert_eq!(selected.len(), 1);
    assert!(key_emails(&selected[0])[0].contains("alice@example.com"));
}

#[test]
fn recipient_keys_empty_when_no_addresses() {
    let (_, alice) = keypair("Alice <alice@example.com>");
    assert!(recipient_keys(&[alice], &[]).is_empty());
    assert!(recipient_keys(&[], &["alice@example.com".to_string()]).is_empty());
}
