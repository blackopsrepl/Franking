/* Contact import: vCard (.vcf) and CSV (Google Contacts / generic). */

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::contacts::{self, Contact};

/// Result of an import operation.
pub struct ImportResult {
    pub added: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// ── vCard (.vcf) ────────────────────────────────────────────────────────────

/// Parse a vCard file and import all contacts into the database.
/// vCard 3.0 / 4.0 format.
pub fn import_vcf(conn: &Connection, content: &str) -> Result<ImportResult> {
    let cards = parse_vcf(content);
    let mut result = ImportResult {
        added: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    for card in cards {
        match contacts::add(conn, &card) {
            Ok(_) => result.added += 1,
            Err(e) => {
                let msg = e.to_string();
                // Duplicate email: count as skipped
                if msg.contains("UNIQUE constraint") {
                    result.skipped += 1;
                } else {
                    result.errors.push(format!("{}: {}", card.email, e));
                }
            }
        }
    }
    Ok(result)
}

/// Parse a raw vCard string into a list of Contact structs.
/// Handles multiple VCARD blocks in one file.
fn parse_vcf(content: &str) -> Vec<Contact> {
    let mut contacts = Vec::new();
    let mut in_card = false;
    let mut name: Option<String> = None;
    let mut email: Option<String> = None;
    let mut phone: Option<String> = None;
    let mut org: Option<String> = None;

    for line in content.lines() {
        let line = line.trim_end_matches('\r');
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            in_card = true;
            name = None;
            email = None;
            phone = None;
            org = None;
            continue;
        }
        if line.eq_ignore_ascii_case("END:VCARD") {
            in_card = false;
            if let Some(addr) = email.take() {
                contacts.push(Contact {
                    id: 0,
                    name: name.take(),
                    email: addr,
                    phone: phone.take(),
                    org: org.take(),
                    notes: None,
                    harvested: false,
                    tags: Vec::new(),
                });
            }
            continue;
        }
        if !in_card {
            continue;
        }

        // Split on ':' - property:value (may have type params before ':')
        let colon = match line.find(':') {
            Some(i) => i,
            None => continue,
        };
        let prop = line[..colon].to_uppercase();
        let value = line[colon + 1..].trim();

        if value.is_empty() {
            continue;
        }

        // FN: formatted name (preferred over N:)
        if prop == "FN" {
            name = Some(value.to_string());
        }
        // N: structured name — family;given;... — use given + family
        else if prop == "N" && name.is_none() {
            let parts: Vec<&str> = value.splitn(5, ';').collect();
            let family = parts.first().copied().unwrap_or("").trim();
            let given = parts.get(1).copied().unwrap_or("").trim();
            let full = match (given.is_empty(), family.is_empty()) {
                (false, false) => format!("{given} {family}"),
                (false, true) => given.to_string(),
                (true, false) => family.to_string(),
                (true, true) => String::new(),
            };
            if !full.is_empty() {
                name = Some(full);
            }
        }
        // EMAIL (any type): take the first one
        else if prop.starts_with("EMAIL") && email.is_none() {
            if value.contains('@') {
                email = Some(value.to_lowercase());
            }
        }
        // TEL
        else if prop.starts_with("TEL") && phone.is_none() {
            phone = Some(value.to_string());
        }
        // ORG
        else if prop == "ORG" && org.is_none() {
            // ORG can be semicolon-separated: org name;dept;...
            let org_name = value.split(';').next().unwrap_or(value).trim();
            if !org_name.is_empty() {
                org = Some(org_name.to_string());
            }
        }
    }

    contacts
}

// ── CSV ─────────────────────────────────────────────────────────────────────

/// Parse a CSV file and import contacts.
///
/// Supported formats:
/// - Google Contacts export: `Name,Given Name,Additional Name,Family Name,...,E-mail 1 - Value,...`
/// - Generic: any CSV with `name` and `email` columns (case-insensitive headers)
pub fn import_csv(conn: &Connection, content: &str) -> Result<ImportResult> {
    let contacts = parse_csv(content).context("failed to parse CSV")?;
    let mut result = ImportResult {
        added: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    for contact in contacts {
        match contacts::add(conn, &contact) {
            Ok(_) => result.added += 1,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("UNIQUE constraint") {
                    result.skipped += 1;
                } else {
                    result.errors.push(format!("{}: {}", contact.email, e));
                }
            }
        }
    }
    Ok(result)
}

fn parse_csv(content: &str) -> Result<Vec<Contact>> {
    let mut lines = content.lines();

    // Header row
    let header = lines.next().context("CSV is empty")?;
    let headers: Vec<String> = split_csv_line(header)
        .into_iter()
        .map(|h| h.to_lowercase())
        .collect();

    // Find column indices
    let col = |names: &[&str]| -> Option<usize> {
        names
            .iter()
            .find_map(|n| headers.iter().position(|h| h.contains(n)))
    };

    let name_col = col(&["name", "full name"]);
    let given_col = col(&["given name", "first name"]);
    let family_col = col(&["family name", "last name", "surname"]);
    let email_col = col(&["e-mail 1 - value", "email 1", "email address", "email"]);
    let phone_col = col(&["phone 1 - value", "phone 1", "phone number", "phone"]);
    let org_col = col(&["organization 1 - name", "company", "organization", "org"]);

    let email_col = email_col.context("CSV has no recognizable email column")?;

    let mut contacts = Vec::new();

    for (line_num, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_csv_line(line);

        let get = |col: usize| -> Option<String> {
            fields
                .get(col)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let email = match get(email_col) {
            Some(e) if e.contains('@') => e.to_lowercase(),
            _ => continue, // skip rows with no valid email
        };

        // Determine display name
        let name = name_col.and_then(get).or_else(|| {
            // Construct from given + family
            let given = given_col.and_then(get);
            let family = family_col.and_then(get);
            match (given, family) {
                (Some(g), Some(f)) => Some(format!("{g} {f}")),
                (Some(g), None) => Some(g),
                (None, Some(f)) => Some(f),
                (None, None) => None,
            }
        });

        let phone = phone_col.and_then(get);
        let org = org_col.and_then(get);

        if name.is_none() && phone.is_none() && org.is_none() {
            // Only an email address — still useful
            let _ = line_num; // suppress warning
        }

        contacts.push(Contact {
            id: 0,
            name,
            email,
            phone,
            org,
            notes: None,
            harvested: false,
            tags: Vec::new(),
        });
    }

    Ok(contacts)
}

/// Minimal CSV line splitter. Handles quoted fields.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    // Escaped quote
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            c => current.push(c),
        }
    }
    fields.push(current);
    fields
}
