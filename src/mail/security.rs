/*! Message authentication verdicts.
Receiving servers record SPF, DKIM, and DMARC results in
`Authentication-Results`; parse and surface them rather than re-running DNS. */

/// SPF/DKIM/DMARC results reported for a message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthenticationVerdicts {
    pub spf: Option<String>,
    pub dkim: Option<String>,
    pub dmarc: Option<String>,
}

impl AuthenticationVerdicts {
    pub fn is_empty(&self) -> bool {
        self.spf.is_none() && self.dkim.is_none() && self.dmarc.is_none()
    }

    /// Human-readable summary, e.g. `SPF pass · DKIM pass · DMARC pass`.
    pub fn summary(&self) -> Option<String> {
        let parts = [
            self.spf.as_deref().map(|value| format!("SPF {value}")),
            self.dkim.as_deref().map(|value| format!("DKIM {value}")),
            self.dmarc.as_deref().map(|value| format!("DMARC {value}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" \u{00b7} "))
        }
    }
}

/// Parse one or more `Authentication-Results` header values.
pub fn parse_authentication_results(values: &[&str]) -> AuthenticationVerdicts {
    let mut verdicts = AuthenticationVerdicts::default();
    for value in values {
        for clause in value.split(';') {
            let clause = clause.trim();
            if let Some(rest) = clause.strip_prefix("spf=") {
                verdicts.spf = Some(result_token(rest));
            } else if let Some(rest) = clause.strip_prefix("dkim=") {
                verdicts.dkim = Some(result_token(rest));
            } else if let Some(rest) = clause.strip_prefix("dmarc=") {
                verdicts.dmarc = Some(result_token(rest));
            }
        }
    }
    verdicts
}

fn result_token(value: &str) -> String {
    value
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('(')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::parse_authentication_results;

    #[test]
    fn parses_spf_dkim_dmarc() {
        let value = "mx.example.com; spf=pass (sender ok) smtp.mailfrom=a@b.com; \
                     dkim=pass header.d=b.com; dmarc=pass header.from=b.com";
        let verdicts = parse_authentication_results(&[value]);

        assert_eq!(verdicts.spf.as_deref(), Some("pass"));
        assert_eq!(verdicts.dkim.as_deref(), Some("pass"));
        assert_eq!(verdicts.dmarc.as_deref(), Some("pass"));
        assert_eq!(
            verdicts.summary().as_deref(),
            Some("SPF pass \u{00b7} DKIM pass \u{00b7} DMARC pass")
        );
    }

    #[test]
    fn reports_failures_and_missing_results() {
        let verdicts = parse_authentication_results(&["mx; spf=fail; dkim=none"]);
        assert_eq!(verdicts.spf.as_deref(), Some("fail"));
        assert_eq!(verdicts.dkim.as_deref(), Some("none"));
        assert!(verdicts.dmarc.is_none());

        let empty = parse_authentication_results(&[]);
        assert!(empty.is_empty());
        assert!(empty.summary().is_none());
    }

    #[test]
    fn later_headers_override_earlier() {
        let verdicts = parse_authentication_results(&["mx; spf=fail", "mx; spf=pass"]);
        assert_eq!(verdicts.spf.as_deref(), Some("pass"));
    }
}

/// Cryptographic protection detected on a message's MIME structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    PgpSigned,
    PgpEncrypted,
    SmimeSigned,
    SmimeEncrypted,
}

impl Protection {
    pub fn label(self) -> &'static str {
        match self {
            Self::PgpSigned => "PGP signed",
            Self::PgpEncrypted => "PGP encrypted",
            Self::SmimeSigned => "S/MIME signed",
            Self::SmimeEncrypted => "S/MIME encrypted",
        }
    }
}

/// Detect PGP/MIME or S/MIME protection from the MIME tree. This identifies
/// the structure only; it does not verify signatures or decrypt.
pub fn detect_protection(document: &crate::mail::model::MessageDocument) -> Option<Protection> {
    let mut top = None;
    let mut content_types = Vec::new();
    for part in &document.parts {
        top = Some(part.content_type.to_ascii_lowercase());
        part.walk(&mut |part| content_types.push(part.content_type.to_ascii_lowercase()));
    }
    let top = top?;

    if top.starts_with("multipart/signed") {
        if content_types.iter().any(|t| t.contains("pgp-signature")) {
            return Some(Protection::PgpSigned);
        }
        if content_types.iter().any(|t| t.contains("pkcs7-signature")) {
            return Some(Protection::SmimeSigned);
        }
    }
    if top.starts_with("multipart/encrypted") {
        if content_types.iter().any(|t| t.contains("pgp-encrypted")) {
            return Some(Protection::PgpEncrypted);
        }
        if content_types.iter().any(|t| t.contains("pkcs7-mime")) {
            return Some(Protection::SmimeEncrypted);
        }
    }
    if top.contains("pkcs7-mime") {
        return Some(Protection::SmimeEncrypted);
    }
    None
}

#[cfg(test)]
mod protection_tests {
    use super::{detect_protection, Protection};
    use crate::mail::mime;

    #[test]
    fn detects_pgp_signed_and_encrypted() {
        let signed = b"MIME-Version: 1.0\r\nContent-Type: multipart/signed; protocol=\"application/pgp-signature\"; boundary=s\r\n\r\n--s\r\nContent-Type: text/plain\r\n\r\nhello\r\n--s\r\nContent-Type: application/pgp-signature\r\n\r\nsig\r\n--s--";
        assert_eq!(
            detect_protection(&mime::parse_message(signed).unwrap()),
            Some(Protection::PgpSigned)
        );

        let encrypted = b"MIME-Version: 1.0\r\nContent-Type: multipart/encrypted; protocol=\"application/pgp-encrypted\"; boundary=e\r\n\r\n--e\r\nContent-Type: application/pgp-encrypted\r\n\r\nVersion: 1\r\n--e\r\nContent-Type: application/octet-stream\r\n\r\ncipher\r\n--e--";
        assert_eq!(
            detect_protection(&mime::parse_message(encrypted).unwrap()),
            Some(Protection::PgpEncrypted)
        );
    }

    #[test]
    fn detects_smime_and_ignores_plain() {
        let smime = b"MIME-Version: 1.0\r\nContent-Type: application/pkcs7-mime; smime-type=enveloped-data; name=smime.p7m\r\n\r\nbinary";
        assert_eq!(
            detect_protection(&mime::parse_message(smime).unwrap()),
            Some(Protection::SmimeEncrypted)
        );

        let plain = b"From: a@b\r\nSubject: hi\r\n\r\nplain body";
        assert_eq!(
            detect_protection(&mime::parse_message(plain).unwrap()),
            None
        );
    }
}
