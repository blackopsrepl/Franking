//! Search prompt and multi-select keybinding tests.

use crossterm::event::KeyCode;
use franking::keys::{resolve, Action, View};
use pretty_assertions::assert_eq;

use super::support::key;

// ── Multi-select ────────────────────────────────────────────────────

#[test]
fn envelope_list_selects_for_batch_actions() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char(' '))),
        Action::ToggleSelect
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('u'))),
        Action::ClearSelection
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('z'))),
        Action::Undo
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('['))),
        Action::CollapseThread
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Left)),
        Action::CollapseThread
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('E'))),
        Action::EmptyFolder
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('e'))),
        Action::Archive
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('C'))),
        Action::CopyMessage
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char(']'))),
        Action::ExpandThread
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Right)),
        Action::ExpandThread
    );
}

// ── Move prompt ─────────────────────────────────────────────────────

#[test]
fn move_prompt_input() {
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Char('S'))),
        Action::MoveInput('S')
    );
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Backspace)),
        Action::MoveBackspace
    );
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Enter)),
        Action::MoveSubmit
    );
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Esc)),
        Action::MoveCancel
    );
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Char('j'))),
        Action::MoveNext
    );
    assert_eq!(
        resolve(View::MovePrompt, key(KeyCode::Up)),
        Action::MovePrev
    );
}

// ── Passphrase unlock prompt ────────────────────────────────────────

// ── Unrecognized keys ───────────────────────────────────────────────

#[test]
fn unrecognized_key_returns_none() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::F(12))),
        Action::None
    );
}

// ── New modal scheme tests (replaces old Ctrl+p/s tests) ─────────────────────

#[test]
fn unread_navigation_keys() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('>'))),
        Action::NextUnread
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('<'))),
        Action::PrevUnread
    );
}

#[test]
fn sort_order_key() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('o'))),
        Action::CycleSortOrder
    );
}

#[test]
fn mark_thread_read_key() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('r'))),
        Action::MarkThreadRead
    );
}
