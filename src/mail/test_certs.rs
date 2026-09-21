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

/// A certificate authority, a certificate it issued, and a revocation list that
/// revokes that certificate, all written into `dir`.
///
/// Returns `None` when the `openssl` command-line tool is unavailable, since
/// the crate has no API for *creating* a revocation list. The caller reports
/// the skip rather than silently passing.
pub(crate) fn revoked_chain(
    dir: &std::path::Path,
    email: &str,
) -> Option<(X509, std::path::PathBuf)> {
    use std::process::Command;

    let openssl = |args: &[&str]| -> Option<()> {
        let status = Command::new("openssl")
            .args(args)
            .current_dir(dir)
            .status()
            .ok()?;
        status.success().then_some(())
    };

    openssl(&["version"])?;
    let leaf_serial = "1000";

    // The authority.
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        "ca.key",
        "-out",
        "ca.crt",
        "-subj",
        "/CN=Test CA",
        "-days",
        "30",
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
    ])?;

    // The certificate it issues.
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        "leaf.key",
        "-out",
        "leaf.csr",
        "-subj",
        &format!("/CN=Leaf/emailAddress={email}"),
    ])?;
    let extensions = dir.join("leaf.ext");
    std::fs::write(
        &extensions,
        // A verifier builds the chain from the key identifiers, so both the
        // leaf and the authority need them.
        "basicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\n",
    )
    .ok()?;
    openssl(&[
        "x509",
        "-req",
        "-in",
        "leaf.csr",
        "-CA",
        "ca.crt",
        "-CAkey",
        "ca.key",
        "-set_serial",
        leaf_serial,
        "-days",
        "30",
        "-out",
        "leaf.crt",
        "-extfile",
        "leaf.ext",
    ])?;

    // The revocation list revoking it.
    let index = dir.join("ca.index");
    std::fs::write(&index, "").ok()?;
    // `openssl ca -gencrl` reads the CRL number from this file; it must exist.
    std::fs::write(dir.join("ca.crlnumber"), "1000\n").ok()?;
    let openssl_cnf = dir.join("openssl.cnf");
    std::fs::write(
        &openssl_cnf,
        format!(
            "[ca]\ndefault_ca=CA_default\n[CA_default]\ndatabase={}\ncrlnumber={}\ndefault_md=sha256\ndefault_crl_days=30\n",
            index.display(),
            dir.join("ca.crlnumber").display()
        ),
    )
    .ok()?;
    // The CRL needs the revoked serial in the database.
    openssl(&[
        "ca",
        "-config",
        "openssl.cnf",
        "-revoke",
        "leaf.crt",
        "-keyfile",
        "ca.key",
        "-cert",
        "ca.crt",
        "-batch",
    ])?;
    openssl(&[
        "ca",
        "-config",
        "openssl.cnf",
        "-gencrl",
        "-keyfile",
        "ca.key",
        "-cert",
        "ca.crt",
        "-out",
        "test-ca.crl",
    ])?;

    let ca = X509::from_pem(&std::fs::read(dir.join("ca.crt")).ok()?).ok()?;
    Some((ca, dir.join("test-ca.crl")))
}
