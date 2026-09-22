/*! Per-folder synchronization cursors. */

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::model::SyncState;

pub fn get_sync_state(conn: &Connection, account: &str, folder: &str) -> Result<Option<SyncState>> {
    conn.query_row(
        "SELECT account, folder, uid_validity, uid_next, highest_modseq, last_synced_at
         FROM sync_state WHERE account = ?1 AND folder = ?2",
        params![account, folder],
        |row| {
            Ok(SyncState {
                account: row.get(0)?,
                folder: row.get(1)?,
                uid_validity: row.get(2)?,
                uid_next: row.get(3)?,
                highest_modseq: row.get::<_, Option<i64>>(4)?.map(|value| value as u64),
                last_synced_at: row.get(5)?,
            })
        },
    )
    .optional()
    .context("failed to load sync state")
}

pub fn set_sync_state(conn: &Connection, state: &SyncState) -> Result<()> {
    conn.execute(
        "INSERT INTO sync_state (
             account, folder, uid_validity, uid_next, highest_modseq, last_synced_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(account, folder) DO UPDATE SET
             uid_validity = excluded.uid_validity,
             uid_next = excluded.uid_next,
             highest_modseq = excluded.highest_modseq,
             last_synced_at = datetime('now')",
        params![
            state.account,
            state.folder,
            state.uid_validity,
            state.uid_next,
            state.highest_modseq.map(|value| value as i64),
        ],
    )
    .context("failed to persist sync state")?;
    Ok(())
}
