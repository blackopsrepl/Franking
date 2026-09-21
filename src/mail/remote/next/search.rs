/*! The app's search grammar, expressed as typed IMAP search keys. */

use imap_types::core::{AString, Vec1};
use imap_types::search::SearchKey;

/// Translate a query into search keys.
///
/// Grammar: free text searches headers and body (TEXT); `subject X`, `from X`,
/// `to X`, and `body X` narrow to one field; `flag seen|flagged` and their
/// `not flag …` forms map to the flag keys; terms combine with ` and `.
pub fn criteria(query: Option<&str>) -> Vec1<SearchKey<'static>> {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return Vec1::from(SearchKey::All);
    };

    let keys: Vec<SearchKey<'static>> = query
        .split(" and ")
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(term)
        .collect();

    match Vec1::try_from(keys) {
        Ok(keys) => keys,
        Err(_) => Vec1::from(SearchKey::All),
    }
}

fn term(term: &str) -> SearchKey<'static> {
    let lower = term.to_ascii_lowercase();
    match lower.as_str() {
        "flag seen" => SearchKey::Seen,
        "not flag seen" => SearchKey::Unseen,
        "flag flagged" => SearchKey::Flagged,
        "not flag flagged" => SearchKey::Unflagged,
        "flag answered" => SearchKey::Answered,
        "not flag answered" => SearchKey::Unanswered,
        _ => {
            if let Some(value) = strip_prefix_ci(term, "subject ") {
                SearchKey::Subject(text(value))
            } else if let Some(value) = strip_prefix_ci(term, "from ") {
                SearchKey::From(text(value))
            } else if let Some(value) = strip_prefix_ci(term, "to ") {
                SearchKey::To(text(value))
            } else if let Some(value) = strip_prefix_ci(term, "body ") {
                SearchKey::Body(text(value))
            } else {
                SearchKey::Text(text(term))
            }
        }
    }
}

fn strip_prefix_ci<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let start = value.len().checked_sub(prefix.len())?;
    if value.is_char_boundary(start) && value[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&value[prefix.len()..])
    } else {
        None
    }
}

fn text(value: &str) -> AString<'static> {
    AString::try_from(value.to_string())
        .unwrap_or_else(|_| AString::try_from("search".to_string()).expect("static search text"))
}
