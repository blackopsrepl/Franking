/*! Transport security selection for an account. */

/// How the client secures an IMAP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    Tls,
    StartTls,
    Plain,
}

impl Security {
    pub fn normalize(value: Option<&str>, default: &str) -> Self {
        match value
            .unwrap_or(default)
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "starttls" => Security::StartTls,
            "plain" | "none" => Security::Plain,
            _ => Security::Tls,
        }
    }
}
