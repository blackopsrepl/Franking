//! Envelope, message, folder, search, and prompt view tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{resolve, Action, View};

use super::support::key;

// ── Envelope list ───────────────────────────────────────────────────

#[test]
fn envelope_list_navigation() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('j'))),
        Action::MoveDown
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Down)),
        Action::MoveDown
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('k'))),
        Action::MoveUp
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Up)),
        Action::MoveUp
    );
}

#[test]
fn envelope_list_actions() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Enter)),
        Action::OpenMessage
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('c'))),
        Action::Compose
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('d'))),
        Action::Delete
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('/'))),
        Action::Search
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('!'))),
        Action::ToggleFlag
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('m'))),
        Action::MoveMessage
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Tab)),
        Action::FocusFolders
    );
}

#[test]
fn envelope_list_jumps() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('g'))),
        Action::JumpTop
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('G'))),
        Action::JumpBottom
    );
}

#[test]
fn envelope_list_paging() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('n'))),
        Action::PageDown
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('p'))),
        Action::PageUp
    );
}

// ── Message view ────────────────────────────────────────────────────

#[test]
fn message_view_navigation() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('j'))),
        Action::ScrollDown
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('k'))),
        Action::ScrollUp
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('q'))),
        Action::Back
    );
    assert_eq!(resolve(View::MessageView, key(KeyCode::Esc)), Action::Back);
}

#[test]
fn toggle_read_is_bound_in_mail_views() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('N'))),
        Action::ToggleRead
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('N'))),
        Action::ToggleRead
    );
}

#[test]
fn folder_actions_are_bound_in_the_envelope_list() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('S'))),
        Action::SyncFolder
    );
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('A'))),
        Action::MarkFolderRead
    );
}

#[test]
fn message_view_actions() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('r'))),
        Action::Reply
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('R'))),
        Action::ReplyAll
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('f'))),
        Action::Forward
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('a'))),
        Action::DownloadAttachments
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('d'))),
        Action::Delete
    );
}

// ── Folder list ─────────────────────────────────────────────────────

#[test]
fn folder_list_navigation() {
    assert_eq!(
        resolve(View::FolderList, key(KeyCode::Char('j'))),
        Action::MoveDown
    );
    assert_eq!(
        resolve(View::FolderList, key(KeyCode::Enter)),
        Action::Select
    );
    assert_eq!(
        resolve(View::FolderList, key(KeyCode::Tab)),
        Action::FocusEnvelopes
    );
}

// ── Search ──────────────────────────────────────────────────────────

#[test]
fn search_input() {
    assert_eq!(
        resolve(View::Search, key(KeyCode::Char('a'))),
        Action::SearchInput('a')
    );
    assert_eq!(
        resolve(View::Search, key(KeyCode::Backspace)),
        Action::SearchBackspace
    );
    assert_eq!(
        resolve(View::Search, key(KeyCode::Enter)),
        Action::SearchSubmit
    );
    assert_eq!(
        resolve(View::Search, key(KeyCode::Esc)),
        Action::SearchCancel
    );
}

// ── Help ────────────────────────────────────────────────────────────

#[test]
fn help_toggle() {
    assert_eq!(
        resolve(View::EnvelopeList, key(KeyCode::Char('?'))),
        Action::ToggleHelp
    );
    assert_eq!(
        resolve(View::Help, key(KeyCode::Char('?'))),
        Action::ToggleHelp
    );
    assert_eq!(resolve(View::Help, key(KeyCode::Esc)), Action::ToggleHelp);
}

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
