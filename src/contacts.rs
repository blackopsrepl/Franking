/* Contact management in the encrypted SQLite database.
Contacts store rich address-book data. Addresses can be added manually,
imported from vCard / CSV, or auto-harvested from sent/received mail. */

use anyhow::{Context, Result};
use rusqlite::Connection;

/// A contact record.
#[derive(Debug, Clone)]
pub struct Contact {
    pub id: i64,
    pub name: Option<String>,
    pub email: String,
    pub phone: Option<String>,
    pub org: Option<String>,
    pub notes: Option<String>,
    pub harvested: bool,
    pub tags: Vec<String>,
}

impl Contact {
    /// Display name: use `name` if available, else `email`.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.email)
    }

    /// Formatted address for use in email headers: `"Name" <email>` or `<email>`.
    pub fn formatted_address(&self) -> String {
        match &self.name {
            Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, self.email),
            _ => self.email.clone(),
        }
    }
}

/// Add a new contact. Returns the new contact's ID.
/// Fails if the email address already exists.
pub fn add(conn: &Connection, contact: &Contact) -> Result<i64> {
    conn.execute(
        "INSERT INTO contacts (name, email, phone, org, notes, harvested)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            contact.name,
            contact.email,
            contact.phone,
            contact.org,
            contact.notes,
            contact.harvested as i32,
        ],
    )
    .with_context(|| format!("cannot add contact: {}", contact.email))?;
    Ok(conn.last_insert_rowid())
}

/// Upsert a contact by email address. If the email already exists and the
/// existing record was auto-harvested, it is updated. If it was manually
/// added, only the `harvested` field and timestamp are updated (we don't
/// overwrite user data). Returns the contact ID.
pub fn upsert_harvested(conn: &Connection, name: Option<&str>, email: &str) -> Result<i64> {
    // Check for existing entry.
    let existing: Option<(i64, bool)> = conn
        .query_row(
            "SELECT id, harvested FROM contacts WHERE email = ?1",
            [email],
            |row| Ok((row.get(0)?, row.get::<_, i32>(1)? != 0)),
        )
        .ok();

    if let Some((id, harvested)) = existing {
        if harvested {
            // Update name if we now have one and didn't before.
            conn.execute(
                "UPDATE contacts SET name = COALESCE(?1, name), updated_at = datetime('now')
                 WHERE id = ?2",
                rusqlite::params![name, id],
            )?;
        }
        // If manually added, don't overwrite anything.
        Ok(id)
    } else {
        conn.execute(
            "INSERT INTO contacts (name, email, harvested) VALUES (?1, ?2, 1)",
            rusqlite::params![name, email],
        )
        .with_context(|| format!("cannot harvest contact: {email}"))?;
        Ok(conn.last_insert_rowid())
    }
}

/// Update an existing contact.
pub fn update(conn: &Connection, contact: &Contact) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET name = ?1, email = ?2, phone = ?3, org = ?4,
             notes = ?5, updated_at = datetime('now')
         WHERE id = ?6",
        rusqlite::params![
            contact.name,
            contact.email,
            contact.phone,
            contact.org,
            contact.notes,
            contact.id,
        ],
    )
    .with_context(|| format!("cannot update contact id={}", contact.id))?;
    Ok(())
}

/// Delete a contact and all its tags.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM contacts WHERE id = ?1", [id])
        .with_context(|| format!("cannot delete contact id={id}"))?;
    Ok(())
}

