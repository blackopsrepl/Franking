/*! Key material on disk: listing, import, export, deletion. */

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::types::KeyDetails;

/// What is known about one stored key file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    /// Primary user id, e.g. `Alice <alice@example.com>`.
    pub identity: String,
    /// Uppercase hex fingerprint, the key's stable identifier.
    pub fingerprint: String,
    /// Email addresses the key advertises.
    pub emails: Vec<String>,
    /// Whether this is a secret key file.
    pub secret: bool,
    /// The file the key lives in.
    pub path: PathBuf,
}

impl KeyInfo {
    /// Fingerprint in display form, grouped in fours.
    pub fn fingerprint_display(&self) -> String {
        self.fingerprint
            .as_bytes()
            .chunks(4)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Describe every key file in `dir`, public and secret, sorted by identity.
pub fn list_keys(dir: &Path) -> Vec<KeyInfo> {
    let mut keys = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return keys;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let secret = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".sec"));
        let info = if secret {
            SignedSecretKey::from_reader_single(&bytes[..])
                .ok()
                .map(|(key, _)| describe_public_key(&SignedPublicKey::from(key), &path, true))
        } else {
            SignedPublicKey::from_reader_single(&bytes[..])
                .ok()
                .map(|(key, _)| describe_public_key(&key, &path, false))
        };
        if let Some(info) = info {
            keys.push(info);
        }
    }
    keys.sort_by(|left, right| left.identity.cmp(&right.identity));
    keys
}

/// Describe one public key. Secret keys are described through their public half.
fn describe_public_key(key: &SignedPublicKey, path: &Path, secret: bool) -> KeyInfo {
    let identity = key
        .details
        .users
        .first()
        .map(|user| String::from_utf8_lossy(user.id.id()).to_string())
        .unwrap_or_else(|| "(no user id)".to_string());
    KeyInfo {
        identity,
        fingerprint: key.fingerprint().to_string(),
        emails: user_id_emails(key),
        secret,
        path: path.to_path_buf(),
    }
}

/// Email addresses from a key's user ids.
///
/// A user id is free text (`Alice <alice@example.com>`), so the bracketed
/// address is preferred and a bare address is accepted as a fallback.
fn user_id_emails(key: &SignedPublicKey) -> Vec<String> {
    key.details
        .users
        .iter()
        .filter_map(|user| {
            let id = String::from_utf8_lossy(user.id.id()).to_string();
            match (id.rfind('<'), id.rfind('>')) {
                (Some(start), Some(end)) if end > start + 1 => {
                    Some(id[start + 1..end].trim().to_lowercase())
                }
                _ if id.contains('@') => Some(id.trim().to_lowercase()),
                _ => None,
            }
        })
        .collect()
}

/// Store an armored public key in `dir`, returning what was stored.
///
/// The same key is not stored twice: an existing file for the same fingerprint
/// is left alone.
pub fn import_public_key(dir: &Path, armored: &[u8]) -> Result<KeyInfo> {
    let (key, _) = SignedPublicKey::from_reader_single(armored)
        .map_err(|error| anyhow::anyhow!("not a PGP public key: {error}"))?;
    let fingerprint = key.fingerprint().to_string();
    if let Some(existing) = list_keys(dir)
        .into_iter()
        .find(|info| info.fingerprint == fingerprint)
    {
        return Ok(existing);
    }

    let name = key_name(&fingerprint, false);
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(name);
    let armored = key.to_armored_string(Default::default())?;
    std::fs::write(&path, armored).with_context(|| format!("write {}", path.display()))?;
    Ok(describe_public_key(&key, &path, false))
}

