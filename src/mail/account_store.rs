use std::path::PathBuf;

use anyhow::Result;
use rusqlite::{params, Connection};

use super::maildir;
use super::types::Account;

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

pub fn seed_defaults(conn: &Connection) -> Result<()> {
    upsert_account(conn, &AccountConfig::test_maildir())
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
             a.name,
             a.backend_kind,
             a.provider_kind,
             a.enabled,
             a.is_default,
             a.maildir_path,
             e.imap_host,
             e.imap_port,
             e.imap_security,
             e.smtp_host,
             e.smtp_port,
             e.smtp_security,
             b.auth_mode,
             b.username,
             b.keyring_imap_secret_id,
             b.keyring_smtp_secret_id
         FROM accounts a
         LEFT JOIN account_endpoints e ON e.account_id = a.id
         LEFT JOIN auth_bindings b ON b.account_id = a.id
         WHERE a.enabled = 1
         ORDER BY a.is_default DESC, a.name ASC",
    )?;

    let rows = stmt.query_map([], row_to_account_record)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_account(conn: &Connection, name: &str) -> Result<Option<AccountRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
             a.name,
             a.backend_kind,
             a.provider_kind,
             a.enabled,
             a.is_default,
             a.maildir_path,
             e.imap_host,
             e.imap_port,
             e.imap_security,
             e.smtp_host,
             e.smtp_port,
             e.smtp_security,
             b.auth_mode,
             b.username,
             b.keyring_imap_secret_id,
             b.keyring_smtp_secret_id
         FROM accounts a
         LEFT JOIN account_endpoints e ON e.account_id = a.id
         LEFT JOIN auth_bindings b ON b.account_id = a.id
         WHERE a.name = ?1
         LIMIT 1",
    )?;

    let mut rows = stmt.query([name])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_account_record(row)?))
    } else {
        Ok(None)
    }
}

pub fn preferred_account(records: &[AccountRecord]) -> Option<&AccountRecord> {
    records
        .iter()
        .find(|account| account.is_default)
        .or_else(|| {
            records
                .iter()
                .find(|account| !account.backend_kind.eq_ignore_ascii_case("maildir"))
        })
        .or_else(|| records.first())
}

pub fn upsert_account(conn: &Connection, config: &AccountConfig) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    if config.is_default {
        tx.execute("UPDATE accounts SET is_default = 0", [])?;
    }

    tx.execute(
        "INSERT INTO accounts (
             name, backend_kind, provider_kind, enabled, is_default, maildir_path
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(name) DO UPDATE SET
             backend_kind = excluded.backend_kind,
             provider_kind = excluded.provider_kind,
             enabled = excluded.enabled,
             is_default = excluded.is_default,
             maildir_path = excluded.maildir_path,
             updated_at = datetime('now')",
        params![
            config.name,
            config.backend_kind,
            config.provider_kind,
            if config.enabled { 1 } else { 0 },
            if config.is_default { 1 } else { 0 },
            config
                .maildir_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
        ],
    )?;

    let account_id: i64 = tx.query_row(
        "SELECT id FROM accounts WHERE name = ?1",
        [config.name.as_str()],
        |row| row.get(0),
    )?;

    tx.execute(
        "INSERT INTO account_endpoints (
             account_id, imap_host, imap_port, imap_security, smtp_host, smtp_port, smtp_security
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(account_id) DO UPDATE SET
             imap_host = excluded.imap_host,
             imap_port = excluded.imap_port,
             imap_security = excluded.imap_security,
             smtp_host = excluded.smtp_host,
             smtp_port = excluded.smtp_port,
             smtp_security = excluded.smtp_security",
        params![
            account_id,
            config.imap_host,
            config.imap_port.map(i64::from),
            config.imap_security,
            config.smtp_host,
            config.smtp_port.map(i64::from),
            config.smtp_security
        ],
    )?;

    tx.execute(
        "INSERT INTO auth_bindings (
             account_id, auth_mode, username, keyring_imap_secret_id, keyring_smtp_secret_id, oauth_state_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL)
         ON CONFLICT(account_id) DO UPDATE SET
             auth_mode = excluded.auth_mode,
             username = excluded.username,
             keyring_imap_secret_id = excluded.keyring_imap_secret_id,
             keyring_smtp_secret_id = excluded.keyring_smtp_secret_id,
             oauth_state_id = excluded.oauth_state_id",
        params![
            account_id,
            config.auth_mode.clone().unwrap_or_default(),
            config.username,
            config.keyring_imap_secret_id,
            config.keyring_smtp_secret_id
        ],
    )?;

    tx.commit()?;
    Ok(())
}

