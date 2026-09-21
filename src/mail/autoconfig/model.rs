/*! Discovered account settings. */

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredConfig {
    pub provider_kind: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub username: String,
    pub auth_mode: String,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    Preset,
    MozillaAutoconfig,
    Autodiscover,
}

/// Normalize a discovery socket type to the app's security vocabulary.
pub(super) fn normalize_security(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "starttls" => "starttls".to_string(),
        "plain" | "none" | "plaintext" => "plain".to_string(),
        _ => "tls".to_string(),
    }
}

/// Normalize a discovery authentication method to the app's auth modes.
pub(super) fn normalize_auth(value: Option<&str>) -> String {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "oauth2" => "oauth2".to_string(),
        "password-encrypted" | "app-password" => "app_password".to_string(),
        _ => "password".to_string(),
    }
}
