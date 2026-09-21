//! Message-view crypto and attachment overlay tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{resolve, Action, View};

use super::support::key;

#[test]
fn message_view_opens_unlock_prompt() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('P'))),
        Action::UnlockPrompt
    );
}

#[test]
fn message_view_toggles_all_headers() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('h'))),
        Action::ToggleHeaders
    );
}

#[test]
fn message_view_lists_links() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('l'))),
        Action::OpenLinks
    );
    assert_eq!(
        resolve(View::LinkList, key(KeyCode::Char('j'))),
        Action::LinkNext
    );
    assert_eq!(
        resolve(View::LinkList, key(KeyCode::Enter)),
        Action::LinkOpen
    );
    assert_eq!(
        resolve(View::LinkList, key(KeyCode::Esc)),
        Action::LinkClose
    );
}

#[test]
fn message_view_archives() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('e'))),
        Action::Archive
    );
}

#[test]
fn message_view_searches_within_the_message() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('/'))),
        Action::SearchMessage
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('n'))),
        Action::NextMatch
    );
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('p'))),
        Action::PrevMatch
    );
    assert_eq!(
        resolve(View::MessageSearch, key(KeyCode::Enter)),
        Action::MessageSearchSubmit
    );
    assert_eq!(
        resolve(View::MessageSearch, key(KeyCode::Esc)),
        Action::MessageSearchCancel
    );
    assert_eq!(
        resolve(View::MessageSearch, key(KeyCode::Char('x'))),
        Action::MessageSearchInput('x')
    );
}

#[test]
fn message_view_undoes_the_last_action() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('z'))),
        Action::Undo
    );
}

#[test]
fn message_view_saves_the_source() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('s'))),
        Action::SaveMessage
    );
}

#[test]
fn message_view_trusts_the_smime_signer() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('T'))),
        Action::TrustSigner
    );
}

#[test]
fn message_view_opens_the_attachment_list() {
    assert_eq!(
        resolve(View::MessageView, key(KeyCode::Char('o'))),
        Action::OpenAttachments
    );
}

#[test]
fn attachment_list_keys() {
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Char('j'))),
        Action::AttachmentNext
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Down)),
        Action::AttachmentNext
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Char('k'))),
        Action::AttachmentPrev
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Enter)),
        Action::AttachmentOpen
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Char('s'))),
        Action::AttachmentSave
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Esc)),
        Action::AttachmentClose
    );
    assert_eq!(
        resolve(View::AttachmentList, key(KeyCode::Char('q'))),
        Action::AttachmentClose
    );
}

#[test]
fn unlock_prompt_input() {
    assert_eq!(
        resolve(View::PassphrasePrompt, key(KeyCode::Char('s'))),
        Action::UnlockInput('s')
    );
    assert_eq!(
        resolve(View::PassphrasePrompt, key(KeyCode::Backspace)),
        Action::UnlockBackspace
    );
    assert_eq!(
        resolve(View::PassphrasePrompt, key(KeyCode::Enter)),
        Action::UnlockSubmit
    );
    assert_eq!(
        resolve(View::PassphrasePrompt, key(KeyCode::Esc)),
        Action::UnlockCancel
    );
}
