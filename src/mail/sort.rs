/*! Client-side ordering of a listed page of envelopes. */

use imap_types::extensions::sort::SortCriterion;

use super::types::Envelope;

use imap_types::extensions::sort as imap_sort;

/// Field the message list is ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Date header string, as the backend supplied it.
    #[default]
    Date,
    Sender,
    Subject,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Date => "date",
            SortKey::Sender => "sender",
            SortKey::Subject => "subject",
        }
    }

    /// Next field in the cycle.
    pub fn next(self) -> Self {
        match self {
            SortKey::Date => SortKey::Sender,
            SortKey::Sender => SortKey::Subject,
            SortKey::Subject => SortKey::Date,
        }
    }
}

/// Current ordering of the message list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOrder {
    pub key: SortKey,
    pub descending: bool,
}

impl Default for SortOrder {
    /// Newest first, matching what servers return by default.
    fn default() -> Self {
        Self {
            key: SortKey::Date,
            descending: true,
        }
    }
}

impl SortOrder {
    /// Advance through key/direction combinations.
    pub fn cycle(self) -> Self {
        if !self.descending {
            SortOrder {
                key: self.key.next(),
                descending: true,
            }
        } else {
            SortOrder {
                key: self.key,
                descending: false,
            }
        }
    }

    /// Human description, e.g. `date ↓`.
    pub fn label(self) -> String {
        format!(
            "{} {}",
            self.key.label(),
            if self.descending { "↓" } else { "↑" }
        )
    }

    /// The server-side SORT criteria for this ordering.
    ///
    /// A server-side sort is what makes ordering meaningful beyond the fetched
    /// page; the local `apply` remains for backends and servers without SORT.
    pub fn criteria(self) -> imap_types::core::Vec1<SortCriterion> {
        imap_types::core::Vec1::from(SortCriterion {
            key: match self.key {
                SortKey::Date => imap_sort::SortKey::Date,
                SortKey::Sender => imap_sort::SortKey::From,
                SortKey::Subject => imap_sort::SortKey::Subject,
            },
            reverse: self.descending,
        })
    }

    /// Order `envelopes` in place, keeping equal keys in their listed order.
    pub fn apply(self, envelopes: &mut [Envelope]) {
        match self.key {
            SortKey::Date => {
                if self.descending {
                    envelopes.sort_by(|a, b| b.date.cmp(&a.date));
                } else {
                    envelopes.sort_by(|a, b| a.date.cmp(&b.date));
                }
            }
            SortKey::Sender => {
                if self.descending {
                    envelopes.sort_by_key(|envelope| envelope.sender_display().to_lowercase());
                    envelopes.reverse();
                } else {
                    envelopes.sort_by_key(|envelope| envelope.sender_display().to_lowercase());
                }
            }
            SortKey::Subject => {
                if self.descending {
                    envelopes.sort_by_key(|envelope| envelope.subject.to_lowercase());
                    envelopes.reverse();
                } else {
                    envelopes.sort_by_key(|envelope| envelope.subject.to_lowercase());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SortKey, SortOrder};
    use crate::mail::types::{Envelope, Sender};

    fn envelope(id: &str, sender: &str, subject: &str, date: &str) -> Envelope {
        Envelope {
            id: id.to_string(),
            flags: Vec::new(),
            subject: subject.to_string(),
            sender: Sender::Plain(sender.to_string()),
            date: date.to_string(),
            message_id: None,
            in_reply_to: None,
            account: None,
            folder: None,
        }
    }

    fn list() -> Vec<Envelope> {
        vec![
            envelope("1", "carol@example.com", "Beta", "2026-01-02"),
            envelope("2", "alice@example.com", "gamma", "2026-01-03"),
            envelope("3", "bob@example.com", "Alpha", "2026-01-01"),
        ]
    }

    #[test]
    fn orders_by_date_both_ways() {
        let mut envelopes = list();
        SortOrder {
            key: SortKey::Date,
            descending: true,
        }
        .apply(&mut envelopes);
        let ids: Vec<&str> = envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["2", "1", "3"]);

        SortOrder {
            key: SortKey::Date,
            descending: false,
        }
        .apply(&mut envelopes);
        let ids: Vec<&str> = envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["3", "1", "2"]);
    }

    #[test]
    fn orders_by_sender_and_subject_case_insensitively() {
        let mut envelopes = list();
        SortOrder {
            key: SortKey::Sender,
            descending: false,
        }
        .apply(&mut envelopes);
        let ids: Vec<&str> = envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["2", "3", "1"]);

        SortOrder {
            key: SortKey::Subject,
            descending: false,
        }
        .apply(&mut envelopes);
        let ids: Vec<&str> = envelopes.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["3", "1", "2"]);
    }

    #[test]
    fn cycles_through_every_field_and_direction() {
        let mut order = SortOrder::default();
        let mut seen = vec![order];
        for _ in 0..6 {
            order = order.cycle();
            seen.push(order);
        }
        assert_eq!(order, SortOrder::default(), "returns to the start");
        assert!(seen
            .iter()
            .any(|order| order.key == SortKey::Subject && !order.descending));
    }
}
