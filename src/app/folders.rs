/*! Folder management: create, rename, and delete prompts. */

use crate::keys::View;

use super::model::App;

/// Which folder operation a prompt is collecting input for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderPromptKind {
    Create,
    Rename,
    Delete,
}

/// Pending folder-management prompt.
#[derive(Debug, Clone)]
pub struct FolderPrompt {
    pub kind: FolderPromptKind,
    pub input: String,
}

impl FolderPrompt {
    /// Status-bar label for the prompt.
    pub fn label(&self) -> &'static str {
        match self.kind {
            FolderPromptKind::Create => " New folder: ",
            FolderPromptKind::Rename => " Rename folder: ",
            FolderPromptKind::Delete => " Delete folder (type yes): ",
        }
    }

    /// Status-bar hint for the prompt.
    pub fn hint(&self) -> &'static str {
        match self.kind {
            FolderPromptKind::Create => "  (Enter to create, Esc to cancel)",
            FolderPromptKind::Rename => "  (Enter to rename, Esc to cancel)",
            FolderPromptKind::Delete => "  (Enter to delete, Esc to cancel)",
        }
    }
}

impl App {
    /// Subscribe to the current folder, or unsubscribe from it.
    pub(crate) fn toggle_folder_subscription(&mut self) {
        if self.current_folder == super::model::UNIFIED_INBOX {
            self.set_status("The unified inbox is not a mailbox on the server.");
            return;
        }
        let Some(folder) = self
            .folders
            .iter()
            .find(|folder| folder.name == self.current_folder)
        else {
            self.set_status("No folder is selected.");
            return;
        };
        let Some(subscribed) = folder.subscribed else {
            self.set_status("This backend does not report subscriptions.");
            return;
        };
        let name = folder.name.clone();
        self.loading = true;
        self.pending_folder_refresh = true;
        self.worker
            .set_folder_subscription(self.acct_owned(), name, !subscribed);
    }

    pub(crate) fn folder_prompt_new(&mut self) {
        self.open_folder_prompt(FolderPromptKind::Create);
    }

    pub(crate) fn folder_prompt_rename(&mut self) {
        self.open_folder_prompt(FolderPromptKind::Rename);
    }

    pub(crate) fn folder_prompt_delete(&mut self) {
        self.open_folder_prompt(FolderPromptKind::Delete);
    }

    fn open_folder_prompt(&mut self, kind: FolderPromptKind) {
        let input = match kind {
            FolderPromptKind::Rename => self.current_folder.clone(),
            _ => String::new(),
        };
        self.folder_prompt = Some(FolderPrompt { kind, input });
        self.view = View::FolderPrompt;
    }

    pub(crate) fn folder_prompt_input(&mut self, c: char) {
        if let Some(prompt) = self.folder_prompt.as_mut() {
            prompt.input.push(c);
        }
    }

    pub(crate) fn folder_prompt_backspace(&mut self) {
        if let Some(prompt) = self.folder_prompt.as_mut() {
            prompt.input.pop();
        }
    }

    pub(crate) fn cancel_folder_prompt(&mut self) {
        self.folder_prompt = None;
        self.view = View::FolderList;
    }

    /// Carry out the pending folder operation.
    pub(crate) fn submit_folder_prompt(&mut self) {
        let Some(prompt) = self.folder_prompt.take() else {
            return;
        };
        let input = prompt.input.trim().to_string();
        self.view = View::FolderList;
        self.pending_folder_refresh = true;

        match prompt.kind {
            FolderPromptKind::Create => {
                if input.is_empty() {
                    self.pending_folder_refresh = false;
                    self.set_error("A folder name is required.");
                    return;
                }
                self.loading = true;
                self.worker.create_folder(self.acct_owned(), input);
            }
            FolderPromptKind::Rename => {
                let from = self.current_folder.clone();
                if input.is_empty() || input == from {
                    self.pending_folder_refresh = false;
                    self.set_status("Folder name unchanged.");
                    return;
                }
                self.loading = true;
                self.worker.rename_folder(self.acct_owned(), from, input);
            }
            FolderPromptKind::Delete => {
                if !matches!(input.to_ascii_lowercase().as_str(), "y" | "yes") {
                    self.pending_folder_refresh = false;
                    self.set_status("Folder not deleted.");
                    return;
                }
                self.loading = true;
                self.worker
                    .delete_folder(self.acct_owned(), self.current_folder.clone());
            }
        }
    }
}
