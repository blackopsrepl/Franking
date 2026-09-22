/*! Conversation nesting for the envelope list. */

use crate::mail::types::Envelope;

/// Nesting depth of each envelope within the visible set, following
/// In-Reply-To links to parents that are also present (capped for safety).
pub(super) fn thread_depths(envelopes: &[Envelope]) -> Vec<usize> {
    use std::collections::HashMap;

    let ids: HashMap<&str, usize> = envelopes
        .iter()
        .enumerate()
        .filter_map(|(index, envelope)| envelope.message_id.as_deref().map(|id| (id, index)))
        .collect();

    envelopes
        .iter()
        .map(|envelope| {
            let mut depth = 0;
            let mut current = envelope.in_reply_to.as_deref();
            while let Some(parent) = current {
                if depth >= 4 {
                    break;
                }
                match ids.get(parent) {
                    Some(&index) => {
                        depth += 1;
                        current = envelopes[index].in_reply_to.as_deref();
                    }
                    None => break,
                }
            }
            depth
        })
        .collect()
}
