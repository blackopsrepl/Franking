/*! Merging multi-folder search results into one list. */

use super::types::Envelope;

/// Combine per-folder results, newest first, dropping duplicates.
///
/// Ordering uses the raw date string the backend supplied, which is the same
/// value the list view displays.
pub fn merge_results(lists: Vec<Vec<Envelope>>) -> Vec<Envelope> {
    let mut merged: Vec<Envelope> = Vec::new();
    for list in lists {
        for envelope in list {
            let duplicate = merged.iter().any(|existing| {
                existing.id == envelope.id
                    && existing.folder.as_deref() == envelope.folder.as_deref()
            });
            if !duplicate {
                merged.push(envelope);
            }
        }
    }
    merged.sort_by(|left, right| right.date.cmp(&left.date));
    merged
}

#[cfg(test)]
mod tests {
    use super::merge_results;
    use crate::mail::types::{Envelope, Sender};

    fn envelope(id: &str, folder: &str, date: &str) -> Envelope {
        Envelope {
            id: id.to_string(),
            flags: Vec::new(),
            subject: format!("Subject {id}"),
            sender: Sender::Plain("alice@example.com".to_string()),
            date: date.to_string(),
            message_id: None,
            in_reply_to: None,
            account: None,
            folder: Some(folder.to_string()),
        }
    }

    #[test]
    fn merges_newest_first_and_drops_duplicates() {
        let merged = merge_results(vec![
            vec![
                envelope("1", "INBOX", "2026-01-02"),
                envelope("2", "INBOX", "2026-01-01"),
            ],
            vec![
                envelope("2", "INBOX", "2026-01-01"),
                envelope("3", "Archive", "2026-01-03"),
            ],
        ]);

        let ids: Vec<&str> = merged.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["3", "1", "2"], "newest first, no duplicates");
        assert_eq!(merged[2].folder.as_deref(), Some("INBOX"));
    }
}
