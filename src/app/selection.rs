/*! Multi-select of messages for batch operations. */

use super::model::App;

impl App {
    /// Toggle the current row's selection and advance the cursor.
    pub(crate) fn toggle_select(&mut self) {
        if self.is_unified_inbox() {
            self.set_status("Batch selection across accounts needs account-aware actions.");
            return;
        }
        let Some(id) = self.selected_envelope_id().map(str::to_string) else {
            return;
        };
        if !self.selected.insert(id.clone()) {
            self.selected.remove(&id);
        }
        self.move_selection(1);
    }

    /// Drop every selected message.
    pub(crate) fn clear_selection(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.selected.clear();
        self.set_status("Selection cleared.");
    }

    /// Envelope ids a batch action applies to: the selection plus the cursor row.
    pub(crate) fn target_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.selected.iter().cloned().collect();
        if let Some(id) = self.selected_envelope_id().map(str::to_string) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.sort();
        ids
    }

    /// Prevent an operation from using a mailbox belonging to another account.
    pub(crate) fn can_mutate_selected_mailbox(&mut self) -> bool {
        if !self.is_unified_inbox() {
            return true;
        }
        let matching = self.selected.is_empty()
            && self.selected_envelope().and_then(|e| e.account.as_deref())
                == self.account_name.as_deref();
        if !matching {
            self.set_status("Switch to the message's account before changing its mailbox.");
        }
        matching
    }
}
