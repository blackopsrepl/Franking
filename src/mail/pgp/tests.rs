/*! OpenPGP round-trip tests using generated keys. */

use pgp::composed::{
    CleartextSignedMessage, EncryptionCaps, KeyType, MessageBuilder, SecretKeyParamsBuilder,
    SignedPublicKey, SignedSecretKey, SubkeyParamsBuilder,
};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::{KeyDetails, Password};
use rand::thread_rng;

use super::{decrypt_inline, detect_inline, verify_cleartext, InlinePgp};

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
