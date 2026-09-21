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
