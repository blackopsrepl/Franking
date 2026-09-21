/*! OAuth state row persistence. */

use anyhow::Result;
use rusqlite::{params, Connection};

use super::model::{OauthState, OauthStateConfig};

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
