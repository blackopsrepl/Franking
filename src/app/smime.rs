/*! S/MIME verification and decryption for a loaded message. */

use crate::mail::model::{BodyDocument, MessageDocument, PartBody};
use crate::mail::security::Protection;
use crate::mail::smime::{self, SmimeKeyring};

use super::pgp::keys_dir;

/// Verify or decrypt S/MIME on a message, returning a status line.
pub(super) fn process_smime(message: &mut MessageDocument) -> Option<String> {
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
            let verified = message
                .raw
                .as_deref()
                .and_then(|raw| smime::verify_mime(raw, &keyring.certs))
                .is_some()
                || find_pkcs7(message)
                    .map(|der| smime::verify_signed_data(&der, &keyring.certs).is_some())
                    .unwrap_or(false);
            Some(if verified {
                "S/MIME signature valid".to_string()
            } else {
                "S/MIME signature could not be verified".to_string()
            })
        }
        Protection::SmimeEncrypted => {
            let der = find_pkcs7(message)?;
            Some(match smime::decrypt_enveloped(&der, &keyring) {
                Some(data) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    message.body = BodyDocument::from_plain(&text);
                    message.plain_body = Some(text);
                    "S/MIME encrypted message decrypted".to_string()
                }
                None => "S/MIME encrypted (no usable key)".to_string(),
            })
        }
        _ => None,
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
