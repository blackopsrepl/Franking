/*! Undo for the last destructive mailbox action. */

use crate::mail::types::FolderRole;

use super::model::App;

/// A reversible mailbox operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UndoOp {
    /// Messages moved from `from` to `to`.
    Move {
        from: String,
        to: String,
        ids: Vec<String>,
    },
    /// One flag applied (`added`) or removed across `ids`.
    Flag {
        folder: String,
        ids: Vec<String>,
        flag: String,
        added: bool,
    },
}

impl App {
    /// Name of the account's Trash folder, if the folder list has one.
    pub(crate) fn trash_folder(&self) -> Option<String> {
        self.folders
            .iter()
            .find(|folder| folder.role == FolderRole::Trash)
            .map(|folder| folder.name.clone())
    }

    /// Reverse the recorded operation, if any.
    pub(crate) fn undo(&mut self) {
        let Some(operation) = self.pending_undo.take() else {
            self.set_status("Nothing to undo.");
            return;
        };
        let account = self.selected_account();
        self.loading = true;
        self.pending_refresh_after_action = true;
        match operation {
            UndoOp::Move { from, to, ids } => {
                self.worker.move_messages(account, to, from, ids);
            }
            UndoOp::Flag {
                folder,
                ids,
                flag,
                added,
            } => {
                self.worker
                    .flag_messages(account, folder, ids, flag, !added);
            }
        }
    }
}
