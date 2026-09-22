/*! S/MIME verification and decryption for a loaded message. */

use crate::mail::model::{BodyDocument, MessageDocument, PartBody};
use crate::mail::security::Protection;
use crate::mail::smime::{self, SmimeKeyring, SmimeSigner, SmimeVerification};

use super::pgp::keys_dir;

/// Result of S/MIME processing for the message view.
pub(super) struct SmimeOutcome {
    /// Status line shown under the message.
    pub status: String,
    /// Signer certificate that is not yet trusted, if any.
    pub untrusted: Option<SmimeSigner>,
}

/// Verify or decrypt S/MIME on a message.
pub(super) fn process_smime(message: &mut MessageDocument) -> Option<SmimeOutcome> {
    let protection = message.protection()?;
    if !matches!(
        protection,
        Protection::SmimeSigned | Protection::SmimeEncrypted
    ) {
        return None;
    }

    let keyring = SmimeKeyring::load(&keys_dir());

    match protection {
        Protection::SmimeSigned => {
            let verification = message
                .raw
                .as_deref()
                .and_then(|raw| smime::verify_mime_detailed(raw, &keyring.trust()))
                .or_else(|| {
                    let der = find_pkcs7(message)?;
                    smime::verify_detailed(&der, None, &keyring.trust())
                })?;
            Some(signature_outcome(verification))
        }
        Protection::SmimeEncrypted => {
            let der = find_pkcs7(message)?;
            Some(match smime::decrypt_enveloped(&der, &keyring) {
                Some(data) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    message.body = BodyDocument::from_plain(&text);
                    message.plain_body = Some(text);
                    SmimeOutcome {
                        status: "S/MIME encrypted message decrypted".to_string(),
                        untrusted: None,
                    }
                }
                None => SmimeOutcome {
                    status: "S/MIME encrypted (no usable key)".to_string(),
                    untrusted: None,
                },
            })
        }
        _ => None,
    }
}

/// Turn a verification into a status line plus any signer to offer for trust.
pub(super) fn signature_outcome(verification: SmimeVerification) -> SmimeOutcome {
    let Some(signer) = verification.signers.first() else {
        return SmimeOutcome {
            status: "S/MIME signature could not be verified".to_string(),
            untrusted: None,
        };
    };

    if let Some(revoked) = verification.revoked_signer() {
        return SmimeOutcome {
            status: format!(
                "S/MIME signature from a REVOKED certificate — {} (do not trust)",
                revoked.subject
            ),
            untrusted: None,
        };
    }

    if verification.is_trusted() {
        return SmimeOutcome {
            status: format!("S/MIME signature valid — {} (trusted)", signer.subject),
            untrusted: None,
        };
    }

    if verification.content.is_none() {
        return SmimeOutcome {
            status: format!(
                "S/MIME signature could not be verified — {} (press T to trust)",
                signer.subject
            ),
            untrusted: verification.untrusted_signer().cloned(),
        };
    }

    SmimeOutcome {
        status: format!(
            "S/MIME signature from an untrusted certificate — {} (press T to trust)",
            signer.subject
        ),
        untrusted: verification.untrusted_signer().cloned(),
    }
}

/// First binary PKCS#7 part in the MIME tree.
fn find_pkcs7(message: &MessageDocument) -> Option<Vec<u8>> {
    for part in &message.parts {
        let mut found = None;
        part.walk(&mut |part| {
            if found.is_none() && part.content_type.to_ascii_lowercase().contains("pkcs7") {
                if let PartBody::Binary(bytes) = &part.body {
                    found = Some(bytes.clone());
                }
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}
