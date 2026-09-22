/*! Server-side Sieve filter management. */

use crossterm::event::KeyEvent;

use crate::compose_editor::ComposeEditor;
use crate::keys::View;

use super::model::App;

/// State for the Sieve script browser and editor.
#[derive(Default)]
pub struct SieveState {
    /// Scripts last returned by the server.
    pub scripts: Vec<crate::mail::sieve::SieveScript>,
    /// Selected script index.
    pub index: usize,
    /// Pending new-script name.
    pub name: String,
    /// Script editor for the script being created or edited.
    pub editor: Option<ComposeEditor>,
    /// Name of the script in the editor.
    pub editor_name: String,
    /// Script awaiting a second delete press.
    pub pending_delete: Option<String>,
    /// Whether to reload the script list after the next action completes.
    pub pending_refresh: bool,
    /// Whether the name prompt is renaming the selected script.
    pub renaming: bool,
}

/// Starter body for a newly created script.
const NEW_SCRIPT_TEMPLATE: &str = "# SolverForge Mail filter\nrequire [\"fileinto\"];\n\n# if header :contains \"from\" \"alice@example.com\" {\n#     fileinto \"Alice\";\n#     stop;\n# }\n";

impl App {
    /// Open the Sieve script browser, reloading the script list.
    pub(crate) fn open_sieve(&mut self) {
        self.status_message.clear();
        self.status_is_error = false;
        self.loading = true;
        self.view = View::SieveScripts;
        let account = self.acct_owned();
        self.worker.fetch_sieve_scripts(account);
    }

    pub(crate) fn sieve_next(&mut self) {
        if !self.sieve.scripts.is_empty() {
            self.sieve.index = (self.sieve.index + 1).min(self.sieve.scripts.len() - 1);
        }
    }

    pub(crate) fn sieve_prev(&mut self) {
        self.sieve.index = self.sieve.index.saturating_sub(1);
    }

    fn selected_script(&self) -> Option<&crate::mail::sieve::SieveScript> {
        self.sieve.scripts.get(self.sieve.index)
    }

    /// Activate the selected script, or deactivate every script when the
    /// selected one is already active.
    pub(crate) fn sieve_activate(&mut self) {
        let Some(script) = self.selected_script() else {
            return;
        };
        let account = self.acct_owned();
        if script.active {
            self.loading = true;
            self.sieve.pending_refresh = true;
            self.worker.sieve_set_active(account, None);
        } else {
            let name = script.name.clone();
            self.loading = true;
            self.sieve.pending_refresh = true;
            self.worker.sieve_set_active(account, Some(name));
        }
    }

    pub(crate) fn sieve_deactivate(&mut self) {
        self.loading = true;
        self.sieve.pending_refresh = true;
        let account = self.acct_owned();
        self.worker.sieve_set_active(account, None);
    }

    /// Load the selected script into the editor.
    pub(crate) fn sieve_edit(&mut self) {
        let Some(script) = self.selected_script() else {
            return;
        };
        let name = script.name.clone();
        self.loading = true;
        let account = self.acct_owned();
        self.worker.fetch_sieve_script(account, name);
    }

    /// Prompt for a name for a new script.
    pub(crate) fn sieve_new(&mut self) {
        self.sieve.name = "solverforge".to_string();
        self.sieve.renaming = false;
        self.view = View::SieveName;
    }

    /// Prompt for a new name for the selected script.
    pub(crate) fn sieve_rename(&mut self) {
        let Some(script) = self.selected_script() else {
            return;
        };
        self.sieve.name = script.name.clone();
        self.sieve.renaming = true;
        self.view = View::SieveName;
    }

    pub(crate) fn sieve_name_input(&mut self, c: char) {
        self.sieve.name.push(c);
    }

    pub(crate) fn sieve_name_backspace(&mut self) {
        self.sieve.name.pop();
    }

    pub(crate) fn sieve_name_cancel(&mut self) {
        self.view = View::SieveScripts;
    }

