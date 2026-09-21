/*! Discovery orchestration: presets, Mozilla autoconfig, Autodiscover, then SRV. */

use super::model::DiscoveredConfig;
use super::srv::{self, SrvLookup, SystemSrvLookup};
use super::{autodiscover, mozilla, presets};

pub fn discover(email: &str) -> Option<DiscoveredConfig> {
    discover_with(email, &SystemSrvLookup)
}

/// Discovery with an injectable SRV resolver, so the mapping is testable
/// without DNS.
pub(super) fn discover_with(email: &str, resolver: &dyn SrvLookup) -> Option<DiscoveredConfig> {
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

    if let Some(config) = autodiscover::fetch(domain, email) {
        return Some(config);
    }

    srv::discover_via_srv(email, domain, resolver)
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
