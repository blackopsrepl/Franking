/*! Per-account policy values that are not secrets.
The screening bypass token is a convenience word a trusted stranger can put in
a subject line to reach the Inbox without being approved as a sender. */

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

/// Read an account's bypass token, if one is set.
pub fn bypass_token(conn: &Connection, account: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT bypass_token FROM account_policy WHERE account = ?1",
            [account],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .filter(|token| !token.is_empty()))
}

/// Set or clear an account's bypass token.
pub fn set_bypass_token(conn: &Connection, account: &str, token: Option<&str>) -> Result<()> {
    let token = token.map(str::trim).filter(|token| !token.is_empty());
    match token {
        Some(token) => conn.execute(
            "INSERT INTO account_policy (account, bypass_token) VALUES (?1, ?2)
             ON CONFLICT(account) DO UPDATE SET bypass_token = excluded.bypass_token",
            params![account, token],
        )?,
        None => conn.execute("DELETE FROM account_policy WHERE account = ?1", [account])?,
    };
    Ok(())
}

/// Whether the token appears as its own word in the subject.
///
/// Matching is case-insensitive and requires token boundaries, so `Secret12`
/// does not match `Secret123`.
pub fn subject_has_token(subject: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let subject = subject.to_ascii_lowercase();
    let token = token.to_ascii_lowercase();
    let bytes = subject.as_bytes();
    let mut start = 0;
    while let Some(found) = subject[start..].find(&token) {
        let at = start + found;
        let before_ok = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let after = at + token.len();
        let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
        if start >= subject.len() {
            break;
        }
    }
    false
}

/// Generate a short, readable bypass token.
pub fn generate_token() -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_round_trips_per_account() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        assert_eq!(bypass_token(&conn, "work").unwrap(), None);
        set_bypass_token(&conn, "work", Some("Red87")).unwrap();
        assert_eq!(
            bypass_token(&conn, "work").unwrap().as_deref(),
            Some("Red87")
        );
        assert_eq!(bypass_token(&conn, "personal").unwrap(), None);
        set_bypass_token(&conn, "work", None).unwrap();
        assert_eq!(bypass_token(&conn, "work").unwrap(), None);
    }

    #[test]
    fn a_token_matches_only_as_a_whole_word() {
        assert!(subject_has_token("Please use Red87 to reach me", "Red87"));
        assert!(subject_has_token("red87", "Red87"));
        assert!(
            !subject_has_token("yoRed87abc", "Red87"),
            "no word boundary"
        );
        assert!(!subject_has_token("Red870", "Red87"), "trailing digit");
        assert!(!subject_has_token("anything", ""));
    }

    #[test]
    fn generated_tokens_avoid_ambiguous_glyphs() {
        let token = generate_token();
        assert_eq!(token.len(), 8);
        assert!(!token.contains('0') && !token.contains('O') && !token.contains('1'));
    }
}
