/*! Search criteria translation and flag/uid helpers. */

use imap::types::Flag;

pub(super) fn search_criteria(query: Option<&str>) -> String {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return "ALL".to_string();
    };

    let mut criteria = Vec::new();
    for term in query.split(" and ") {
        let term = term.trim();
        match term.to_ascii_lowercase().as_str() {
            "flag seen" => criteria.push("SEEN".to_string()),
            "not flag seen" => criteria.push("UNSEEN".to_string()),
            "flag flagged" => criteria.push("FLAGGED".to_string()),
            "not flag flagged" => criteria.push("UNFLAGGED".to_string()),
            other => {
                if let Some(value) = other.strip_prefix("subject ") {
                    criteria.push(format!("SUBJECT {}", imap_quote(value.trim())));
                } else if let Some(value) = other.strip_prefix("from ") {
                    criteria.push(format!("FROM {}", imap_quote(value.trim())));
                } else {
                    criteria.push(format!("TEXT {}", imap_quote(other)));
                }
            }
        }
    }

    if criteria.is_empty() {
        "ALL".to_string()
    } else {
        criteria.join(" ")
    }
}

pub(super) fn imap_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub(super) fn uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn imap_flag(flag: &str) -> &str {
    if flag.eq_ignore_ascii_case("seen") {
        "\\Seen"
    } else if flag.eq_ignore_ascii_case("flagged") {
        "\\Flagged"
    } else if flag.eq_ignore_ascii_case("answered") {
        "\\Answered"
    } else if flag.eq_ignore_ascii_case("deleted") {
        "\\Deleted"
    } else {
        flag
    }
}

pub(super) fn imap_flag_name(flag: &Flag<'_>) -> String {
    match flag {
        Flag::Seen => "Seen".to_string(),
        Flag::Flagged => "Flagged".to_string(),
        Flag::Answered => "Answered".to_string(),
        Flag::Deleted => "Deleted".to_string(),
        Flag::Draft => "Draft".to_string(),
        Flag::Recent => "Recent".to_string(),
        Flag::MayCreate => "MayCreate".to_string(),
        Flag::Custom(value) => value.to_string(),
    }
}
