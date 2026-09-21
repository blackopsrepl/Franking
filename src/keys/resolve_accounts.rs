/*! Key resolution for the account list and account form. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

pub(super) fn resolve_account_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::Back,
        KeyCode::Char('q') => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter => Action::Select,
        KeyCode::Char('s') => Action::SetDefaultAccount,
        KeyCode::Char('d') => Action::DeleteAccount,
        KeyCode::Char('a') => Action::AccountNew,
        KeyCode::Char('e') => Action::AccountEdit,
        _ => Action::None,
    }
}

pub(super) fn resolve_account_edit(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Tab => Action::AccountEditFieldNext,
        KeyCode::BackTab => Action::AccountEditFieldPrev,
        KeyCode::Enter => Action::AccountEditSave,
        KeyCode::Esc => Action::AccountEditCancel,
        KeyCode::Backspace => Action::AccountEditBackspace,
        KeyCode::Char(' ') => Action::AccountEditToggleDefault,
        KeyCode::Char(c) => Action::AccountEditInput(c),
        _ => Action::None,
    }
}
