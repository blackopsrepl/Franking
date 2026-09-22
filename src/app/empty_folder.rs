/*! Empty the current folder, with a two-press confirmation. */

use crate::keys::View;

use super::model::App;

impl App {
    /// Empty the current folder; requires a second press to confirm.
    pub(crate) fn empty_folder(&mut self) {
        let folder = self.current_folder.clone();
        if self.pending_empty_folder.as_deref() != Some(folder.as_str()) {
            self.pending_empty_folder = Some(folder.clone());
            self.set_status(&format!(
                "Press E again to permanently delete every message in {folder}."
            ));
            return;
        }
        self.pending_empty_folder = None;
        self.loading = true;
        self.pending_refresh_after_action = true;
        self.pending_undo = None;
        self.view = View::EnvelopeList;
        let account = self.acct_owned();
        self.worker.empty_folder(account, folder);
    }
}
