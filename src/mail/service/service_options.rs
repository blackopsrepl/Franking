/*! How an outgoing message should be protected. */

/// How an outgoing message should be protected.
#[derive(Debug, Clone, Default)]
pub struct SendOptions {
    /// Sign the message as PGP/MIME.
    pub sign: bool,
    /// Encrypt the message to its recipients as PGP/MIME.
    pub encrypt: bool,
    /// Sign the message with S/MIME.
    pub smime_sign: bool,
    /// Encrypt the message to its recipients with S/MIME.
    pub smime_encrypt: bool,
    /// Passphrase for the PGP signing secret key.
    pub passphrase: String,
    /// Directory holding key material; defaults to the app keyring.
    pub keys_dir: Option<std::path::PathBuf>,
}

impl SendOptions {
    /// True when PGP/MIME wrapping is requested.
    pub fn is_pgp(&self) -> bool {
        self.sign || self.encrypt
    }

    /// True when S/MIME wrapping is requested.
    pub fn is_smime(&self) -> bool {
        self.smime_sign || self.smime_encrypt
    }

    /// True when any cryptographic wrapping is requested.
    pub fn is_protected(&self) -> bool {
        self.is_pgp() || self.is_smime()
    }
}
