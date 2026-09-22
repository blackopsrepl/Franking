/*! Mozilla autoconfig discovery (Thunderbird ISPDB schema). */

use std::time::Duration;

use roxmltree::Document;

use super::model::{normalize_auth, normalize_security, DiscoveredConfig, DiscoverySource};

struct Server {
    host: String,
    port: u16,
    socket_type: String,
    authentication: String,
    username: String,
}

pub(super) fn parse(xml: &str, email: &str) -> Option<DiscoveredConfig> {
    let document = Document::parse(xml).ok()?;
    let incoming = server(&document, "incomingServer", "imap", 993)?;
    let outgoing = server(&document, "outgoingServer", "smtp", 587)?;

    Some(DiscoveredConfig {
        provider_kind: "generic".to_string(),
        imap_host: incoming.host,
        imap_port: incoming.port,
        imap_security: normalize_security(&incoming.socket_type),
        smtp_host: outgoing.host,
        smtp_port: outgoing.port,
        smtp_security: normalize_security(&outgoing.socket_type),
        username: substitute_username(&incoming.username, email),
        auth_mode: normalize_auth(Some(&incoming.authentication)),
        source: DiscoverySource::MozillaAutoconfig,
    })
}

pub(super) fn fetch(url: &str, email: &str) -> Option<DiscoveredConfig> {
    let body = ureq::get(url)
        .timeout(Duration::from_secs(10))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    parse(&body, email)
}

fn server(document: &Document<'_>, tag: &str, kind: &str, default_port: u16) -> Option<Server> {
    let node = document.descendants().find(|node| {
        node.is_element()
            && node.tag_name().name() == tag
            && node
                .attribute("type")
                .map(|value| value.eq_ignore_ascii_case(kind))
                .unwrap_or(false)
    })?;

    let child = |name: &str| {
        node.children()
            .find(|child| child.is_element() && child.tag_name().name() == name)
            .and_then(|child| child.text())
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    let host = child("hostname");
    if host.is_empty() {
        return None;
    }

    Some(Server {
        host,
        port: child("port").parse().unwrap_or(default_port),
        socket_type: child("socketType"),
        authentication: child("authentication"),
        username: child("username"),
    })
}

fn substitute_username(template: &str, email: &str) -> String {
    if template.is_empty() || template.contains("%EMAILADDRESS%") {
        email.to_string()
    } else {
        template.to_string()
    }
}
