//! Shared key-event helpers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(crate) fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}
