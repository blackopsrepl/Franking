/* Sender identity management.
An identity is a From address (display name + email) associated with an
account.  Each account can have multiple identities; one may be marked as
the default.  When composing, the user picks which identity to send from;
the selection is written as a `From:` header in the outgoing template. */

use anyhow::{Context, Result};
use rusqlite::Connection;

/// A sender identity.
#[derive(Debug, Clone)]
pub struct Identity {
    pub id: i64,
    /// Account name this identity belongs to.
    pub account: String,
    /// Short label to identify this identity in the UI (e.g. "Work", "Personal").
    /// Distinct from `display_name`: this is never placed in email headers.
    pub name: Option<String>,
    /// Optional sender display name shown in the From header (e.g. "Alice Example").
    pub display_name: Option<String>,
    /// Email address (e.g. "alice@example.com").
    pub email: String,
    /// Optional signature appended to new messages sent from this identity.
    pub signature: Option<String>,
    /// Mailbox the sent copy goes to, overriding the account's Sent mailbox.
    pub sent_folder: Option<String>,
    /// Whether this is the default identity for the account.
    pub is_default: bool,
}

impl Identity {
    /// Formatted `From:` header value: `"Name" <email>` or `<email>`.
    /// Uses `display_name` (the sender name), not `name` (the UI label).
    pub fn formatted(&self) -> String {
        match &self.display_name {
            Some(n) if !n.is_empty() => format!("\"{}\" <{}>", n, self.email),
            _ => self.email.clone(),
        }
    }

    /// Short label for UI display: prefers `name`, falls back to `display_name`,
    /// then just `email`.  Always appends `<email>` when a name is shown.
    pub fn label(&self) -> String {
        let shown_name = self
            .name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.display_name.as_deref().filter(|s| !s.is_empty()));
        match shown_name {
            Some(n) => format!("{} <{}>", n, self.email),
            None => self.email.clone(),
        }
    }
}

/// Add a new identity for an account.  Returns the new identity's ID.
/// The values needed to store an identity.
#[derive(Debug, Clone, Default)]
pub struct NewIdentity {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub email: String,
    pub signature: Option<String>,
    /// Mailbox for the sent copy; `None` uses the account's Sent mailbox.
    pub sent_folder: Option<String>,
    pub is_default: bool,
}

/// Store a new identity, returning its row id.
pub fn add(conn: &Connection, account: &str, identity: &NewIdentity) -> Result<i64> {
    // If this is the new default, clear any existing default first.
    if identity.is_default {
        clear_default(conn, account)?;
    }
    conn.execute(
        "INSERT INTO identities
             (account, name, display_name, email, signature, sent_folder, is_default)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            account,
            identity.name,
            identity.display_name,
            identity.email.to_lowercase(),
            identity.signature,
            identity.sent_folder,
            identity.is_default as i32
        ],
    )
    .with_context(|| {
        format!(
            "cannot add identity {} for account {account}",
            identity.email
        )
    })?;
    Ok(conn.last_insert_rowid())
}

/// List all identities for an account, default first.
pub fn list_for_account(conn: &Connection, account: &str) -> Result<Vec<Identity>> {
    let mut stmt = conn.prepare(
        "SELECT id, account, name, display_name, email, signature, sent_folder, is_default
         FROM identities
         WHERE account = ?1
         ORDER BY is_default DESC, name, display_name, email",
    )?;
    let rows = stmt.query_map([account], row_to_identity)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("cannot list identities")
}

/// Get the default identity for an account, if any.
pub fn get_default(conn: &Connection, account: &str) -> Result<Option<Identity>> {
    let result = conn.query_row(
        "SELECT id, account, name, display_name, email, signature, sent_folder, is_default
         FROM identities WHERE account = ?1 AND is_default = 1 LIMIT 1",
        [account],
        row_to_identity,
    );
    match result {
        Ok(i) => Ok(Some(i)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("cannot get default identity"),
    }
}

/// Delete an identity by ID.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM identities WHERE id = ?1", [id])
        .with_context(|| format!("cannot delete identity id={id}"))?;
    Ok(())
}

/// Mark one identity as the default for its account (clears any existing default).
pub fn set_default(conn: &Connection, account: &str, id: i64) -> Result<()> {
    clear_default(conn, account)?;
    conn.execute("UPDATE identities SET is_default = 1 WHERE id = ?1", [id])
        .with_context(|| format!("cannot set default identity id={id}"))?;
    Ok(())
}

fn clear_default(conn: &Connection, account: &str) -> Result<()> {
    conn.execute(
        "UPDATE identities SET is_default = 0 WHERE account = ?1",
        [account],
    )?;
    Ok(())
}

fn row_to_identity(row: &rusqlite::Row<'_>) -> rusqlite::Result<Identity> {
    Ok(Identity {
        id: row.get(0)?,
        account: row.get(1)?,
        name: row.get(2)?,
        display_name: row.get(3)?,
        email: row.get(4)?,
        signature: row.get(5)?,
        sent_folder: row.get(6)?,
        is_default: row.get::<_, i32>(7)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::{add, get_default, list_for_account, NewIdentity};

    fn identity(email: &str) -> NewIdentity {
        NewIdentity {
            name: Some("Work".to_string()),
            display_name: Some("Alice".to_string()),
            email: email.to_string(),
            signature: Some("-- \nAlice".to_string()),
            sent_folder: Some("Sent Items".to_string()),
            is_default: true,
        }
    }

    #[test]
    fn stores_and_returns_the_signature_and_sent_folder() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        add(&conn, "acct", &identity("alice@example.com")).unwrap();

        let listed = list_for_account(&conn, "acct").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].signature.as_deref(), Some("-- \nAlice"));
        assert_eq!(listed[0].sent_folder.as_deref(), Some("Sent Items"));

        let default = get_default(&conn, "acct").unwrap().expect("default");
        assert_eq!(default.sent_folder.as_deref(), Some("Sent Items"));
    }

    #[test]
    fn identities_without_optional_values_keep_none() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        add(
            &conn,
            "acct",
            &NewIdentity {
                email: "bob@example.com".to_string(),
                ..NewIdentity::default()
            },
        )
        .unwrap();
        let listed = list_for_account(&conn, "acct").unwrap();
        assert!(listed[0].signature.is_none());
        assert!(
            listed[0].sent_folder.is_none(),
            "no override means the account's Sent mailbox is used"
        );
    }
}
