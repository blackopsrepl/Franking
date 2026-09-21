/*! Discovery orchestration: presets, then Mozilla autoconfig, then Autodiscover. */

use super::model::DiscoveredConfig;
use super::{autodiscover, mozilla, presets};

pub fn discover(email: &str) -> Option<DiscoveredConfig> {
    let email = email.trim();
    let domain = email.rsplit('@').next()?.trim();
    if domain.is_empty() || !domain.contains('.') {
        return None;
    }

    if let Some(config) = presets::preset_for_domain(domain, email) {
        return Some(config);
    }

    for url in mozilla_urls(domain, email) {
        if let Some(config) = mozilla::fetch(&url, email) {
            return Some(config);
        }
    }

    autodiscover::fetch(domain, email)
}

fn mozilla_urls(domain: &str, email: &str) -> Vec<String> {
    vec![
        format!("https://autoconfig.{domain}/mail/config-v1.1.xml?emailaddress={email}"),
        format!(
            "https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={email}"
        ),
        format!("https://autoconfig.thunderbird.net/v1.1/{domain}"),
    ]
}
