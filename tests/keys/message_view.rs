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
