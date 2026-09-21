/*! Move-to-folder prompt with a folder picker. */

use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn enter_move_prompt(&mut self) {
        self.open_folder_picker(false);
    }

    /// Open the folder picker in copy mode.
    pub(crate) fn enter_copy_prompt(&mut self) {
        self.open_folder_picker(true);
    }

    fn open_folder_picker(&mut self, copy: bool) {
        if self.selected_envelope_id().is_some() {
            self.move_target.clear();
            self.move_index = 0;
            self.move_is_copy = copy;
            self.view = View::MovePrompt;
        }
    }

    /// Folders matching the typed filter, excluding the current folder.
    pub(crate) fn move_candidates(&self) -> Vec<String> {
        let filter = self.move_target.trim().to_ascii_lowercase();
        self.folders
            .iter()
            .map(|folder| folder.name.clone())
            .filter(|name| !name.eq_ignore_ascii_case(&self.current_folder))
            .filter(|name| filter.is_empty() || name.to_ascii_lowercase().contains(&filter))
            .collect()
    }

    pub(crate) fn move_next_candidate(&mut self) {
        let count = self.move_candidates().len();
        if count > 0 {
            self.move_index = (self.move_index + 1).min(count - 1);
        }
    }

    pub(crate) fn move_prev_candidate(&mut self) {
        self.move_index = self.move_index.saturating_sub(1);
    }

    /// Folder the prompt will move to: the highlighted candidate, else the
    /// typed text.
    pub(crate) fn resolved_move_target(&self) -> Option<String> {
        let candidates = self.move_candidates();
        if let Some(name) = candidates.get(self.move_index) {
            return Some(name.clone());
        }
        let typed = self.move_target.trim();
        (!typed.is_empty()).then(|| typed.to_string())
    }

    pub(crate) fn submit_move(&mut self) {
        let Some(target) = self.resolved_move_target() else {
            self.set_status("No target folder specified.");
            self.view = View::EnvelopeList;
            return;
        };
        let ids = self.target_ids();
        if let Some(id) = ids.first().cloned() {
            self.loading = true;
            self.pending_refresh_after_action = true;
            self.view = View::EnvelopeList;
            let account = self.acct_owned();
            let folder = self.current_folder.clone();

            if self.move_is_copy {
                // A copy leaves the original in place, so there is nothing to undo.
                if ids.len() == 1 {
                    self.worker.copy_message(account, folder, target, id);
                } else {
                    self.selected.clear();
                    self.worker.copy_messages(account, folder, target, ids);
                }
                return;
            }

            self.pending_undo = Some(super::undo::UndoOp::Move {
                from: folder.clone(),
                to: target.clone(),
                ids: ids.clone(),
            });
            if ids.len() == 1 {
                self.worker.move_message(account, folder, target, id);
            } else {
                self.selected.clear();
                self.worker.move_messages(account, folder, target, ids);
            }
        }
    }

    pub(crate) fn cancel_move(&mut self) {
        self.view = View::EnvelopeList;
    }

    pub(crate) fn move_prompt_input(&mut self, c: char) {
        self.move_target.push(c);
        self.move_index = 0;
    }

    pub(crate) fn move_prompt_backspace(&mut self) {
        self.move_target.pop();
        self.move_index = 0;
    }
}