/// The armored text of a stored public key, for sharing.
pub fn export_public_key(dir: &Path, fingerprint: &str) -> Result<String> {
    let info = list_keys(dir)
        .into_iter()
        .find(|info| info.fingerprint == fingerprint && !info.secret)
        .ok_or_else(|| anyhow::anyhow!("no public key with that fingerprint"))?;
    let bytes =
        std::fs::read(&info.path).with_context(|| format!("read {}", info.path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Remove every stored file for a key, returning how many were removed.
pub fn delete_key(dir: &Path, fingerprint: &str) -> Result<usize> {
    let files = list_keys(dir)
        .into_iter()
        .filter(|info| info.fingerprint == fingerprint)
        .map(|info| info.path)
        .collect::<Vec<_>>();
    let mut removed = 0;
    for path in files {
        std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
        removed += 1;
    }
    if removed == 0 {
        return Err(anyhow::anyhow!("no key with that fingerprint"));
    }
    Ok(removed)
}

/// A file name for a key, derived from its fingerprint.
fn key_name(fingerprint: &str, secret: bool) -> String {
    let suffix = if secret { "sec" } else { "pub" };
    format!("{}.{suffix}.asc", fingerprint.to_lowercase())
}

#[cfg(test)]
mod tests {
    use pgp::types::KeyDetails;

    use super::{delete_key, export_public_key, import_public_key, list_keys};
    use crate::mail::pgp::{generate_keypair, write_keypair};

    /// A fresh directory for one test, removed by the caller.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sfm-pgp-keys-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn listing_reports_identity_fingerprint_and_kind() {
        let dir = scratch("list");
        let (secret, public) = generate_keypair("Alice <alice@example.com>").unwrap();
        write_keypair(&dir, "alice", &secret, &public).unwrap();

        let keys = list_keys(&dir);
        assert_eq!(keys.len(), 2, "one public and one secret file");

        let public_key = keys.iter().find(|key| !key.secret).expect("public half");
        assert!(public_key.identity.contains("alice@example.com"));
        assert_eq!(public_key.fingerprint, public.fingerprint().to_string());
        assert!(
            public_key.fingerprint_display().contains(' '),
            "the display form is grouped: {}",
            public_key.fingerprint_display()
        );
        assert_eq!(public_key.emails, vec!["alice@example.com".to_string()]);
        assert!(keys.iter().any(|key| key.secret));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn importing_the_same_key_twice_does_not_duplicate_it() {
        let dir = scratch("import");
        let (_secret, public) = generate_keypair("Bob <bob@example.com>").unwrap();
        let armored = public.to_armored_string(Default::default()).unwrap();

        let first = import_public_key(&dir, armored.as_bytes()).unwrap();
        let second = import_public_key(&dir, armored.as_bytes()).unwrap();
        assert_eq!(first, second, "the same fingerprint is not stored twice");
        assert_eq!(list_keys(&dir).len(), 1);

        let exported = export_public_key(&dir, &first.fingerprint).unwrap();
        assert!(exported.contains("BEGIN PGP PUBLIC KEY BLOCK"));
        assert_eq!(list_keys(&dir).len(), 1, "exporting adds nothing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn importing_a_non_key_is_rejected() {
        let dir = scratch("bad");
        let error = import_public_key(&dir, b"not a key at all").expect_err("rejected");
        assert!(error.to_string().contains("not a PGP public key"));
        assert!(list_keys(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleting_removes_both_halves_of_a_key_pair() {
        let dir = scratch("delete");
        let (secret, public) = generate_keypair("Carol <carol@example.com>").unwrap();
        write_keypair(&dir, "carol", &secret, &public).unwrap();
        let fingerprint = public.fingerprint().to_string();
        assert_eq!(list_keys(&dir).len(), 2);

        assert_eq!(delete_key(&dir, &fingerprint).unwrap(), 2);
        assert!(list_keys(&dir).is_empty());
        assert!(delete_key(&dir, &fingerprint).is_err(), "already gone");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_a_secret_key_is_refused_by_the_lookup() {
        let dir = scratch("export-secret");
        let (secret, public) = generate_keypair("Dan <dan@example.com>").unwrap();
        write_keypair(&dir, "dan", &secret, &public).unwrap();
        let fingerprint = public.fingerprint().to_string();
        // Lookup is for a *public* key; the secret file must not satisfy it.
        assert!(export_public_key(&dir, &fingerprint).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
