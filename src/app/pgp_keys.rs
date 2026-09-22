/*! Key material management: listing, import, generation, export, deletion. */

use crate::keys::View;
use crate::mail::pgp::{self, KeyInfo};

use super::model::App;

/// What the text prompt is currently collecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPrompt {
    /// Path of a file holding an armored public key.
    ImportPath,
    /// Identity (`Name <email>`) for a new key pair.
    GenerateIdentity,
}

/// State of the key-material overlay.
#[derive(Default)]
pub struct KeysState {
    /// Keys found on disk, newest listing first.
    pub keys: Vec<KeyInfo>,
    /// Selected row.
    pub index: usize,
    /// What the prompt is collecting, when it is open.
    pub prompt: Option<KeyPrompt>,
    /// Prompt text being typed.
    pub input: String,
    /// Fingerprint awaiting a second delete press.
    pub pending_delete: Option<String>,
}

impl KeysState {
    /// The selected key, if the listing is not empty.
    pub fn selected(&self) -> Option<&KeyInfo> {
        self.keys.get(self.index)
    }
}

impl App {
    /// Dispatch one key-overlay action.
    pub(crate) fn handle_keys_action(&mut self, action: crate::keys::Action) {
        use crate::keys::Action;

        match action {
            Action::OpenKeys => self.open_keys(),
            Action::KeysNext => self.keys_next(),
            Action::KeysPrev => self.keys_prev(),
            Action::KeysImport => self.keys_import(),
            Action::KeysGenerate => self.keys_generate(),
            Action::KeysExport => self.keys_export(),
            Action::KeysDelete => self.keys_delete(),
            Action::KeysInput(c) => self.keys_input(c),
            Action::KeysBackspace => self.keys_backspace(),
            Action::KeysSubmit => self.keys_submit(),
            Action::KeysCancel => self.keys_cancel(),
            Action::KeysClose => self.close_keys(),
            _ => {}
        }
    }

    /// Open the key overlay, listing what is on disk.
    pub(crate) fn open_keys(&mut self) {
        self.status_message.clear();
        self.status_is_error = false;
        self.keys.keys = pgp::list_keys(&super::pgp::keys_dir());
        self.keys.index = self.keys.index.min(self.keys.keys.len().saturating_sub(1));
        self.keys.pending_delete = None;
        self.view = View::Keys;
    }

    /// Reload the listing without changing the view.
    pub(crate) fn refresh_keys(&mut self) {
        self.keys.keys = pgp::list_keys(&super::pgp::keys_dir());
        if self.keys.keys.is_empty() {
            self.keys.index = 0;
        } else {
            self.keys.index = self.keys.index.min(self.keys.keys.len() - 1);
        }
    }

    pub(crate) fn keys_next(&mut self) {
        if !self.keys.keys.is_empty() {
            self.keys.index = (self.keys.index + 1).min(self.keys.keys.len() - 1);
        }
    }

    pub(crate) fn keys_prev(&mut self) {
        self.keys.index = self.keys.index.saturating_sub(1);
    }

    /// Ask for the path of a public key to import.
    pub(crate) fn keys_import(&mut self) {
        self.keys.prompt = Some(KeyPrompt::ImportPath);
        self.keys.input.clear();
        self.view = View::KeysPrompt;
    }

    /// Ask for the identity to generate a key pair for.
    pub(crate) fn keys_generate(&mut self) {
        self.keys.prompt = Some(KeyPrompt::GenerateIdentity);
        self.keys.input.clear();
        self.view = View::KeysPrompt;
    }

    pub(crate) fn keys_input(&mut self, c: char) {
        self.keys.input.push(c);
    }

    pub(crate) fn keys_backspace(&mut self) {
        self.keys.input.pop();
    }

    /// Leave the key overlay.
    pub(crate) fn close_keys(&mut self) {
        self.keys.prompt = None;
        self.keys.input.clear();
        self.view = View::EnvelopeList;
    }

    /// Close the prompt without acting, staying on the key list.
    pub(crate) fn keys_cancel(&mut self) {
        self.keys.prompt = None;
        self.keys.input.clear();
        self.view = View::Keys;
    }

    /// Carry out the prompt's request.
    pub(crate) fn keys_submit(&mut self) {
        let Some(prompt) = self.keys.prompt.take() else {
            return;
        };
        let input = self.keys.input.trim().to_string();
        self.keys.input.clear();
        self.view = View::Keys;
        if input.is_empty() {
            self.set_error("A value is required.");
            return;
        }

        match prompt {
            KeyPrompt::ImportPath => self.import_key(&input),
            KeyPrompt::GenerateIdentity => self.generate_key(&input),
        }
    }

    fn import_key(&mut self, path: &str) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.set_error(&format!("Could not read {path}: {error}"));
                return;
            }
        };
        match pgp::import_public_key(&super::pgp::keys_dir(), &bytes) {
            Ok(info) => {
                self.refresh_keys();
                self.set_status(&format!("Imported the key for {}.", info.identity));
            }
            Err(error) => self.set_error(&format!("Could not import the key: {error}")),
        }
    }

    fn generate_key(&mut self, identity: &str) {
        match pgp::generate_keypair(identity) {
            Ok((secret, public)) => {
                let name = key_file_stem(identity);
                match pgp::write_keypair(&super::pgp::keys_dir(), &name, &secret, &public) {
                    Ok(()) => {
                        self.refresh_keys();
                        self.set_status(&format!("Generated a key pair for {identity}."));
                    }
                    Err(error) => self.set_error(&format!("Could not store the key: {error}")),
                }
            }
            Err(error) => self.set_error(&format!("Could not generate a key: {error}")),
        }
    }

    /// Write the selected public key where the user can pick it up.
    pub(crate) fn keys_export(&mut self) {
        let Some(key) = self.keys.selected() else {
            self.set_status("No key selected to export.");
            return;
        };
        if key.secret {
            self.set_status("Only public keys are exported; select the public half.");
            return;
        }
        let fingerprint = key.fingerprint.clone();
        match pgp::export_public_key(&super::pgp::keys_dir(), &fingerprint) {
            Ok(armored) => {
                let path = super::pgp::keys_dir().join(format!("{fingerprint}.export.asc"));
                match std::fs::write(&path, armored) {
                    Ok(()) => {
                        self.set_status(&format!("Exported the public key to {}.", path.display()))
                    }
                    Err(error) => self.set_error(&format!("Could not write the export: {error}")),
                }
            }
            Err(error) => self.set_error(&format!("Could not export the key: {error}")),
        }
    }

    /// Delete the selected key after a second press.
    pub(crate) fn keys_delete(&mut self) {
        let Some(key) = self.keys.selected() else {
            return;
        };
        let fingerprint = key.fingerprint.clone();
        if self.keys.pending_delete.as_deref() != Some(fingerprint.as_str()) {
            self.keys.pending_delete = Some(fingerprint);
            self.set_status("Press d again to delete this key.");
            return;
        }
        self.keys.pending_delete = None;
        match pgp::delete_key(&super::pgp::keys_dir(), &fingerprint) {
            Ok(removed) => {
                self.refresh_keys();
                self.set_status(&format!("Removed {removed} key file(s)."));
            }
            Err(error) => self.set_error(&format!("Could not delete the key: {error}")),
        }
    }
}

/// A file name stem for a generated key.
fn key_file_stem(identity: &str) -> String {
    identity
        .split('@')
        .next()
        .unwrap_or("key")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}
