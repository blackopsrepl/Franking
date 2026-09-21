/*! Sieve action dispatch. */

use crate::keys::Action;

use super::model::App;

impl App {
    /// Handle the Sieve browser and editor actions.
    pub(crate) fn handle_sieve_action(&mut self, action: Action) {
        match action {
            Action::OpenSieve => self.open_sieve(),
            Action::SieveNext => self.sieve_next(),
            Action::SievePrev => self.sieve_prev(),
            Action::SieveActivate => self.sieve_activate(),
            Action::SieveDeactivate => self.sieve_deactivate(),
            Action::SieveEdit => self.sieve_edit(),
            Action::SieveNew => self.sieve_new(),
            Action::SieveRename => self.sieve_rename(),
            Action::SieveDelete => self.sieve_delete(),
            Action::SieveSave => self.sieve_save(),
            Action::SieveClose => self.close_sieve(),
            Action::SieveEscape => self.sieve_escape(),
            Action::SieveNameInput(c) => self.sieve_name_input(c),
            Action::SieveNameBackspace => self.sieve_name_backspace(),
            Action::SieveNameSubmit => self.sieve_name_submit(),
            Action::SieveNameCancel => self.sieve_name_cancel(),
            Action::SieveEditorKey(key) => self.sieve_editor_key(key),
            // ── Compose editor ───────────────────────────────────────
            _ => {}
        }
    }
}