pub fn get_oauth_state(conn: &Connection, account_name: &str) -> Result<Option<OauthState>> {
    let mut stmt = conn.prepare(
        "SELECT
             o.provider_kind,
             o.client_id,
             o.client_secret_ref,
             o.refresh_token_ref,
             o.access_token_cached,
             o.access_token_expires_at,
             o.scopes,
             o.token_endpoint,
             o.auth_endpoint
         FROM oauth_states o
         JOIN accounts a ON a.id = o.account_id
         WHERE a.name = ?1
         LIMIT 1",
    )?;

    let mut rows = stmt.query([account_name])?;
    if let Some(row) = rows.next()? {
        Ok(Some(OauthState {
            provider_kind: row.get(0)?,
            client_id: row.get(1)?,
            client_secret_ref: row.get(2)?,
            refresh_token_ref: row.get(3)?,
            access_token_cached: row.get(4)?,
            access_token_expires_at: row.get(5)?,
            scopes: row.get(6)?,
            token_endpoint: row.get(7)?,
            auth_endpoint: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_oauth_state(
    conn: &Connection,
    account_name: &str,
    config: &OauthStateConfig,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let account_id: i64 = tx.query_row(
        "SELECT id FROM accounts WHERE name = ?1",
        [account_name],
        |row| row.get(0),
    )?;

    let existing_id = tx
        .query_row(
            "SELECT id FROM oauth_states WHERE account_id = ?1",
            [account_id],
            |row| row.get::<_, i64>(0),
        )
        .ok();

    match existing_id {
        Some(id) => {
            tx.execute(
                "UPDATE oauth_states
                 SET provider_kind = ?2,
                     client_id = ?3,
                     client_secret_ref = ?4,
                     refresh_token_ref = ?5,
                     access_token_cached = ?6,
                     access_token_expires_at = ?7,
                     scopes = ?8,
                     token_endpoint = ?9,
                     auth_endpoint = ?10
                 WHERE id = ?1",
                params![
                    id,
                    config.provider_kind,
                    config.client_id,
                    config.client_secret_ref,
                    config.refresh_token_ref,
                    config.access_token_cached,
                    config.access_token_expires_at,
                    config.scopes,
                    config.token_endpoint,
                    config.auth_endpoint
                ],
            )?;
        }
        None => {
            tx.execute(
                "INSERT INTO oauth_states (
                     account_id,
                     provider_kind,
                     client_id,
                     client_secret_ref,
                     refresh_token_ref,
                     access_token_cached,
                     access_token_expires_at,
                     scopes,
                     token_endpoint,
                     auth_endpoint
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    account_id,
                    config.provider_kind,
                    config.client_id,
                    config.client_secret_ref,
                    config.refresh_token_ref,
                    config.access_token_cached,
                    config.access_token_expires_at,
                    config.scopes,
                    config.token_endpoint,
                    config.auth_endpoint
                ],
            )?;
        }
    }

    let oauth_state_id: i64 = tx.query_row(
        "SELECT id FROM oauth_states WHERE account_id = ?1",
        [account_id],
        |row| row.get(0),
    )?;

    tx.execute(
        "UPDATE auth_bindings
         SET oauth_state_id = ?2
         WHERE account_id = ?1",
        params![account_id, oauth_state_id],
    )?;

    tx.commit()?;
    Ok(())
}

fn row_to_account_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    let path = row.get::<_, Option<String>>(5)?;
    Ok(AccountRecord {
        name: row.get(0)?,
        backend_kind: row.get(1)?,
        provider_kind: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        is_default: row.get::<_, i64>(4)? != 0,
        maildir_path: path.map(PathBuf::from),
        imap_host: row.get(6)?,
        imap_port: row.get::<_, Option<i64>>(7)?.map(|value| value as u16),
        imap_security: row.get(8)?,
        smtp_host: row.get(9)?,
        smtp_port: row.get::<_, Option<i64>>(10)?.map(|value| value as u16),
        smtp_security: row.get(11)?,
        auth_mode: row.get(12)?,
        username: row.get(13)?,
        keyring_imap_secret_id: row.get(14)?,
        keyring_smtp_secret_id: row.get(15)?,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        get_account, get_oauth_state, upsert_account, upsert_oauth_state, AccountConfig,
        OauthStateConfig,
    };

    #[test]
    fn upsert_account_persists_endpoint_and_auth_details() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        let config = AccountConfig {
            name: "work".to_string(),
            backend_kind: "imap".to_string(),
            provider_kind: "generic".to_string(),
            enabled: true,
            is_default: true,
            maildir_path: None,
            imap_host: Some("imap.example.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: Some(465),
            smtp_security: Some("tls".to_string()),
            auth_mode: Some("password".to_string()),
            username: Some("alice@example.com".to_string()),
            keyring_imap_secret_id: Some("solverforge-mail/work/imap".to_string()),
            keyring_smtp_secret_id: Some("solverforge-mail/work/smtp".to_string()),
        };

        upsert_account(&conn, &config).unwrap();

        let stored = get_account(&conn, "work").unwrap().unwrap();
        assert_eq!(stored.imap_host.as_deref(), Some("imap.example.com"));
        assert_eq!(stored.smtp_port, Some(465));
        assert_eq!(stored.auth_mode.as_deref(), Some("password"));
        assert_eq!(
            stored.keyring_imap_secret_id.as_deref(),
            Some("solverforge-mail/work/imap")
        );
        assert_eq!(stored.provider_kind, "generic");
        assert!(stored.is_default);
    }

    #[test]
    fn upsert_oauth_state_persists_refresh_metadata() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        let config = AccountConfig {
            name: "gmail".to_string(),
            backend_kind: "imap".to_string(),
            provider_kind: "gmail".to_string(),
            enabled: true,
            is_default: false,
            maildir_path: None,
            imap_host: Some("imap.gmail.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.gmail.com".to_string()),
            smtp_port: Some(587),
            smtp_security: Some("starttls".to_string()),
            auth_mode: Some("oauth2".to_string()),
            username: Some("alice@gmail.com".to_string()),
            keyring_imap_secret_id: None,
            keyring_smtp_secret_id: None,
        };
        upsert_account(&conn, &config).unwrap();

        let oauth = OauthStateConfig {
            provider_kind: "gmail".to_string(),
            client_id: "client".to_string(),
            client_secret_ref: Some("solverforge-mail/gmail/client-secret".to_string()),
            refresh_token_ref: "solverforge-mail/gmail/refresh-token".to_string(),
            access_token_cached: Some("access".to_string()),
            access_token_expires_at: Some("2026-04-13T10:00:00+00:00".to_string()),
            scopes: "scope1 scope2".to_string(),
            token_endpoint: "https://token.example.com".to_string(),
            auth_endpoint: "https://auth.example.com".to_string(),
        };
        upsert_oauth_state(&conn, "gmail", &oauth).unwrap();

        let stored = get_oauth_state(&conn, "gmail").unwrap().unwrap();
        assert_eq!(stored.client_id, "client");
        assert_eq!(
            stored.client_secret_ref.as_deref(),
            Some("solverforge-mail/gmail/client-secret")
        );
        assert_eq!(
            stored.refresh_token_ref,
            "solverforge-mail/gmail/refresh-token"
        );
        assert_eq!(stored.access_token_cached.as_deref(), Some("access"));
    }
}
