/*! Known-provider presets.

These avoid network discovery for the major providers and encode the correct
authentication mode (OAuth2 for Google and Microsoft, app password for iCloud).
*/

use super::model::{DiscoveredConfig, DiscoverySource};

pub(super) fn preset_for_domain(domain: &str, email: &str) -> Option<DiscoveredConfig> {
    let (provider, imap_host, smtp_host, auth_mode) = match domain.to_ascii_lowercase().as_str() {
        "gmail.com" | "googlemail.com" => ("gmail", "imap.gmail.com", "smtp.gmail.com", "oauth2"),
        "icloud.com" | "me.com" | "mac.com" => (
            "icloud",
            "imap.mail.me.com",
            "smtp.mail.me.com",
            "app_password",
        ),
        "outlook.com" | "hotmail.com" | "live.com" => (
            "outlook",
            "outlook.office365.com",
            "smtp.office365.com",
            "oauth2",
        ),
        _ => return None,
    };

    Some(DiscoveredConfig {
        provider_kind: provider.to_string(),
        imap_host: imap_host.to_string(),
        imap_port: 993,
        imap_security: "tls".to_string(),
        smtp_host: smtp_host.to_string(),
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        username: email.to_string(),
        auth_mode: auth_mode.to_string(),
        source: DiscoverySource::Preset,
    })
}
