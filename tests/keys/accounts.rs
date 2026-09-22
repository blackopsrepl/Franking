//! Account list keybinding tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{resolve, Action, View};

use super::support::{ctrl, key};

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
    assert_eq!(
        resolve(View::AccountEdit, ctrl(KeyCode::Char('d'))),
        Action::AccountEditDiscover
    );
}

#[test]
fn file_picker_keys() {
    assert_eq!(
        resolve(View::FilePicker, key(KeyCode::Char('j'))),
        Action::FilePickerNext
    );
    assert_eq!(
        resolve(View::FilePicker, key(KeyCode::Enter)),
        Action::FilePickerEnter
    );
    assert_eq!(
        resolve(View::FilePicker, key(KeyCode::Backspace)),
        Action::FilePickerUp
    );
    assert_eq!(
        resolve(View::FilePicker, key(KeyCode::Esc)),
        Action::FilePickerClose
    );
}

// ── Outbox ──────────────────────────────────────────────────────────

#[test]
fn outbox_keys() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('O'))),
        Action::OpenOutbox
    );
    assert_eq!(
        resolve(View::Outbox, key(KeyCode::Char('j'))),
        Action::OutboxNext
    );
    assert_eq!(
        resolve(View::Outbox, key(KeyCode::Enter)),
        Action::OutboxSend
    );
    assert_eq!(
        resolve(View::Outbox, key(KeyCode::Char('d'))),
        Action::OutboxDiscard
    );
    assert_eq!(
        resolve(View::Outbox, key(KeyCode::Esc)),
        Action::OutboxClose
    );
}

#[test]
fn settings_navigation_keys() {
    assert_eq!(
        resolve(View::Settings, key(KeyCode::Char('j'))),
        Action::SettingsNext
    );
    assert_eq!(
        resolve(View::Settings, key(KeyCode::Char('k'))),
        Action::SettingsPrev
    );
}

#[test]
fn settings_keys() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('P'))),
        Action::OpenSettings
    );
    assert_eq!(
        resolve(View::Settings, key(KeyCode::Char(' '))),
        Action::SettingsToggleNotifications
    );
    assert_eq!(
        resolve(View::Settings, key(KeyCode::Esc)),
        Action::SettingsClose
    );
}
