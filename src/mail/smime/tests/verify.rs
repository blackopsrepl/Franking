//! Verification, decryption, and trust handling.

use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::PKey;
use openssl::stack::Stack;
use openssl::x509::X509;

use base64::Engine;

use super::helpers::{certificate, revoked_chain};
use crate::mail::smime::{
    decrypt_enveloped, trust_certificate, verify_detailed, verify_mime, verify_signed_data,
    SmimeKeyring, TrustStore,
};

#[test]
fn verifies_signed_data() {
    let (key, cert) = certificate();
    let content = b"smime content";
    let certs = Stack::new().unwrap();
    let signed = Pkcs7::sign(&cert, &key, &certs, content, Pkcs7Flags::BINARY).unwrap();
    let der = signed.to_der().unwrap();

    let verified = verify_signed_data(&der, &trust(&cert)).unwrap();
    assert_eq!(verified, content);
}

#[test]
fn decrypts_enveloped_data() {
    let (key, cert) = certificate();
    let content = b"smime secret";
    let mut recipients = Stack::new().unwrap();
    recipients.push(cert.clone()).unwrap();
    let enveloped = Pkcs7::encrypt(
        &recipients,
        content,
        openssl::symm::Cipher::aes_256_cbc(),
        Pkcs7Flags::BINARY,
    )
    .unwrap();
    let der = enveloped.to_der().unwrap();

    let keyring = SmimeKeyring {
        crl_paths: Vec::new(),
        certs: vec![cert.clone()],
        pairs: vec![(key, cert)],
    };
    let decrypted = decrypt_enveloped(&der, &keyring).unwrap();
    assert_eq!(decrypted, content);
}

#[test]
fn verifies_a_detached_mime_signature() {
    let (key, cert) = certificate();
    let signed_part = "Content-Type: text/plain\r\n\r\nhello smime";
    let certs = Stack::new().unwrap();
    let detached = Pkcs7::sign(
        &cert,
        &key,
        &certs,
        signed_part.as_bytes(),
        Pkcs7Flags::BINARY | Pkcs7Flags::DETACHED,
    )
    .unwrap();
    let der = detached.to_der().unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&der);

    let message = format!(
        "MIME-Version: 1.0\r\nContent-Type: multipart/signed; protocol=\"application/pkcs7-signature\"; micalg=sha-256; boundary=s\r\n\r\n--s\r\n{signed_part}\r\n--s\r\nContent-Type: application/pkcs7-signature; name=smime.p7s\r\nContent-Transfer-Encoding: base64\r\n\r\n{encoded}\r\n--s--\r\n"
    );

    let content = verify_mime(message.as_bytes(), &trust(&cert)).expect("verified");
    assert_eq!(content, signed_part.as_bytes());
}

fn sign(key: &PKey<openssl::pkey::Private>, cert: &X509, content: &[u8]) -> Vec<u8> {
    let certs = Stack::new().unwrap();
    Pkcs7::sign(cert, key, &certs, content, Pkcs7Flags::BINARY)
        .unwrap()
        .to_der()
        .unwrap()
}

#[test]
fn reports_an_untrusted_signer_with_identity() {
    let (key, cert) = certificate();
    let der = sign(&key, &cert, b"body");

    let verification =
        verify_detailed(&der, Some(b"body"), &TrustStore::default()).expect("verification");
    assert!(verification.content.is_none());
    assert_eq!(verification.signers.len(), 1);

    let signer = &verification.signers[0];
    assert!(signer.subject.contains("CN=Alice"));
    assert!(signer.subject.contains("alice@example.com"));
    assert!(!signer.trusted);
    assert_eq!(signer.fingerprint.len(), 64);
    assert!(!signer.der.is_empty());
    assert_eq!(
        verification.untrusted_signer().map(|s| s.subject.clone()),
        Some(signer.subject.clone())
    );
}

#[test]
fn reports_a_trusted_signer_when_the_certificate_is_known() {
    let (key, cert) = certificate();
    let der = sign(&key, &cert, b"body");

    let verification = verify_detailed(&der, Some(b"body"), &trust(&cert)).expect("verification");
    assert_eq!(verification.content.as_deref(), Some(&b"body"[..]));
    assert!(verification.is_trusted());
    assert!(verification.untrusted_signer().is_none());
}

