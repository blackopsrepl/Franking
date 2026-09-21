/*! Key resolution for the Sieve filter screens. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;

pub(super) fn resolve_sieve_scripts(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::SieveNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SievePrev,
        KeyCode::Enter => Action::SieveActivate,
        KeyCode::Char('e') => Action::SieveEdit,
        KeyCode::Char('n') => Action::SieveNew,
        KeyCode::Char('d') => Action::SieveDelete,
        KeyCode::Char('r') => Action::SieveRename,
        KeyCode::Char('x') => Action::SieveDeactivate,
        KeyCode::Esc | KeyCode::Char('q') => Action::SieveClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_sieve_name(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SieveNameSubmit,
        KeyCode::Esc => Action::SieveNameCancel,
        KeyCode::Backspace => Action::SieveNameBackspace,
        KeyCode::Char(c) => Action::SieveNameInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_sieve_edit(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return Action::SieveSave;
    }
    if key.code == KeyCode::Esc {
        return Action::SieveEscape;
    }
    Action::SieveEditorKey(key)
}
