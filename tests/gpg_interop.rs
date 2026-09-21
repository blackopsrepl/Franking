//! OpenPGP interoperability against GnuPG.
//!
//! Every case needs a real `gpg`; the suite returns early when it is absent so
//! that ordinary CI without GnuPG stays green.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use solverforge_mail::mail::pgp::{self, Keyring};
use solverforge_mail::mail::pgp_mime;
use solverforge_mail::mail::pgp_sign;
use solverforge_mail::mail::service::SendOptions;

const UID: &str = "Interop Test <interop@example.com>";

/// A temporary GnuPG home plus our keyring directory.
struct Fixture {
    home: PathBuf,
    keys: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Option<Self> {
        if Command::new("gpg").arg("--version").output().is_err() {
            eprintln!("skipping: gpg is not installed");
            return None;
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let root =
            std::env::temp_dir().join(format!("sfmail-gpg-{}-{stamp:x}", std::process::id()));
        let home = root.join("gnupg");
        let keys = root.join("keys");
        std::fs::create_dir_all(&home).ok()?;
        std::fs::create_dir_all(&keys).ok()?;
        std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).ok()?;

        let (secret, public) = pgp::generate_keypair(UID).ok()?;
        pgp::write_keypair(&keys, "interop", &secret, &public).ok()?;

        let armored = secret.to_armored_string(Default::default()).ok()?;
        let secret_path = root.join("secret.asc");
        std::fs::write(&secret_path, armored).ok()?;

        let fixture = Fixture { home, keys, root };
        fixture.gpg(&["--import", secret_path.to_str()?]).ok()?;
        Some(fixture)
    }

    fn gpg(&self, args: &[&str]) -> std::io::Result<std::process::Output> {
        let output = Command::new("gpg")
            .arg("--homedir")
            .arg(&self.home)
            .arg("--batch")
            .arg("--yes")
            .args(args)
            .output()?;
        if !output.status.success() {
            panic!(
                "gpg {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output)
    }

    fn keyring(&self) -> Keyring {
        Keyring::load(&self.keys)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

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
        "To: interop@example.com",
        "Subject: Interop",
        "",
        "Body for GnuPG interop.",
    ]
    .join("\r\n")
    .into_bytes()
}

/// The substring of `body` between two markers, inclusive.
fn between(body: &[u8], begin: &str, end: &str) -> Option<Vec<u8>> {
    let text = String::from_utf8_lossy(body);
    let start = text.find(begin)?;
    let stop = text[start..].find(end)? + start + end.len();
    Some(text[start..stop].as_bytes().to_vec())
}

#[test]
fn our_detached_signature_verifies_with_gpg() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();
    let entity = sample_message();

    let armored = pgp_sign::sign_detached(&entity, &keyring.secret[0], "").expect("sign");
    let data = fx.file("data.txt");
    let sig = fx.file("sig.asc");
    std::fs::write(&data, &entity).unwrap();
    std::fs::write(&sig, &armored).unwrap();

    fx.gpg(&["--verify", sig.to_str().unwrap(), data.to_str().unwrap()])
        .expect("verify");
}

#[test]
fn our_encrypted_message_decrypts_with_gpg() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();

    let armored = pgp_sign::encrypt_to_keys(&sample_message(), &keyring.public).expect("encrypt");
    let path = fx.file("message.asc");
    std::fs::write(&path, &armored).unwrap();

    let output = fx.gpg(&["--decrypt", path.to_str().unwrap()]).expect("gpg");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Body for GnuPG interop."));
}

#[test]
fn gpg_detached_signature_verifies_with_our_keyring() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();
    let entity = sample_message();

    let data = fx.file("gpg-data.txt");
    let sig = fx.file("gpg-sig.asc");
    std::fs::write(&data, &entity).unwrap();
    fx.gpg(&[
        "--armor",
        "--detach-sign",
        "--local-user",
        UID,
        "--output",
        sig.to_str().unwrap(),
        data.to_str().unwrap(),
    ])
    .expect("gpg");

    let signature = std::fs::read(&sig).unwrap();
    let fingerprints = pgp::verify_detached(&entity, &signature, &keyring.public);
    assert_eq!(fingerprints.len(), 1);
}

#[test]
fn gpg_encrypted_message_decrypts_with_our_keyring() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();
    let entity = sample_message();

    let path = fx.file("gpg-message.asc");
    let plain = fx.file("gpg-plain.txt");
    std::fs::write(&plain, &entity).unwrap();
    fx.gpg(&[
        "--armor",
        "--trust-model",
        "always",
        "--recipient",
        UID,
        "--output",
        path.to_str().unwrap(),
        "--encrypt",
        plain.to_str().unwrap(),
    ])
    .expect("gpg");
    let armored = std::fs::read(&path).unwrap();

    let decrypted = pgp::decrypt_inline(&String::from_utf8_lossy(&armored), &keyring.secret, "")
        .expect("decrypt");
    assert_eq!(decrypted, entity);
}

#[test]
fn our_signed_pgp_mime_verifies_with_gpg() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();

    let wrapped = pgp_mime::wrap(&sample_message(), &options(true, false), &keyring).expect("wrap");
    let (part, signature) = split_signed(&wrapped);

    let part_path = fx.file("part.txt");
    let sig_path = fx.file("part-sig.asc");
    std::fs::write(&part_path, &part).unwrap();
    std::fs::write(&sig_path, &signature).unwrap();

    fx.gpg(&[
        "--verify",
        sig_path.to_str().unwrap(),
        part_path.to_str().unwrap(),
    ])
    .expect("gpg");
}

#[test]
fn our_encrypted_pgp_mime_decrypts_with_gpg() {
    let Some(fx) = Fixture::new() else { return };
    let keyring = fx.keyring();

    let wrapped = pgp_mime::wrap(&sample_message(), &options(false, true), &keyring).expect("wrap");
    let message = between(
        &wrapped,
        "-----BEGIN PGP MESSAGE-----",
        "-----END PGP MESSAGE-----",
    )
    .expect("armored ciphertext");

    let path = fx.file("cipher.asc");
    std::fs::write(&path, &message).unwrap();
    let output = fx.gpg(&["--decrypt", path.to_str().unwrap()]).expect("gpg");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Body for GnuPG interop."));
}

/// The first signed part and the armored signature from a multipart/signed body.
fn split_signed(wrapped: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let signature = between(
        wrapped,
        "-----BEGIN PGP SIGNATURE-----",
        "-----END PGP SIGNATURE-----",
    )
    .expect("armored signature");
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
    (wrapped[first..stop].to_vec(), signature)
}
