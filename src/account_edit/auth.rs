/*! Authentication method for a new or edited account. */

/// Authentication method for a new or edited account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    #[default]
    Password,
    /// Google OAuth2 (Gmail / Google Workspace).
    GmailOAuth,
    /// Microsoft OAuth2 (Outlook / Microsoft 365).
    OutlookOAuth,
}

impl AuthMode {
    pub fn label(self) -> &'static str {
        match self {
            AuthMode::Password => "password / app password",
            AuthMode::GmailOAuth => "Google OAuth2",
            AuthMode::OutlookOAuth => "Microsoft OAuth2",
        }
    }

    pub fn provider_kind(self) -> Option<&'static str> {
        match self {
            AuthMode::Password => None,
            AuthMode::GmailOAuth => Some("gmail"),
            AuthMode::OutlookOAuth => Some("outlook"),
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            AuthMode::Password => AuthMode::GmailOAuth,
            AuthMode::GmailOAuth => AuthMode::OutlookOAuth,
            AuthMode::OutlookOAuth => AuthMode::Password,
        }
    }
}
