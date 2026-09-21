/*! S/MIME round-trip tests using a generated certificate. */

use openssl::asn1::{Asn1Integer, Asn1Time};
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::stack::Stack;
use openssl::x509::{X509NameBuilder, X509};

use base64::Engine;

use super::{decrypt_enveloped, verify_mime, verify_signed_data, SmimeKeyring};

fn certificate() -> (PKey<openssl::pkey::Private>, X509) {
    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "Alice").unwrap();
    name.append_entry_by_text("emailAddress", "alice@example.com")
        .unwrap();
    let name = name.build();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let serial = Asn1Integer::from_bn(&BigNum::from_u32(42).unwrap()).unwrap();
    builder.set_serial_number(&serial).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(30).unwrap())
        .unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();
    (key, builder.build())
}

#[test]
fn verifies_signed_data() {
    let (key, cert) = certificate();
    let content = b"smime content";
    let certs = Stack::new().unwrap();
    let signed = Pkcs7::sign(&cert, &key, &certs, content, Pkcs7Flags::BINARY).unwrap();
    let der = signed.to_der().unwrap();

    let verified = verify_signed_data(&der, std::slice::from_ref(&cert)).unwrap();
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

    let content = verify_mime(message.as_bytes(), std::slice::from_ref(&cert)).expect("verified");
    assert_eq!(content, signed_part.as_bytes());
}
