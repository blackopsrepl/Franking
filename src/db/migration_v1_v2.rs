/*! App-owned account metadata introduced after the initial local schema. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE account_endpoints (
             account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
             imap_host TEXT, imap_port INTEGER, imap_security TEXT,
             smtp_host TEXT, smtp_port INTEGER, smtp_security TEXT
         );
         CREATE TABLE oauth_states (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
             provider_kind TEXT NOT NULL, client_id TEXT, client_secret_ref TEXT,
             refresh_token_ref TEXT, access_token_cached TEXT,
             access_token_expires_at TEXT, scopes TEXT,
             token_endpoint TEXT, auth_endpoint TEXT
         );
         CREATE INDEX idx_oauth_states_account ON oauth_states(account_id);
         CREATE TABLE auth_bindings (
             account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
             auth_mode TEXT NOT NULL, username TEXT,
             keyring_imap_secret_id TEXT, keyring_smtp_secret_id TEXT,
             oauth_state_id INTEGER REFERENCES oauth_states(id) ON DELETE SET NULL
         );
         CREATE TABLE folder_cache (
             account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
             remote_id TEXT NOT NULL, name TEXT NOT NULL,
             attributes TEXT, unread_count INTEGER, sync_token TEXT,
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY(account_id, remote_id)
         );
         CREATE TABLE envelope_cache (
             account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
             folder_remote_id TEXT NOT NULL, remote_uid TEXT NOT NULL,
             message_id TEXT, subject TEXT, sender_display TEXT,
             received_at TEXT, flags TEXT, thread_hint TEXT,
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY(account_id, folder_remote_id, remote_uid)
         );",
    )?;
    Ok(())
}
