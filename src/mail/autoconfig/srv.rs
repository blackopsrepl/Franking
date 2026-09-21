/*! RFC 6186 SRV-based discovery.
Lookups use the system resolver; the record-to-config mapping is pure so it can
be tested without DNS.
*/

use std::cmp::Reverse;

use super::model::{DiscoveredConfig, DiscoverySource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

pub(super) trait SrvLookup {
    fn lookup(&self, name: &str) -> Vec<SrvRecord>;
}

pub(super) struct SystemSrvLookup;

impl SrvLookup for SystemSrvLookup {
    fn lookup(&self, name: &str) -> Vec<SrvRecord> {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return Vec::new();
        };

        runtime.block_on(async {
            let provider = hickory_resolver::name_server::TokioConnectionProvider::default();
            let Ok(resolver) = hickory_resolver::AsyncResolver::from_system_conf(provider) else {
                return Vec::new();
            };
            match resolver.srv_lookup(name).await {
                Ok(lookup) => lookup
                    .iter()
                    .map(|record| SrvRecord {
                        priority: record.priority(),
                        weight: record.weight(),
                        port: record.port(),
                        target: record.target().to_string(),
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        })
    }
}

/// Resolve IMAP and SMTP submission endpoints from RFC 6186 SRV records.
pub(super) fn discover_via_srv(
    email: &str,
    domain: &str,
    resolver: &dyn SrvLookup,
) -> Option<DiscoveredConfig> {
    let imap = preferred(resolver.lookup(&format!("_imaps._tcp.{domain}")))
        .map(|record| (record, "tls"))
        .or_else(|| {
            preferred(resolver.lookup(&format!("_imap._tcp.{domain}")))
                .map(|record| (record, "starttls"))
        })?;
    let smtp = preferred(resolver.lookup(&format!("_submissions._tcp.{domain}")))
        .map(|record| (record, "tls"))
        .or_else(|| {
            preferred(resolver.lookup(&format!("_submission._tcp.{domain}")))
                .map(|record| (record, "starttls"))
        })?;

    Some(DiscoveredConfig {
        provider_kind: "generic".to_string(),
        imap_host: trim_dot(&imap.0.target),
        imap_port: imap.0.port,
        imap_security: imap.1.to_string(),
        smtp_host: trim_dot(&smtp.0.target),
        smtp_port: smtp.0.port,
        smtp_security: smtp.1.to_string(),
        username: email.to_string(),
        auth_mode: "password".to_string(),
        source: DiscoverySource::Srv,
    })
}

fn preferred(records: Vec<SrvRecord>) -> Option<SrvRecord> {
    records
        .into_iter()
        .min_by_key(|record| (record.priority, Reverse(record.weight)))
}

fn trim_dot(target: &str) -> String {
    target.trim_end_matches('.').to_string()
}
