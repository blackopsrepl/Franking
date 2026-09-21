//! Certificate fixtures for the S/MIME tests (inbound and outbound).

use openssl::asn1::{Asn1Integer, Asn1Time};
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::{X509NameBuilder, X509};

pub(crate) fn certificate() -> (PKey<openssl::pkey::Private>, X509) {
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

/// A self-signed certificate for `email`, so recipient lookup has something to
/// match on besides the common name.
pub(crate) fn certificate_for(email: &str, common_name: &str) -> (PKey<Private>, X509) {
    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();

    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    name.append_entry_by_text("emailAddress", email).unwrap();
    let name = name.build();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    let serial = Asn1Integer::from_bn(&BigNum::from_u32(7).unwrap()).unwrap();
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
