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
