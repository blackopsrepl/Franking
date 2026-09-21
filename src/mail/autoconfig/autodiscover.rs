/*! Microsoft Autodiscover (POX) discovery. */

use std::time::Duration;

use roxmltree::Document;

use super::model::{DiscoveredConfig, DiscoverySource};

struct Protocol {
    server: String,
    port: u16,
    ssl: bool,
}

pub(super) fn parse(xml: &str, email: &str) -> Option<DiscoveredConfig> {
    let document = Document::parse(xml).ok()?;
    let imap = protocol(&document, "IMAP")?;
    let smtp = protocol(&document, "SMTP")?;

    Some(DiscoveredConfig {
        provider_kind: "generic".to_string(),
        imap_host: imap.server,
        imap_port: imap.port,
        imap_security: security(imap.ssl).to_string(),
        smtp_host: smtp.server,
        smtp_port: smtp.port,
        smtp_security: security(smtp.ssl).to_string(),
        username: email.to_string(),
        auth_mode: "password".to_string(),
        source: DiscoverySource::Autodiscover,
    })
}

pub(super) fn fetch(domain: &str, email: &str) -> Option<DiscoveredConfig> {
    let url = format!("https://autodiscover.{domain}/autodiscover/autodiscover.xml");
    let request = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
         <Autodiscover xmlns=\"http://schemas.microsoft.com/exchange/autodiscover/outlook/requestschema/2006\">\
         <Request><EMailAddress>{email}</EMailAddress>\
         <AcceptableResponseSchema>http://schemas.microsoft.com/exchange/autodiscover/outlook/responseschema/2006a</AcceptableResponseSchema>\
         </Request></Autodiscover>"
    );

    let body = ureq::post(&url)
        .set("Content-Type", "text/xml")
        .timeout(Duration::from_secs(10))
        .send_string(&request)
        .ok()?
        .into_string()
        .ok()?;
    parse(&body, email)
}

fn protocol(document: &Document<'_>, kind: &str) -> Option<Protocol> {
    let node = document.descendants().find(|node| {
        node.is_element()
            && node.tag_name().name() == "Protocol"
            && child_text(*node, "Type").eq_ignore_ascii_case(kind)
    })?;

    let server = child_text(node, "Server");
    if server.is_empty() {
        return None;
    }
    let default_port = if kind.eq_ignore_ascii_case("IMAP") {
        993
    } else {
        587
    };

    Some(Protocol {
        server,
        port: child_text(node, "Port").parse().unwrap_or(default_port),
        ssl: child_text(node, "SSL").eq_ignore_ascii_case("on"),
    })
}

fn child_text(node: roxmltree::Node<'_, '_>, name: &str) -> String {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn security(ssl: bool) -> &'static str {
    if ssl {
        "tls"
    } else {
        "starttls"
    }
}
