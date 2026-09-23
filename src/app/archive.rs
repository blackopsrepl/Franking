/*! Archive the selected messages into the account's archive folder. */

use crate::mail::types::FolderRole;

use super::model::App;
use super::undo::UndoOp;

impl App {
    /// Folder messages are archived to: the `\Archive` role, else a folder
    /// literally named "Archive".
    pub(crate) fn archive_folder(&self) -> Option<String> {
        self.folders
            .iter()
            .find(|folder| folder.role == FolderRole::Archive)
            .or_else(|| {
                self.folders
                    .iter()
                    .find(|folder| folder.name.eq_ignore_ascii_case("archive"))
            })
            .map(|folder| folder.name.clone())
    }

    /// Move the selection (or cursor row) to the archive folder.
    pub(crate) fn archive(&mut self) {
        if !self.can_mutate_selected_mailbox() {
            return;
        }
        let Some(target) = self.archive_folder() else {
            self.set_status("This account has no archive folder.");
            return;
        };
        let from = self.selected_folder();
        if from.eq_ignore_ascii_case(&target) {
            self.set_status("Already in the archive.");
            return;
        }
        let ids = self.target_ids();
        let Some(id) = ids.first().cloned() else {
            return;
        };

        self.loading = true;
        self.pending_refresh_after_action = true;
        self.pending_undo = Some(UndoOp::Move {
            from: from.clone(),
            to: target.clone(),
            ids: ids.clone(),
        });
        let account = self.acct_owned();
        if ids.len() == 1 {
            self.worker.move_message(account, from, target, id);
        } else {
            self.selected.clear();
            self.worker.move_messages(account, from, target, ids);
        }
    }
}
