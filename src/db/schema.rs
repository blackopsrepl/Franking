/*! Current-state schema DDL for new databases only. */

use anyhow::Result;
use rusqlite::Connection;

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
             sent_folder  TEXT,
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
             sent_folder    TEXT,
             send_after     TEXT,
             created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE saved_searches (
             name       TEXT PRIMARY KEY,
             query      TEXT NOT NULL,
             scope      TEXT NOT NULL DEFAULT 'folder',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE sender_routes (
              account TEXT NOT NULL,
              sender  TEXT NOT NULL,
              route   TEXT NOT NULL CHECK(route IN ('inbox', 'reading', 'receipts', 'blocked')),
              PRIMARY KEY (account, sender)
          );

         CREATE TABLE message_markers (
              account TEXT NOT NULL,
              folder TEXT NOT NULL,
              uid TEXT NOT NULL,
              message_id TEXT,
              reply_later INTEGER NOT NULL DEFAULT 0 CHECK(reply_later IN (0, 1)),
              saved INTEGER NOT NULL DEFAULT 0 CHECK(saved IN (0, 1)),
              PRIMARY KEY (account, folder, uid)
         );
         CREATE INDEX idx_message_markers_account
              ON message_markers(account, reply_later, saved);

         CREATE TABLE conversation_rules (
              id           INTEGER PRIMARY KEY AUTOINCREMENT,
              account      TEXT    NOT NULL,
              anchor       TEXT    NOT NULL,
              muted        INTEGER NOT NULL DEFAULT 0 CHECK(muted IN (0, 1)),
              resurface_at TEXT,
              created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
              UNIQUE(account, anchor)
         );
         CREATE INDEX idx_conversation_rules_muted
              ON conversation_rules(account, muted);
         CREATE INDEX idx_conversation_rules_resurface
              ON conversation_rules(account, resurface_at);

         CREATE TABLE message_routes (
              account    TEXT NOT NULL,
              folder     TEXT NOT NULL,
              uid        TEXT NOT NULL,
              message_id TEXT,
              route      TEXT NOT NULL,
              PRIMARY KEY (account, folder, uid)
         );
         CREATE INDEX idx_message_routes_account
              ON message_routes(account);

         CREATE TABLE message_notes (
              account    TEXT NOT NULL,
              folder     TEXT NOT NULL,
              uid        TEXT NOT NULL,
              message_id TEXT,
              body       TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              PRIMARY KEY (account, folder, uid)
         );
         CREATE TABLE account_policy (
              account      TEXT PRIMARY KEY,
              bypass_token TEXT
         );
         CREATE TABLE subject_aliases (
              account    TEXT NOT NULL,
              anchor     TEXT NOT NULL,
              alias      TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              PRIMARY KEY (account, anchor)
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
