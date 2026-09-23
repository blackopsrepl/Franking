/*! Searchable local message store, keeping the older envelope cache intact. */

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE messages (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             account TEXT NOT NULL, folder TEXT NOT NULL, uid TEXT NOT NULL,
             uid_validity INTEGER, message_id TEXT, thread_root TEXT,
             in_reply_to TEXT, refs TEXT, subject TEXT NOT NULL DEFAULT '',
             from_display TEXT NOT NULL DEFAULT '', from_email TEXT,
             to_display TEXT NOT NULL DEFAULT '', date_epoch INTEGER,
             flags TEXT NOT NULL DEFAULT '', size INTEGER NOT NULL DEFAULT 0,
             has_attachments INTEGER NOT NULL DEFAULT 0,
             snippet TEXT NOT NULL DEFAULT '', body_text TEXT NOT NULL DEFAULT '',
             raw BLOB, updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             UNIQUE(account, folder, uid)
         );
         CREATE INDEX idx_messages_folder ON messages(account, folder, date_epoch DESC);
         CREATE INDEX idx_messages_message_id ON messages(message_id);
         CREATE INDEX idx_messages_thread_root ON messages(account, thread_root);
         CREATE VIRTUAL TABLE messages_fts USING fts5(
             subject, from_display, to_display, body_text,
             content = 'messages', content_rowid = 'id',
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
         CREATE TABLE sync_state (
             account TEXT NOT NULL, folder TEXT NOT NULL,
             uid_validity INTEGER, uid_next INTEGER, highest_modseq INTEGER,
             last_synced_at TEXT, PRIMARY KEY(account, folder)
         );
         INSERT INTO messages (account, folder, uid, message_id, subject,
                               from_display, date_epoch, flags, updated_at)
         SELECT a.name, e.folder_remote_id, e.remote_uid, e.message_id,
                COALESCE(e.subject, ''), COALESCE(e.sender_display, ''),
                CAST(strftime('%s', e.received_at) AS INTEGER),
                COALESCE(e.flags, ''), e.updated_at
         FROM envelope_cache e JOIN accounts a ON a.id = e.account_id;",
    )?;
    Ok(())
}
