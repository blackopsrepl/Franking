/*! Account and OAuth row persistence. */

use std::path::PathBuf;

use anyhow::Result;
use rusqlite::{params, Connection};

use super::model::{AccountConfig, AccountRecord};

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
