/*! Outbound S/MIME: detached CMS signatures and enveloped data. */

use anyhow::{anyhow, Context, Result};
use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
use openssl::pkey::{PKey, Private};
use openssl::stack::Stack;
use openssl::symm::Cipher;
use openssl::x509::X509;

/// DER of a detached CMS SignedData over `content`.
///
/// Detached, so the signature travels beside the content in
/// `multipart/signed` rather than inside it, which is what the inbound
/// verifier and other S/MIME clients expect.
pub fn sign_detached(content: &[u8], key: &PKey<Private>, cert: &X509) -> Result<Vec<u8>> {
    let mut chain = Stack::new().context("build the certificate stack")?;
    chain.push(cert.clone()).context("add the signer")?;

    // Authenticated attributes are included (no `NOATTR`): RFC 5652 signed
    // attributes carrying the content type and message digest are what other
    // S/MIME clients expect, and our verifier checks them.
    let pkcs7 = Pkcs7::sign(
        cert,
        key,
        &chain,
        content,
        Pkcs7Flags::BINARY | Pkcs7Flags::DETACHED,
    )
    .context("sign the message")?;
    pkcs7.to_der().context("encode the signature")
}

/// DER of a CMS EnvelopedData for the given recipient certificates.
pub fn encrypt(content: &[u8], recipients: &[X509]) -> Result<Vec<u8>> {
    if recipients.is_empty() {
        return Err(anyhow!("no recipient certificate is available"));
    }
    let mut certs = Stack::new().context("build the recipient stack")?;
    for cert in recipients {
        certs.push(cert.clone()).context("add a recipient")?;
    }
    // S/MIME clients expect a widely supported cipher, since the recipient
    // cannot negotiate one.
    let pkcs7 = Pkcs7::encrypt(&certs, content, Cipher::aes_256_cbc(), Pkcs7Flags::BINARY)
        .context("encrypt the message")?;
    pkcs7.to_der().context("encode the ciphertext")
}
