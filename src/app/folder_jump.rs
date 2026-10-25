/*! Incremental folder search in the folder sidebar. */

use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn sidebar_len(&self) -> usize {
        self.folders.len() + 2
    }

    pub(crate) fn sidebar_active_index(&self) -> Option<usize> {
        if self.starred_view {
            Some(self.folders.len() + 1)
        } else if self.followup_lane.is_some() {
            Some(self.folders.len())
        } else {
            self.folders
                .iter()
                .position(|folder| folder.name == self.current_folder)
        }
    }

    /// Deterministic viewport shared by rendering and mouse hit testing.
    /// No render-only ListState offset can get lost between frames.
    pub(crate) fn sidebar_offset(&self, visible_rows: usize) -> usize {
        let selected = if self.view == View::FolderList {
            Some(self.folder_index.min(self.sidebar_len() - 1))
        } else {
            self.sidebar_active_index()
        };
        selected
            .unwrap_or(0)
            .saturating_sub(visible_rows.saturating_sub(1))
    }

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
        let labels = self
            .folders
            .iter()
            .map(|folder| folder.name.as_str())
            .chain(["Reply later", "Starred (cached)"]);
        let found = labels.enumerate().find_map(|(index, label)| {
            label
                .to_ascii_lowercase()
                .starts_with(&query)
                .then(|| (index, label.to_owned()))
        });
        match found {
            Some((index, name)) => {
                self.folder_index = index;
                self.set_status(&format!("Folder: {name}"));
            }
            None => self.set_status(&format!("No folder starts with {query}")),
        }
    }
}
