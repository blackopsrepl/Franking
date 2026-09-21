/*! OpenPGP round-trip tests using generated keys. */

use pgp::composed::{
    CleartextSignedMessage, DetachedSignature, EncryptionCaps, KeyType, MessageBuilder,
    SecretKeyParamsBuilder, SignedPublicKey, SignedSecretKey, SubkeyParamsBuilder,
};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::{KeyDetails, Password};
use rand::thread_rng;

use super::{
    decrypt_inline, decrypt_mime, detect_inline, verify_cleartext, verify_mime, InlinePgp,
};

fn keypair(uid: &str) -> (SignedSecretKey, SignedPublicKey) {
    let mut signing = SubkeyParamsBuilder::default();
    signing
        .key_type(KeyType::Ed25519Legacy)
        .can_sign(true)
        .can_encrypt(EncryptionCaps::None)
        .can_authenticate(false);
    let mut encryption = SubkeyParamsBuilder::default();
    encryption
        .key_type(KeyType::ECDH(
            pgp::crypto::ecc_curve::ECCCurve::Curve25519Legacy,
        ))
        .can_sign(false)
        .can_encrypt(EncryptionCaps::All)
        .can_authenticate(false);

    let params = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_certify(true)
        .can_sign(false)
        .can_encrypt(EncryptionCaps::None)
        .primary_user_id(uid.into())
        .passphrase(None)
        .subkey(signing.build().unwrap())
        .subkey(encryption.build().unwrap())
        .build()
        .unwrap();

    let secret = params.generate(thread_rng()).unwrap();
    let public = SignedPublicKey::from(secret.clone());
    (secret, public)
}

#[test]
fn verifies_a_cleartext_signature() {
    let (secret, public) = keypair("Alice <alice@example.com>");
    let signed =
        CleartextSignedMessage::sign(thread_rng(), "hello world", &*secret, &Password::empty())
            .unwrap();
    let armored = signed.to_armored_string(Default::default()).unwrap();

    assert_eq!(detect_inline(&armored), Some(InlinePgp::Signed));
    let fingerprints = verify_cleartext(&armored, &[public]);
    assert_eq!(fingerprints.len(), 1);
}

#[test]
fn decrypts_an_inline_message() {
    let (secret, public) = keypair("Bob <bob@example.com>");
    let encryption_subkey = public
        .public_subkeys
        .iter()
        .find(|subkey| subkey.algorithm().can_encrypt())
        .expect("an encryption subkey");
    let mut builder = MessageBuilder::from_bytes("", b"secret text".to_vec())
        .seipd_v1(thread_rng(), SymmetricKeyAlgorithm::AES256);
    builder
        .encrypt_to_key(thread_rng(), encryption_subkey)
        .unwrap();
    let armored = builder
        .to_armored_string(thread_rng(), Default::default())
        .unwrap();

    assert_eq!(detect_inline(&armored), Some(InlinePgp::Encrypted));
    let decrypted = decrypt_inline(&armored, &[secret], "").unwrap();
    assert_eq!(decrypted, b"secret text");
}

#[test]
fn verifies_a_pgp_mime_message() {
    let (secret, public) = keypair("Carol <carol@example.com>");
    let signed_part = "Content-Type: text/plain\r\n\r\nhello mime";
    let signature = DetachedSignature::sign_binary_data(
        thread_rng(),
        &*secret,
        &Password::empty(),
        pgp::crypto::hash::HashAlgorithm::Sha256,
        signed_part.as_bytes(),
    )
    .unwrap();
    let armored = signature.to_armored_string(Default::default()).unwrap();

    let message = format!(
        "MIME-Version: 1.0\r\nContent-Type: multipart/signed; protocol=\"application/pgp-signature\"; micalg=pgp-sha256; boundary=s\r\n\r\n--s\r\n{signed_part}\r\n--s\r\nContent-Type: application/pgp-signature\r\n\r\n{armored}\r\n--s--\r\n"
    );

    let fingerprints = verify_mime(message.as_bytes(), &[public]).expect("pgp/mime structure");
    assert_eq!(fingerprints.len(), 1, "signature should verify");
}

#[test]
fn decrypts_a_pgp_mime_message() {
    let (secret, public) = keypair("Dave <dave@example.com>");
    let encryption_subkey = public
        .public_subkeys
        .iter()
        .find(|subkey| subkey.algorithm().can_encrypt())
        .expect("an encryption subkey");
    let mut builder = MessageBuilder::from_bytes("", b"mime secret".to_vec())
        .seipd_v1(thread_rng(), SymmetricKeyAlgorithm::AES256);
    builder
        .encrypt_to_key(thread_rng(), encryption_subkey)
        .unwrap();
    let encrypted = builder.to_vec(thread_rng()).unwrap();

    let mut message = Vec::new();
    message.extend_from_slice(
        b"MIME-Version: 1.0\r\nContent-Type: multipart/encrypted; protocol=\"application/pgp-encrypted\"; boundary=e\r\n\r\n--e\r\nContent-Type: application/pgp-encrypted\r\n\r\nVersion: 1\r\n--e\r\nContent-Type: application/octet-stream\r\n\r\n",
    );
    message.extend_from_slice(&encrypted);
    message.extend_from_slice(b"\r\n--e--\r\n");

    let decrypted = decrypt_mime(&message, &[secret], "").unwrap();
    assert_eq!(decrypted, b"mime secret");
}

#[test]
fn writes_and_reloads_a_keypair() {
    let dir = std::env::temp_dir().join(format!("sfm-pgp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let (secret, public) = super::generate_keypair("Eve <eve@example.com>").unwrap();
    super::write_keypair(&dir, "eve", &secret, &public).unwrap();

    let keyring = super::Keyring::load(&dir);
    assert_eq!(keyring.public.len(), 1);
    assert_eq!(keyring.secret.len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}
