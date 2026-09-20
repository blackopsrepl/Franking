use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailErrorKind {
    BackendUnavailable,
    AccountNotFound,
    ConfigInvalid,
    KeyringUnavailable,
    SecretMissing,
    OauthReconfigureRequired,
    OauthRefreshFailure,
    ImapAuthRejected,
    SmtpAuthRejected,
    TlsFailure,
    TransportTimeout,
    ConnectionDropped,
    LocalMaildirFailure,
    UnsupportedFeature,
    InvalidInput,
    Io,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailError {
    pub kind: MailErrorKind,
    pub detail: String,
}

pub type MailResult<T> = Result<T, MailError>;

impl MailError {
    pub fn new(kind: MailErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    pub fn backend_unavailable(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::BackendUnavailable, detail)
    }

    pub fn account_not_found(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::AccountNotFound, detail)
    }

    pub fn config_invalid(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::ConfigInvalid, detail)
    }

    pub fn keyring_unavailable(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::KeyringUnavailable, detail)
    }

    pub fn secret_missing(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::SecretMissing, detail)
    }

    pub fn oauth_reconfigure_required(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::OauthReconfigureRequired, detail)
    }

    pub fn oauth_refresh_failure(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::OauthRefreshFailure, detail)
    }

    pub fn imap_auth_rejected(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::ImapAuthRejected, detail)
    }

    pub fn smtp_auth_rejected(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::SmtpAuthRejected, detail)
    }

    pub fn tls_failure(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::TlsFailure, detail)
    }

    pub fn transport_timeout(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::TransportTimeout, detail)
    }

    pub fn connection_dropped(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::ConnectionDropped, detail)
    }

    pub fn local_maildir_failure(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::LocalMaildirFailure, detail)
    }

    pub fn unsupported_feature(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::UnsupportedFeature, detail)
    }

    pub fn invalid_input(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::InvalidInput, detail)
    }

    pub fn io(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::Io, detail)
    }

    pub fn other(detail: impl Into<String>) -> Self {
        Self::new(MailErrorKind::Other, detail)
    }

    /// Whether the failure is transport-level, meaning a cached result is
    /// preferable offline and a pooled connection should be discarded.
    pub fn is_transport(&self) -> bool {
        matches!(
            self.kind,
            MailErrorKind::TransportTimeout | MailErrorKind::ConnectionDropped | MailErrorKind::Io
        )
    }
}

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.kind {
            MailErrorKind::BackendUnavailable => "Mail backend unavailable",
            MailErrorKind::AccountNotFound => "Account not found",
            MailErrorKind::ConfigInvalid => "Mail configuration is invalid",
            MailErrorKind::KeyringUnavailable => "Keyring is unavailable",
            MailErrorKind::SecretMissing => "Required secret is missing",
            MailErrorKind::OauthReconfigureRequired => "OAuth reconfiguration is required",
            MailErrorKind::OauthRefreshFailure => "OAuth token refresh failed",
            MailErrorKind::ImapAuthRejected => "IMAP authentication was rejected",
            MailErrorKind::SmtpAuthRejected => "SMTP authentication was rejected",
            MailErrorKind::TlsFailure => "TLS negotiation failed",
            MailErrorKind::TransportTimeout => "Mail transport timed out",
            MailErrorKind::ConnectionDropped => "Mail connection dropped",
            MailErrorKind::LocalMaildirFailure => {
                "Local maildir backend failed. This is not an authentication error"
            }
            MailErrorKind::UnsupportedFeature => "Unsupported mail feature",
            MailErrorKind::InvalidInput => "Invalid mail input",
            MailErrorKind::Io => "Mail I/O failed",
            MailErrorKind::Other => "Mail operation failed",
        };

        if self.detail.is_empty() {
            write!(f, "{prefix}")
        } else {
            write!(f, "{prefix}: {}", self.detail)
        }
    }
}

impl std::error::Error for MailError {}
