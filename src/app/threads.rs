/*! Collapsing and expanding conversation threads in the envelope list.
Threads arrive contiguously from server-side THREAD, so a collapsed thread
is hidden by removing the envelopes that follow its root until the root
changes; expanding puts the stashed envelopes back in order. */

use crate::keys::View;
use crate::mail::types::Envelope;

use super::model::App;

/// Stable key for the thread an envelope belongs to: the top-most present
/// ancestor's Message-ID, or the envelope's own id when it is a root.
fn root_key(envelopes: &[Envelope], index: usize) -> String {
    let by_id: std::collections::HashMap<&str, usize> = envelopes
        .iter()
        .enumerate()
        .filter_map(|(i, envelope)| envelope.message_id.as_deref().map(|id| (id, i)))
        .collect();

    let mut current = index;
    let mut hops = 0;
    while hops < 8 {
        let Some(parent) = envelopes[current].in_reply_to.as_deref() else {
            break;
        };
        let Some(&parent_index) = by_id.get(parent) else {
            break;
        };
        current = parent_index;
        hops += 1;
    }
    envelopes[current]
        .message_id
        .clone()
        .unwrap_or_else(|| envelopes[current].id.clone())
}

impl App {
    /// Thread key for every envelope currently listed.
    pub(crate) fn thread_root_keys(&self) -> Vec<String> {
        (0..self.envelopes.len())
            .map(|index| root_key(&self.envelopes, index))
            .collect()
    }

    /// Hide every message below the cursor's thread root.
    pub(crate) fn collapse_thread(&mut self) {
        if !self.threaded {
            self.set_status("Threading is off (press t).");
            return;
        }
        let Some(index) = self.envelope_state.selected() else {
            return;
        };
        if index >= self.envelopes.len() {
            return;
        }
        let key = root_key(&self.envelopes, index);
        let root = (0..=index)
            .rev()
            .find(|&i| root_key(&self.envelopes, i) == key)
            .unwrap_or(index);

        let hidden: Vec<Envelope> = {
            let mut end = root + 1;
            while end < self.envelopes.len() && root_key(&self.envelopes, end) == key {
                end += 1;
            }
            self.envelopes.drain(root + 1..end).collect()
        };

        if hidden.is_empty() {
            self.set_status("No replies to collapse.");
            return;
        }
        let count = hidden.len();
        self.collapsed_threads.insert(key, hidden);
        self.envelope_state
            .select(Some(root.min(self.envelopes.len().saturating_sub(1))));
        self.set_status(&format!("Collapsed {count} message(s)."));
    }

    /// Restore the messages hidden under the cursor's thread root.
    pub(crate) fn expand_thread(&mut self) {
        let Some(index) = self.envelope_state.selected() else {
            return;
        };
        if index >= self.envelopes.len() {
            return;
        }
        let key = root_key(&self.envelopes, index);
        let Some(hidden) = self.collapsed_threads.remove(&key) else {
            self.set_status("No collapsed thread here.");
            return;
        };
        let count = hidden.len();
        let at = index + 1;
        for (offset, envelope) in hidden.into_iter().enumerate() {
            self.envelopes.insert(at + offset, envelope);
        }
        self.set_status(&format!("Expanded {count} message(s)."));
    }

    /// Leave threading mode cleanly, restoring hidden envelopes.
    pub(crate) fn clear_collapsed_threads(&mut self) {
        if self.collapsed_threads.is_empty() {
            return;
        }
        let mut hidden: Vec<Envelope> = self
            .collapsed_threads
            .drain()
            .flat_map(|(_, envelopes)| envelopes)
            .collect();
        if !hidden.is_empty() {
            self.envelopes.append(&mut hidden);
        }
        self.view = View::EnvelopeList;
    }
}
