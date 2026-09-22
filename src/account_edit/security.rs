/*! How a connection to a mail server is protected. */

/// How a connection to a mail server is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionSecurity {
    /// TLS from the first byte, the usual choice on 993 and 465.
    #[default]
    Tls,
    /// Cleartext that is upgraded with STARTTLS, the usual choice on 143 and 587.
    StartTls,
    /// No protection; only for a local test server.
    Plain,
}

impl ConnectionSecurity {
    pub fn label(self) -> &'static str {
        match self {
            ConnectionSecurity::Tls => "TLS",
            ConnectionSecurity::StartTls => "STARTTLS",
            ConnectionSecurity::Plain => "plain",
        }
    }

    /// The stored name, matching `AccountRecord`'s security fields.
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionSecurity::Tls => "tls",
            ConnectionSecurity::StartTls => "starttls",
            ConnectionSecurity::Plain => "plain",
        }
    }

    /// The security a record was stored with.
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("tls").trim().to_ascii_lowercase().as_str() {
            "starttls" => ConnectionSecurity::StartTls,
            "plain" | "none" => ConnectionSecurity::Plain,
            _ => ConnectionSecurity::Tls,
        }
    }

    /// The next choice in the cycle.
    pub fn next(self) -> Self {
        match self {
            ConnectionSecurity::Tls => ConnectionSecurity::StartTls,
            ConnectionSecurity::StartTls => ConnectionSecurity::Plain,
            ConnectionSecurity::Plain => ConnectionSecurity::Tls,
        }
    }

    /// The usual choice for a port, so the common cases need no attention.
    pub fn for_port(port: u16) -> Self {
        match port {
            993 | 465 => ConnectionSecurity::Tls,
            // A submission or IMAP port without implicit TLS, and the local
            // test ports, which speak cleartext.
            _ => ConnectionSecurity::StartTls,
        }
    }
}
