/*! Local conversation merges: group separate threads without changing senders. */

use crate::db::thread_merges;

use super::model::App;

impl App {
    /// Recompute the merged roots for the loaded accounts.
    pub(crate) fn load_merge_roots(&mut self, accounts: &[String]) {
        self.merge_roots.clear();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        for account in accounts {
            if let Ok(roots) = thread_merges::roots_for_account(conn, account) {
                self.merge_roots.extend(roots);
            }
        }
    }

    /// Begin, complete, or undo a merge for the cursor's conversation.
    pub(crate) fn merge_conversation(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let Some(anchors) = self.conversation_anchors.get(&envelope.id).cloned() else {
            self.set_status("This message has no conversation identity.");
            return;
        };
        let Some(root) = anchors.first().cloned() else {
            return;
        };
        let account = envelope
            .account
            .clone()
            .or_else(|| self.acct_owned())
            .unwrap_or_default();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        // An already-merged conversation is unmerged first.
        if self.merge_roots.contains_key(&root) {
            match thread_merges::clear(conn, &account, &root) {
                Ok(_) => {
                    self.pending_merge = None;
                    self.set_status("Conversation split back out.");
                    self.load_envelopes();
                }
                Err(error) => self.set_error(&format!("Could not split: {error}")),
            }
            return;
        }
        match self.pending_merge.take() {
            None => {
                self.pending_merge = Some((account, root));
                self.set_status(
                    "Select another conversation and press Ctrl+t to merge it into this one.",
                );
            }
            Some((source_account, target)) if source_account == account && target != root => {
                // The marked conversation becomes the root; the cursor joins it.
                if let Err(error) = thread_merges::set(conn, &account, &root, &target) {
                    self.set_error(&format!("Could not merge: {error}"));
                    return;
                }
                self.set_status("Conversations merged.");
                self.load_envelopes();
            }
            Some((source_account, target)) => {
                if target == root {
                    self.set_status("Merge cancelled.");
                } else {
                    self.pending_merge = Some((source_account, target));
                    self.set_status("Merging needs a conversation in the same account.");
                }
            }
        }
    }
}