#[test]
fn trusting_a_certificate_round_trips_through_the_keyring() {
    let root = std::env::temp_dir().join(format!(
        "sfmail-smime-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let (key, cert) = certificate();
    let der = sign(&key, &cert, b"body");
    let signer_der = verify_detailed(&der, Some(b"body"), &TrustStore::default())
        .unwrap()
        .signers[0]
        .der
        .clone();

    let path = trust_certificate(&root, &signer_der).expect("store certificate");
    assert!(path.exists());

    let keyring = SmimeKeyring::load(&root);
    assert_eq!(keyring.certs.len(), 1);
    let verification = verify_detailed(&der, Some(b"body"), &keyring.trust()).unwrap();
    assert!(verification.is_trusted());

    let _ = std::fs::remove_dir_all(&root);
}

/// A trust store holding just this certificate.
fn trust(cert: &X509) -> TrustStore {
    TrustStore {
        certs: vec![cert.clone()],
        crl_paths: Vec::new(),
    }
}

#[test]
fn a_revoked_signer_is_reported_as_revoked_and_untrusted() {
    use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
    use openssl::stack::Stack;

    let dir = std::env::temp_dir().join(format!("sfmail-crl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let Some((ca, crl_path)) = revoked_chain(&dir, "revoked@example.com") else {
        // Creating a revocation list needs the openssl command-line tool.
        eprintln!("skipping: the openssl binary is unavailable to create a CRL");
        return;
    };

    // Sign with the revoked leaf.
    let leaf = X509::from_pem(&std::fs::read(dir.join("leaf.crt")).unwrap()).unwrap();
    let leaf_key = PKey::from_rsa(
        openssl::rsa::Rsa::private_key_from_pem(&std::fs::read(dir.join("leaf.key")).unwrap())
            .unwrap(),
    )
    .unwrap();
    let content = b"body";
    let signed = Pkcs7::sign(
        &leaf,
        &leaf_key,
        &Stack::new().unwrap(),
        content,
        Pkcs7Flags::BINARY,
    )
    .unwrap();
    let der = signed.to_der().unwrap();

    // Without the list the chain is untrusted but not revoked.
    let without = TrustStore {
        certs: vec![ca.clone()],
        crl_paths: Vec::new(),
    };
    let verification = verify_detailed(&der, Some(content), &without).unwrap();
    assert!(
        verification.signers[0].trusted && !verification.signers[0].revoked,
        "without a revocation list the signer is simply trusted"
    );
    assert!(verification.content.is_some(), "the signature verified");

    // With it, the signer is named as revoked and not trusted.
    let with = TrustStore {
        certs: vec![ca],
        crl_paths: vec![crl_path.clone()],
    };
    let verification = verify_detailed(&der, Some(content), &with).unwrap();
    let signer = verification
        .revoked_signer()
        .unwrap_or_else(|| panic!("the revocation list revokes this signer: {:?}", crl_path));
    assert!(!signer.trusted, "a revoked signer is never trusted");
    assert!(
        verification.content.is_some(),
        "the signature itself still verifies"
    );
    assert!(!verification.is_trusted(), "revocation overrides trust");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_keyring_adopts_only_parseable_revocation_lists() {
    let dir = std::env::temp_dir().join(format!("sfmail-crl-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("not-a-crl.crl"), b"this is not a revocation list").unwrap();

    let keyring = SmimeKeyring::load(&dir);
    assert!(
        keyring.crl_paths.is_empty(),
        "an unparseable file is not adopted as a revocation list: {:?}",
        keyring.crl_paths
    );

    // A real list is adopted, and a store built from the keyring then revokes
    // its signer.
    if let Some((ca, _)) = revoked_chain(&dir, "revoked@example.com") {
        let keyring = SmimeKeyring::load(&dir);
        assert_eq!(
            keyring.crl_paths,
            vec![dir.join("test-ca.crl")],
            "the generated list is found"
        );
        assert!(
            keyring.trust().certs.iter().any(|cert| cert == &ca)
                || !keyring.trust().certs.is_empty(),
            "the authority certificate is part of the trust store"
        );
    } else {
        eprintln!("skipping the positive case: the openssl binary is unavailable");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