/// Get a single contact by ID, including its tags.
pub fn get(conn: &Connection, id: i64) -> Result<Option<Contact>> {
    let result = conn.query_row(
        "SELECT id, name, email, phone, org, notes, harvested FROM contacts WHERE id = ?1",
        [id],
        row_to_contact,
    );
    match result {
        Ok(mut c) => {
            c.tags = get_tags(conn, c.id)?;
            Ok(Some(c))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("cannot get contact"),
    }
}

/// Search contacts by query string (fuzzy match on name + email).
/// Returns up to `limit` results ordered by relevance (exact match first,
/// then prefix match, then substring match).
pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Contact>> {
    let pattern = format!("%{}%", query.to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT id, name, email, phone, org, notes, harvested
         FROM contacts
         WHERE lower(email) LIKE ?1 OR lower(name) LIKE ?1
         ORDER BY
             CASE
                 WHEN lower(email) = lower(?2) THEN 0
                 WHEN lower(email) LIKE lower(?2) || '%' THEN 1
                 WHEN lower(name)  LIKE lower(?2) || '%' THEN 2
                 ELSE 3
             END,
             name, email
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![pattern, query, limit as i64],
        row_to_contact,
    )?;
    let mut contacts: Vec<Contact> = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("contact search failed")?;
    for c in &mut contacts {
        c.tags = get_tags(conn, c.id)?;
    }
    Ok(contacts)
}

/// List all contacts ordered by name, with optional tag filter.
pub fn list(conn: &Connection, tag_filter: Option<&str>) -> Result<Vec<Contact>> {
    let mut contacts: Vec<Contact> = if let Some(tag) = tag_filter {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name, c.email, c.phone, c.org, c.notes, c.harvested
             FROM contacts c
             JOIN contact_tags t ON t.contact_id = c.id
             WHERE t.tag = ?1
             ORDER BY c.name, c.email",
        )?;
        let rows: Vec<Contact> = stmt
            .query_map([tag], row_to_contact)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("cannot list contacts by tag")?;
        rows
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, org, notes, harvested
             FROM contacts ORDER BY name, email",
        )?;
        let rows: Vec<Contact> = stmt
            .query_map([], row_to_contact)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("cannot list contacts")?;
        rows
    };
    for c in &mut contacts {
        c.tags = get_tags(conn, c.id)?;
    }
    Ok(contacts)
}

/// Every distinct tag in use, sorted.
pub fn all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut statement = conn.prepare("SELECT DISTINCT tag FROM contact_tags ORDER BY tag")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("cannot list tags")
}

/// Tag a contact.
pub fn add_tag(conn: &Connection, contact_id: i64, tag: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO contact_tags (contact_id, tag) VALUES (?1, ?2)",
        rusqlite::params![contact_id, tag],
    )?;
    Ok(())
}

/// Remove a tag from a contact.
pub fn remove_tag(conn: &Connection, contact_id: i64, tag: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM contact_tags WHERE contact_id = ?1 AND tag = ?2",
        rusqlite::params![contact_id, tag],
    )?;
    Ok(())
}

/// Get all tags for a contact.
fn get_tags(conn: &Connection, contact_id: i64) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT tag FROM contact_tags WHERE contact_id = ?1 ORDER BY tag")?;
    let tags: Vec<String> = stmt
        .query_map([contact_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("cannot get tags")?;
    Ok(tags)
}

fn row_to_contact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        phone: row.get(3)?,
        org: row.get(4)?,
        notes: row.get(5)?,
        harvested: row.get::<_, i32>(6)? != 0,
        tags: Vec::new(), // populated separately
    })
}

/// Extract (name, email) pairs from a header value like:
/// `"Alice" <alice@example.com>, bob@example.com`
pub fn parse_address_list(header: &str) -> Vec<(Option<String>, String)> {
    let mut results = Vec::new();
    for part in header.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(addr) = parse_single_address(part) {
            results.push(addr);
        }
    }
    results
}

fn parse_single_address(s: &str) -> Option<(Option<String>, String)> {
    // Format: "Name" <email>  or  Name <email>  or  <email>  or  email
    if let Some(angle_start) = s.find('<') {
        let angle_end = s.find('>')?;
        let email = s[angle_start + 1..angle_end].trim().to_lowercase();
        if email.contains('@') {
            let name_part = s[..angle_start].trim().trim_matches('"').to_string();
            let name = if name_part.is_empty() {
                None
            } else {
                Some(name_part)
            };
            return Some((name, email));
        }
    } else if s.contains('@') {
        return Some((None, s.to_lowercase()));
    }
    None
}
