/*! Keys view bindings. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

/// The key list, and the text prompt it can raise.
pub(super) fn resolve_keys(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::KeysClose,
        KeyCode::Char('j') | KeyCode::Down => Action::KeysPrev,
        KeyCode::Char('k') | KeyCode::Up => Action::KeysNext,
        KeyCode::Char('i') => Action::KeysImport,
        KeyCode::Char('g') => Action::KeysGenerate,
        KeyCode::Char('x') => Action::KeysExport,
        KeyCode::Char('d') => Action::KeysDelete,
        _ => Action::None,
    }
}

/// The import-path or key-generation prompt.
pub(super) fn resolve_keys_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::KeysSubmit,
        KeyCode::Esc => Action::KeysCancel,
        KeyCode::Backspace => Action::KeysBackspace,
        KeyCode::Char(c) => Action::KeysInput(c),
        _ => Action::None,
    }
}