    /// Open the editor for the requested new script, or rename the selected one.
    pub(crate) fn sieve_name_submit(&mut self) {
        let name = self.sieve.name.trim().to_string();
        if name.is_empty() {
            self.set_error("A script name is required.");
            return;
        }
        if self.sieve.renaming {
            self.sieve.renaming = false;
            let Some(script) = self.selected_script().map(|s| s.name.clone()) else {
                self.view = View::SieveScripts;
                return;
            };
            if name == script {
                self.view = View::SieveScripts;
                self.set_status("Script name unchanged.");
                return;
            }
            self.loading = true;
            self.sieve.pending_refresh = true;
            self.view = View::SieveScripts;
            let account = self.acct_owned();
            self.worker.sieve_rename_script(account, script, name);
            return;
        }
        self.sieve.editor_name = name;
        self.sieve.editor = Some(ComposeEditor::from_text(NEW_SCRIPT_TEMPLATE));
        self.view = View::SieveEdit;
    }

    /// Delete the selected script after a second confirmation press.
    pub(crate) fn sieve_delete(&mut self) {
        let Some(script) = self.selected_script() else {
            return;
        };
        let name = script.name.clone();
        if self.sieve.pending_delete.as_deref() == Some(name.as_str()) {
            self.sieve.pending_delete = None;
            self.loading = true;
            self.sieve.pending_refresh = true;
            let account = self.acct_owned();
            self.worker.sieve_delete_script(account, name);
        } else {
            self.sieve.pending_delete = Some(name.clone());
            self.set_status(&format!("Press d again to delete {name}."));
        }
    }

    /// Save the editor contents as the current script.
    pub(crate) fn sieve_save(&mut self) {
        let Some(editor) = self.sieve.editor.as_ref() else {
            return;
        };
        let body = editor.text();
        let name = self.sieve.editor_name.clone();
        self.loading = true;
        self.sieve.pending_refresh = true;
        self.view = View::SieveScripts;
        let account = self.acct_owned();
        self.worker.sieve_save_script(account, name, body);
    }

    /// Forward a key to the script editor.
    pub(crate) fn sieve_editor_key(&mut self, key: KeyEvent) {
        if let Some(editor) = self.sieve.editor.as_mut() {
            editor.handle_key(key);
        }
    }

    /// Leave the editor, clearing an active editor search first.
    pub(crate) fn sieve_escape(&mut self) {
        let searching = self
            .sieve
            .editor
            .as_ref()
            .is_some_and(ComposeEditor::is_search_active);
        if searching {
            if let Some(editor) = self.sieve.editor.as_mut() {
                editor.clear_search();
            }
            return;
        }
        self.sieve.editor = None;
        self.view = View::SieveScripts;
    }

    pub(crate) fn close_sieve(&mut self) {
        self.sieve.editor = None;
        self.sieve.pending_delete = None;
        self.view = View::EnvelopeList;
    }
}

impl App {
    /// Adopt the script list returned by the server.
    pub(crate) fn handle_sieve_scripts(
        &mut self,
        result: Result<Vec<crate::mail::sieve::SieveScript>, crate::mail::MailError>,
    ) {
        self.loading = false;
        match result {
            Ok(scripts) => {
                self.sieve.scripts = scripts;
                if self.sieve.index >= self.sieve.scripts.len() {
                    self.sieve.index = self.sieve.scripts.len().saturating_sub(1);
                }
            }
            Err(error) => self.set_error(&format!("Sieve: {error}")),
        }
    }

    /// Open the editor for a script fetched from the server.
    pub(crate) fn handle_sieve_body(
        &mut self,
        name: String,
        result: Result<String, crate::mail::MailError>,
    ) {
        self.loading = false;
        match result {
            Ok(body) => {
                self.status_message.clear();
                self.status_is_error = false;
                self.sieve.editor_name = name;
                self.sieve.editor = Some(ComposeEditor::from_text(&body));
                self.view = View::SieveEdit;
            }
            Err(error) => self.set_error(&format!("Sieve: {error}")),
        }
    }
}
