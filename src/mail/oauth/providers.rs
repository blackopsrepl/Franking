/*! Provider presets and the provider registry. */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProviderKind {
    Gmail,
    Outlook,
}

#[derive(Debug, Clone, Copy)]
pub struct OAuthProvider {
    pub kind: OAuthProviderKind,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub imap_security: &'static str,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_security: &'static str,
    pub auth_endpoint: &'static str,
    pub token_endpoint: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub struct OAuthAuthorization {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<String>,
    pub scopes: String,
}

pub const GMAIL_PROVIDER: OAuthProvider = OAuthProvider {
    kind: OAuthProviderKind::Gmail,
    provider_kind: "gmail",
    display_name: "Gmail",
    imap_host: "imap.gmail.com",
    imap_port: 993,
    imap_security: "tls",
    smtp_host: "smtp.gmail.com",
    smtp_port: 587,
    smtp_security: "starttls",
    auth_endpoint: "https://accounts.google.com/o/oauth2/v2/auth",
    token_endpoint: "https://oauth2.googleapis.com/token",
    scopes: &["https://mail.google.com/"],
};

pub const OUTLOOK_PROVIDER: OAuthProvider = OAuthProvider {
    kind: OAuthProviderKind::Outlook,
    provider_kind: "outlook",
    display_name: "Outlook",
    imap_host: "outlook.office365.com",
    imap_port: 993,
    imap_security: "tls",
    smtp_host: "smtp.office365.com",
    smtp_port: 587,
    smtp_security: "starttls",
    auth_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
    token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
    scopes: &[
        "offline_access",
        "https://outlook.office.com/IMAP.AccessAsUser.All",
        "https://outlook.office.com/SMTP.Send",
    ],
};

pub fn provider_by_kind(kind: &str) -> Option<&'static OAuthProvider> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "gmail" => Some(&GMAIL_PROVIDER),
        "outlook" => Some(&OUTLOOK_PROVIDER),
        _ => None,
    }
}
