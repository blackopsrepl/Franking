//! Account list keybinding tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{resolve, Action, View};

use super::support::key;

// ── Account list ────────────────────────────────────────────────────

#[test]
fn account_list_manages_defaults_and_deletion() {
    assert_eq!(
        resolve(View::AccountList, key(KeyCode::Char('s'))),
        Action::SetDefaultAccount
    );
    assert_eq!(
        resolve(View::AccountList, key(KeyCode::Char('d'))),
        Action::DeleteAccount
    );
}

#[test]
fn account_list_opens_the_account_form() {
    assert_eq!(
        resolve(View::AccountList, key(KeyCode::Char('a'))),
        Action::AccountNew
    );
    assert_eq!(
        resolve(View::AccountList, key(KeyCode::Char('e'))),
        Action::AccountEdit
    );
}

#[test]
fn account_form_keys() {
    assert_eq!(
        resolve(View::AccountEdit, key(KeyCode::Tab)),
        Action::AccountEditFieldNext
    );
    assert_eq!(
        resolve(View::AccountEdit, key(KeyCode::BackTab)),
        Action::AccountEditFieldPrev
    );
    assert_eq!(
        resolve(View::AccountEdit, key(KeyCode::Enter)),
        Action::AccountEditSave
    );
    assert_eq!(
        resolve(View::AccountEdit, key(KeyCode::Esc)),
        Action::AccountEditCancel
    );
    assert_eq!(
        resolve(View::AccountEdit, key(KeyCode::Char('x'))),
        Action::AccountEditInput('x')
    );
}
