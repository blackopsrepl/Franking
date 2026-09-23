/*! Local-workflow tables, split from the core account and mail schema. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create_local_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "         CREATE TABLE sender_routes (
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
              loud         INTEGER NOT NULL DEFAULT 0 CHECK(loud IN (0, 1)),
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
         CREATE TABLE stages (
              id       INTEGER PRIMARY KEY AUTOINCREMENT,
              account  TEXT    NOT NULL,
              name     TEXT    NOT NULL,
              position INTEGER NOT NULL DEFAULT 0,
              UNIQUE(account, name)
         );
         CREATE TABLE conversation_stages (
              account    TEXT    NOT NULL,
              anchor     TEXT    NOT NULL,
              stage_id   INTEGER NOT NULL REFERENCES stages(id) ON DELETE CASCADE,
              updated_at TEXT    NOT NULL DEFAULT (datetime('now')),
              PRIMARY KEY (account, anchor)
         );
         CREATE TABLE bundles (
              account TEXT NOT NULL,
              sender  TEXT NOT NULL,
              PRIMARY KEY (account, sender)
         );
         CREATE TABLE clips (
              id         INTEGER PRIMARY KEY AUTOINCREMENT,
              account    TEXT,
              folder     TEXT,
              uid        TEXT NOT NULL,
              message_id TEXT,
              body       TEXT NOT NULL,
              source     TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE snippets (
              name       TEXT PRIMARY KEY,
              body       TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT (datetime('now'))
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
         );",
    )?;
    Ok(())
}
