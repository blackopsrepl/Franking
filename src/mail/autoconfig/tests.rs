/*! Auto-discovery unit tests. */

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::model::DiscoverySource;
use super::{autodiscover, discover, mozilla, presets};

const MOZILLA_XML: &str = r#"<?xml version="1.0"?>
<clientConfig version="1.1">
  <emailProvider id="example.com">
    <domain>example.com</domain>
    <incomingServer type="imap">
      <hostname>imap.example.com</hostname>
      <port>993</port>
      <socketType>SSL</socketType>
      <authentication>password-cleartext</authentication>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
    <outgoingServer type="smtp">
      <hostname>smtp.example.com</hostname>
      <port>587</port>
      <socketType>STARTTLS</socketType>
      <authentication>password-cleartext</authentication>
      <username>%EMAILADDRESS%</username>
    </outgoingServer>
  </emailProvider>
</clientConfig>"#;

const AUTODISCOVER_XML: &str = r#"<?xml version="1.0"?>
<Autodiscover xmlns="http://schemas.microsoft.com/exchange/autodiscover/responseschema/2006">
  <Response><Account>
    <Protocol><Type>IMAP</Type><Server>imap.corp.example</Server><Port>993</Port><SSL>on</SSL></Protocol>
    <Protocol><Type>SMTP</Type><Server>smtp.corp.example</Server><Port>587</Port><SSL>off</SSL></Protocol>
  </Account></Response>
</Autodiscover>"#;

fn serve_once(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0u8; 2048];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}/config-v1.1.xml")
}

#[test]
fn presets_cover_google_icloud_and_outlook() {
    let google = presets::preset_for_domain("gmail.com", "alice@gmail.com").unwrap();
    assert_eq!(google.imap_host, "imap.gmail.com");
    assert_eq!(google.auth_mode, "oauth2");
    assert_eq!(google.source, DiscoverySource::Preset);

    let icloud = presets::preset_for_domain("icloud.com", "alice@icloud.com").unwrap();
    assert_eq!(icloud.smtp_host, "smtp.mail.me.com");
    assert_eq!(icloud.auth_mode, "app_password");

    let outlook = presets::preset_for_domain("outlook.com", "alice@outlook.com").unwrap();
    assert_eq!(outlook.imap_host, "outlook.office365.com");
    assert_eq!(outlook.auth_mode, "oauth2");
}

#[test]
fn mozilla_autoconfig_maps_servers_and_security() {
    let config = mozilla::parse(MOZILLA_XML, "alice@example.com").unwrap();
    assert_eq!(config.imap_host, "imap.example.com");
    assert_eq!(config.imap_port, 993);
    assert_eq!(config.imap_security, "tls");
    assert_eq!(config.smtp_host, "smtp.example.com");
    assert_eq!(config.smtp_port, 587);
    assert_eq!(config.smtp_security, "starttls");
    assert_eq!(config.username, "alice@example.com");
    assert_eq!(config.source, DiscoverySource::MozillaAutoconfig);
}

#[test]
fn autodiscover_maps_ssl_flags() {
    let config = autodiscover::parse(AUTODISCOVER_XML, "alice@corp.example").unwrap();
    assert_eq!(config.imap_host, "imap.corp.example");
    assert_eq!(config.imap_security, "tls");
    assert_eq!(config.smtp_host, "smtp.corp.example");
    assert_eq!(config.smtp_security, "starttls");
    assert_eq!(config.source, DiscoverySource::Autodiscover);
}

#[test]
fn malformed_documents_are_rejected() {
    assert!(mozilla::parse("<not-xml", "a@b.com").is_none());
    assert!(mozilla::parse("<clientConfig/>", "a@b.com").is_none());
    assert!(autodiscover::parse("<Autodiscover/>", "a@b.com").is_none());
}

#[test]
fn fetch_parses_a_served_autoconfig_document() {
    let url = serve_once(MOZILLA_XML);
    let config = mozilla::fetch(&url, "alice@example.com").unwrap();
    assert_eq!(config.imap_host, "imap.example.com");
}

#[test]
fn discover_prefers_presets_without_network() {
    let config = discover::discover("alice@gmail.com").unwrap();
    assert_eq!(config.imap_host, "imap.gmail.com");
    assert_eq!(config.source, DiscoverySource::Preset);
    assert!(discover::discover("not-an-email").is_none());
}

mod srv_tests {
    use super::super::discover;
    use super::super::model::DiscoverySource;
    use super::super::srv::{SrvLookup, SrvRecord};

    struct MockSrv(Vec<(&'static str, Vec<SrvRecord>)>);

    impl SrvLookup for MockSrv {
        fn lookup(&self, name: &str) -> Vec<SrvRecord> {
            self.0
                .iter()
                .find(|(candidate, _)| *candidate == name)
                .map(|(_, records)| records.clone())
                .unwrap_or_default()
        }
    }

    fn record(priority: u16, weight: u16, port: u16, target: &str) -> SrvRecord {
        SrvRecord {
            priority,
            weight,
            port,
            target: target.to_string(),
        }
    }

    #[test]
    fn srv_records_map_to_imap_and_submission_endpoints() {
        let resolver = MockSrv(vec![
            (
                "_imaps._tcp.example.org",
                vec![record(10, 5, 993, "imap1.example.org.")],
            ),
            (
                "_submissions._tcp.example.org",
                vec![record(10, 5, 465, "smtp1.example.org.")],
            ),
        ]);

        let config = discover::discover_with("alice@example.org", &resolver).unwrap();
        assert_eq!(config.imap_host, "imap1.example.org");
        assert_eq!(config.imap_port, 993);
        assert_eq!(config.imap_security, "tls");
        assert_eq!(config.smtp_host, "smtp1.example.org");
        assert_eq!(config.smtp_port, 465);
        assert_eq!(config.source, DiscoverySource::Srv);
    }

    #[test]
    fn srv_prefers_lower_priority_then_higher_weight() {
        let resolver = MockSrv(vec![
            (
                "_imaps._tcp.example.org",
                vec![
                    record(20, 0, 993, "backup.example.org."),
                    record(10, 1, 993, "low.example.org."),
                    record(10, 9, 993, "high.example.org."),
                ],
            ),
            (
                "_submissions._tcp.example.org",
                vec![record(10, 0, 587, "smtp.example.org.")],
            ),
        ]);

        let config = discover::discover_with("alice@example.org", &resolver).unwrap();
        assert_eq!(config.imap_host, "high.example.org");
    }

    #[test]
    fn srv_falls_back_to_starttls_service_names() {
        let resolver = MockSrv(vec![
            (
                "_imap._tcp.example.org",
                vec![record(10, 0, 143, "imap.example.org.")],
            ),
            (
                "_submission._tcp.example.org",
                vec![record(10, 0, 587, "smtp.example.org.")],
            ),
        ]);

        let config = discover::discover_with("alice@example.org", &resolver).unwrap();
        assert_eq!(config.imap_port, 143);
        assert_eq!(config.imap_security, "starttls");
        assert_eq!(config.smtp_security, "starttls");
    }

    #[test]
    fn srv_requires_both_endpoints() {
        let resolver = MockSrv(vec![(
            "_imaps._tcp.example.org",
            vec![record(10, 0, 993, "imap.example.org.")],
        )]);

        assert!(discover::discover_with("alice@example.org", &resolver).is_none());
    }
}
