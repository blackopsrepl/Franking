use std::path::PathBuf;

use crate::mail::maildir;
use crate::mail::types::Account;

pub const TEST_ACCOUNT_NAME: &str = "test";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub name: String,
    pub backend_kind: String,
    pub provider_kind: String,
    pub enabled: bool,
    pub is_default: bool,
    pub maildir_path: Option<PathBuf>,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub imap_security: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_security: Option<String>,
    pub auth_mode: Option<String>,
    pub username: Option<String>,
    pub keyring_imap_secret_id: Option<String>,
    pub keyring_smtp_secret_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountConfig {
    pub name: String,
    pub backend_kind: String,
    pub provider_kind: String,
    pub enabled: bool,
    pub is_default: bool,
    pub maildir_path: Option<PathBuf>,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub imap_security: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_security: Option<String>,
    pub auth_mode: Option<String>,
    pub username: Option<String>,
    pub keyring_imap_secret_id: Option<String>,
    pub keyring_smtp_secret_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OauthState {
    pub provider_kind: String,
    pub client_id: String,
    pub client_secret_ref: Option<String>,
    pub refresh_token_ref: String,
    pub access_token_cached: Option<String>,
    pub access_token_expires_at: Option<String>,
    pub scopes: String,
    pub token_endpoint: String,
    pub auth_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OauthStateConfig {
    pub provider_kind: String,
    pub client_id: String,
    pub client_secret_ref: Option<String>,
    pub refresh_token_ref: String,
    pub access_token_cached: Option<String>,
    pub access_token_expires_at: Option<String>,
    pub scopes: String,
    pub token_endpoint: String,
    pub auth_endpoint: String,
}

impl AccountConfig {
    pub fn test_maildir() -> Self {
        Self {
            name: TEST_ACCOUNT_NAME.to_string(),
            backend_kind: "maildir".to_string(),
            provider_kind: "custom".to_string(),
            enabled: true,
            is_default: false,
            maildir_path: Some(maildir::default_test_maildir_path()),
            imap_host: None,
            imap_port: None,
            imap_security: None,
            smtp_host: None,
            smtp_port: None,
            smtp_security: None,
            auth_mode: Some("maildir".to_string()),
            username: None,
            keyring_imap_secret_id: None,
            keyring_smtp_secret_id: None,
        }
    }
}

impl AccountRecord {
    pub fn to_account(&self) -> Account {
        Account {
            name: self.name.clone(),
            backend: self.backend_kind.clone(),
            default: self.is_default,
        }
    }

    pub fn is_maildir(&self) -> bool {
        self.backend_kind.eq_ignore_ascii_case("maildir")
    }

    pub fn is_routable(&self) -> bool {
        self.is_maildir() || self.backend_kind.eq_ignore_ascii_case("imap")
    }
}
