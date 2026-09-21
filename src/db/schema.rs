/*! Current-state schema DDL.
Older databases are dropped and rebuilt rather than migrated, so this module
owns both the destructive reset and the create statements. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn reset_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS contact_tags;
         DROP TABLE IF EXISTS contacts;
         DROP TABLE IF EXISTS identities;
         DROP TABLE IF EXISTS messages_fts;
         DROP TABLE IF EXISTS messages;
         DROP TABLE IF EXISTS sync_state;
         DROP TABLE IF EXISTS envelope_cache;
         DROP TABLE IF EXISTS folder_cache;
         DROP TABLE IF EXISTS auth_bindings;
         DROP TABLE IF EXISTS oauth_states;
         DROP TABLE IF EXISTS account_endpoints;
         DROP TABLE IF EXISTS accounts;
         DROP TABLE IF EXISTS credentials;
         DROP TABLE IF EXISTS outbox;
         DROP TABLE IF EXISTS saved_searches;
         DROP TABLE IF EXISTS legacy_credentials_backup;
         DROP TABLE IF EXISTS meta;",
    )?;
    Ok(())
}

pub(super) fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE meta (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );

         CREATE TABLE contacts (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             name       TEXT,
             email      TEXT    NOT NULL,
             phone      TEXT,
             org        TEXT,
             notes      TEXT,
             harvested  INTEGER NOT NULL DEFAULT 0,
             created_at TEXT    NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
         );
         CREATE UNIQUE INDEX idx_contacts_email
             ON contacts(email);
         CREATE INDEX idx_contacts_name
             ON contacts(name);

         CREATE TABLE contact_tags (
             contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
             tag        TEXT    NOT NULL,
             PRIMARY KEY (contact_id, tag)
         );

         CREATE TABLE identities (
             id           INTEGER PRIMARY KEY AUTOINCREMENT,
             account      TEXT    NOT NULL,
             name         TEXT,
             display_name TEXT,
             email        TEXT    NOT NULL,
             signature    TEXT,
             is_default   INTEGER NOT NULL DEFAULT 0,
             created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
             UNIQUE(account, email)
         );
         CREATE INDEX idx_identities_account
             ON identities(account);

         CREATE TABLE accounts (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             name          TEXT    NOT NULL UNIQUE,
             backend_kind  TEXT    NOT NULL,
             provider_kind TEXT    NOT NULL DEFAULT 'custom',
             enabled       INTEGER NOT NULL DEFAULT 1,
             is_default    INTEGER NOT NULL DEFAULT 0,
             maildir_path  TEXT,
             created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
             updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX idx_accounts_enabled
             ON accounts(enabled);
         CREATE INDEX idx_accounts_default
             ON accounts(is_default);

         CREATE TABLE account_endpoints (
             account_id     INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
             sieve_host     TEXT,
             sieve_port     INTEGER,
             sieve_security TEXT,
             imap_host      TEXT,
             imap_port      INTEGER,
             imap_security  TEXT,
             smtp_host      TEXT,
             smtp_port      INTEGER,
             smtp_security  TEXT
         );

         CREATE TABLE oauth_states (
             id                      INTEGER PRIMARY KEY AUTOINCREMENT,
             account_id              INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
             provider_kind           TEXT    NOT NULL,
             client_id               TEXT,
             client_secret_ref       TEXT,
             refresh_token_ref       TEXT,
             access_token_cached     TEXT,
             access_token_expires_at TEXT,
             scopes                  TEXT,
             token_endpoint          TEXT,
             auth_endpoint           TEXT
         );
         CREATE INDEX idx_oauth_states_account
             ON oauth_states(account_id);

         CREATE TABLE auth_bindings (
             account_id              INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
             auth_mode               TEXT    NOT NULL,
             username                TEXT,
             keyring_imap_secret_id  TEXT,
             keyring_smtp_secret_id  TEXT,
             oauth_state_id          INTEGER REFERENCES oauth_states(id) ON DELETE SET NULL
         );

         CREATE TABLE folder_cache (
             account_id     INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
             remote_id      TEXT    NOT NULL,
             name           TEXT    NOT NULL,
             attributes     TEXT,
             unread_count   INTEGER,
             sync_token     TEXT,
             updated_at     TEXT    NOT NULL DEFAULT (datetime('now')),
             PRIMARY KEY (account_id, remote_id)
         );

         CREATE TABLE messages (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             account         TEXT    NOT NULL,
             folder          TEXT    NOT NULL,
             uid             TEXT    NOT NULL,
             uid_validity    INTEGER,
             message_id      TEXT,
             thread_root     TEXT,
             in_reply_to     TEXT,
             refs            TEXT,
             subject         TEXT    NOT NULL DEFAULT '',
             from_display    TEXT    NOT NULL DEFAULT '',
             from_email      TEXT,
             to_display      TEXT    NOT NULL DEFAULT '',
             date_epoch      INTEGER,
             flags           TEXT    NOT NULL DEFAULT '',
             size            INTEGER NOT NULL DEFAULT 0,
             has_attachments INTEGER NOT NULL DEFAULT 0,
             snippet         TEXT    NOT NULL DEFAULT '',
             body_text       TEXT    NOT NULL DEFAULT '',
             raw             BLOB,
             updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),
             UNIQUE(account, folder, uid)
         );
         CREATE INDEX idx_messages_folder
             ON messages(account, folder, date_epoch DESC);
         CREATE INDEX idx_messages_message_id
             ON messages(message_id);
         CREATE INDEX idx_messages_thread_root
             ON messages(account, thread_root);

         CREATE VIRTUAL TABLE messages_fts USING fts5(
             subject,
             from_display,
             to_display,
             body_text,
             content = 'messages',
             content_rowid = 'id',
             tokenize = 'unicode61 remove_diacritics 2'
         );
         CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
             INSERT INTO messages_fts(rowid, subject, from_display, to_display, body_text)
             VALUES (new.id, new.subject, new.from_display, new.to_display, new.body_text);
         END;
         CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
             INSERT INTO messages_fts(messages_fts, rowid, subject, from_display, to_display, body_text)
             VALUES ('delete', old.id, old.subject, old.from_display, old.to_display, old.body_text);
         END;
         CREATE TRIGGER messages_au AFTER UPDATE ON messages BEGIN
             INSERT INTO messages_fts(messages_fts, rowid, subject, from_display, to_display, body_text)
             VALUES ('delete', old.id, old.subject, old.from_display, old.to_display, old.body_text);
             INSERT INTO messages_fts(rowid, subject, from_display, to_display, body_text)
             VALUES (new.id, new.subject, new.from_display, new.to_display, new.body_text);
         END;

         CREATE TABLE outbox (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             account    TEXT,
             template   TEXT    NOT NULL,
             sign           INTEGER NOT NULL DEFAULT 0,
             encrypt        INTEGER NOT NULL DEFAULT 0,
             smime_sign     INTEGER NOT NULL DEFAULT 0,
             smime_encrypt  INTEGER NOT NULL DEFAULT 0,
             send_after     TEXT,
             created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE saved_searches (
             name        TEXT PRIMARY KEY,
             query       TEXT    NOT NULL,
             all_folders INTEGER NOT NULL DEFAULT 0,
             created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE sync_state (
             account        TEXT    NOT NULL,
             folder         TEXT    NOT NULL,
             uid_validity   INTEGER,
             uid_next       INTEGER,
             highest_modseq INTEGER,
             last_synced_at TEXT,
             PRIMARY KEY (account, folder)
         );",
    )?;
    Ok(())
}

/// The table and column named by an `ALTER TABLE … ADD COLUMN …` statement.
fn added_column(statement: &str) -> Option<(&str, &str)> {
    let table = statement
        .strip_prefix("ALTER TABLE ")?
        .split_whitespace()
        .next()?;
    let column = statement
        .split("ADD COLUMN ")
        .nth(1)?
        .split_whitespace()
        .next()?;
    Some((table, column))
}

/// Add columns introduced after the first schema without resetting local data.
pub(super) fn migrate_schema(conn: &Connection) -> Result<()> {
    let has_signature: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('identities') WHERE name = 'signature'")?
        .exists([])?;
    if !has_signature {
        conn.execute_batch("ALTER TABLE identities ADD COLUMN signature TEXT;")?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS outbox (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             account    TEXT,
             template   TEXT    NOT NULL,
             sign           INTEGER NOT NULL DEFAULT 0,
             encrypt        INTEGER NOT NULL DEFAULT 0,
             smime_sign     INTEGER NOT NULL DEFAULT 0,
             smime_encrypt  INTEGER NOT NULL DEFAULT 0,
             send_after     TEXT,
             created_at TEXT    NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    let has_send_after: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('outbox') WHERE name = 'send_after'")?
        .exists([])?;
    if !has_send_after {
        conn.execute_batch("ALTER TABLE outbox ADD COLUMN send_after TEXT;")?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_searches (
             name        TEXT PRIMARY KEY,
             query       TEXT    NOT NULL,
             all_folders INTEGER NOT NULL DEFAULT 0,
             created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
         );",
    )?;
    for column in [
        "ALTER TABLE outbox ADD COLUMN smime_sign INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE outbox ADD COLUMN smime_encrypt INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_host TEXT;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_port INTEGER;",
        "ALTER TABLE account_endpoints ADD COLUMN sieve_security TEXT;",
    ] {
        let Some((table, name)) = added_column(column) else {
            continue;
        };
        // Guard against the column's own table: an earlier version checked
        // every statement against one table, so adding a column to any other
        // table ran an ALTER that the CREATE had already covered.
        let exists: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
            .exists([table, name])?;
        if !exists {
            conn.execute_batch(column)?;
        }
    }
    Ok(())
}
