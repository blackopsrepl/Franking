/*! Crash-safe autosave of the message being composed.
Autosaves go to a local file rather than the server's Drafts mailbox so a
periodic save never spams the account; the file is cleared once the message
is sent or discarded, and recovered on the next start. */

use std::path::{Path, PathBuf};

use crate::compose::{ComposeMode, ComposeState};
use crate::keys::View;

use super::model::App;

const DRAFT_FILE: &str = "unsent-message.mml";

/// Ticks per second (the tick rate is 250ms).
const TICKS_PER_SECOND: u64 = 4;

/// Directory used for autosaves.
pub(crate) fn default_dir() -> PathBuf {
    crate::brand::data_dir().join("compose")
}

/// Write the composed template, creating the directory when needed.
pub(crate) fn write(dir: &Path, template: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(DRAFT_FILE), template)
}

/// Read a previously autosaved template, if one exists and is not empty.
pub(crate) fn read(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(DRAFT_FILE)).ok()?;
    (!text.trim().is_empty()).then_some(text)
}

/// Remove the autosave, if present.
pub(crate) fn clear(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(DRAFT_FILE));
}

impl App {
    /// Autosave the message in progress at a fixed interval.
    pub(crate) fn autosave_tick(&mut self) {
        if self.autosave_seconds == 0 {
            return;
        }
        let composing = self
            .compose_state
            .as_ref()
            .is_some_and(|state| state.dirty || !state.body.is_empty());
        if !composing {
            self.autosave_ticks = 0;
            return;
        }
        self.autosave_ticks += 1;
        if self.autosave_ticks < self.autosave_seconds * TICKS_PER_SECOND {
            return;
        }
        self.autosave_ticks = 0;
        let state = self.compose_state.as_ref().expect("composing");
        let template = crate::compose::reassemble_template(state);
        let dir = self.autosave_dir.clone();
        if write(&dir, &template).is_ok() {
            self.set_status("Draft autosaved locally.");
        }
    }

    /// Remove the autosave, if one exists.
    pub(crate) fn clear_autosave(&mut self) {
        clear(&self.autosave_dir);
        self.autosave_ticks = 0;
    }

    /// Restore an autosaved message into a fresh compose session.
    pub(crate) fn recover_autosave(&mut self) -> bool {
        let Some(template) = read(&self.autosave_dir) else {
            return false;
        };
        let mut state = ComposeState::new(ComposeMode::New, self.acct_owned());
        self.load_identities_into(&mut state);
        crate::compose::populate_from_template(&mut state, &template);
        self.compose_state = Some(state);
        self.view = View::Compose;
        self.set_status("Recovered an unsent message.");
        true
    }
}
