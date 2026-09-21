/*! Jumping between unread messages. */

use super::model::App;

impl App {
    /// Move the cursor to the next (`forward`) or previous unread message,
    /// wrapping around the list.
    pub(crate) fn jump_unread(&mut self, forward: bool) {
        let count = self.envelopes.len();
        if count == 0 {
            return;
        }
        let start = self.envelope_state.selected().unwrap_or(0);

        for offset in 1..=count {
            let index = if forward {
                (start + offset) % count
            } else {
                (start + count - (offset % count)) % count
            };
            if !self.envelopes[index].is_seen() {
                self.envelope_state.select(Some(index));
                return;
            }
        }
        self.set_status("No unread messages in this folder.");
    }
}
