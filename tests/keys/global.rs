//! Global keybinding tests.

use crossterm::event::KeyCode;
use franking::keys::{resolve, Action, View};
use pretty_assertions::assert_eq;

use super::support::key;

use super::support::ctrl;

#[test]
fn ctrl_c_quits_from_any_view() {
    assert_eq!(
        resolve(View::EnvelopeList, ctrl(KeyCode::Char('c'))),
        Action::Quit
    );
    assert_eq!(
        resolve(View::MessageView, ctrl(KeyCode::Char('c'))),
        Action::Quit
    );
    assert_eq!(
        resolve(View::FolderList, ctrl(KeyCode::Char('c'))),
        Action::Quit
    );
}

#[test]
fn ctrl_a_switches_account() {
    assert_eq!(
        resolve(View::EnvelopeList, ctrl(KeyCode::Char('a'))),
        Action::SwitchAccount
    );
}

#[test]
fn ctrl_r_refreshes() {
    assert_eq!(
        resolve(View::EnvelopeList, ctrl(KeyCode::Char('r'))),
        Action::Refresh
    );
}

#[test]
fn contacts_view_cycles_the_tag_filter() {
    assert_eq!(
        resolve(View::Contacts, key(KeyCode::Char('t'))),
        Action::CycleContactTag
    );
}
