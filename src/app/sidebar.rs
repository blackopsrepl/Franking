/*! Sidebar rows: server folders followed by the local views. */

use crate::keys::View;

use super::model::{App, LocalView};

impl App {
    pub(crate) fn sidebar_len(&self) -> usize {
        self.folders.len() + 2
    }

    pub(crate) fn sidebar_active_index(&self) -> Option<usize> {
        match self.local_view {
            Some(LocalView::ReplyLater) => Some(self.folders.len()),
            Some(LocalView::Starred) => Some(self.folders.len() + 1),
            None => self
                .folders
                .iter()
                .position(|folder| folder.name == self.current_folder),
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
}
