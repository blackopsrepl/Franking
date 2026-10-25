//! Shared key-event helpers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(crate) fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

pub(crate) fn menu_only(
    view: franking::keys::View,
    old_key: KeyCode,
    action: franking::keys::Action,
) {
    let commands = franking::keys::commands::commands(view);
    assert!(commands
        .iter()
        .any(|entry| entry.menu && entry.action == action));
    assert_ne!(franking::keys::resolve(view, key(old_key)), action);
}
