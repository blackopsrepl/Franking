//! Sieve filter browser and editor keybinding tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{resolve, Action, View};

use super::support::{ctrl, key};

#[test]
fn folder_list_opens_the_filter_browser() {
    assert_eq!(
        resolve(View::FolderList, key(KeyCode::Char('F'))),
        Action::OpenSieve
    );
}

#[test]
fn sieve_browser_keys() {
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('j'))),
        Action::SieveNext
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Down)),
        Action::SieveNext
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('k'))),
        Action::SievePrev
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Enter)),
        Action::SieveActivate
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('e'))),
        Action::SieveEdit
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('n'))),
        Action::SieveNew
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('d'))),
        Action::SieveDelete
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Char('x'))),
        Action::SieveDeactivate
    );
    assert_eq!(
        resolve(View::SieveScripts, key(KeyCode::Esc)),
        Action::SieveClose
    );
}

#[test]
fn sieve_name_prompt_keys() {
    assert_eq!(
        resolve(View::SieveName, key(KeyCode::Char('a'))),
        Action::SieveNameInput('a')
    );
    assert_eq!(
        resolve(View::SieveName, key(KeyCode::Backspace)),
        Action::SieveNameBackspace
    );
    assert_eq!(
        resolve(View::SieveName, key(KeyCode::Enter)),
        Action::SieveNameSubmit
    );
    assert_eq!(
        resolve(View::SieveName, key(KeyCode::Esc)),
        Action::SieveNameCancel
    );
}

#[test]
fn sieve_editor_saves_with_ctrl_s() {
    assert_eq!(
        resolve(View::SieveEdit, ctrl(KeyCode::Char('s'))),
        Action::SieveSave
    );

    assert_eq!(
        resolve(View::SieveEdit, key(KeyCode::Esc)),
        Action::SieveEscape
    );

    let typed = resolve(View::SieveEdit, key(KeyCode::Char('i')));
    assert_eq!(typed, Action::SieveEditorKey(key(KeyCode::Char('i'))));
}
