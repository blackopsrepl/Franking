/*! Incremental folder search in the folder sidebar. */

use crate::keys::View;

use super::model::App;

impl App {
    /// Extend the folder query and jump to the first match.
    pub(crate) fn folder_jump_input(&mut self, c: char) {
        self.folder_jump.push(c);
        self.select_folder_jump_match();
    }

    pub(crate) fn folder_jump_backspace(&mut self) {
        self.folder_jump.pop();
        self.select_folder_jump_match();
    }

    /// Clear the query, or leave the sidebar when it is already empty.
    pub(crate) fn folder_jump_clear(&mut self) {
        if self.folder_jump.is_empty() {
            self.view = View::EnvelopeList;
            return;
        }
        self.folder_jump.clear();
        self.set_status("");
    }

    fn select_folder_jump_match(&mut self) {
        let query = self.folder_jump.to_ascii_lowercase();
        if query.is_empty() {
            return;
        }
        match self
            .folders
            .iter()
            .position(|folder| folder.name.to_ascii_lowercase().starts_with(&query))
        {
            Some(index) => {
                self.folder_index = index;
                let name = self.folders[index].name.clone();
                self.set_status(&format!("Folder: {name}"));
            }
            None => self.set_status(&format!("No folder starts with {query}")),
        }
    }
}
