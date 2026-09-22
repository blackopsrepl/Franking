//! Compose, identity, and contact modal tests.

use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;
use solverforge_mail::keys::{
    resolve, resolve_compose_with_context, Action, ComposeFocus, ComposeKeyContext, EditMode, View,
};

use super::support::{ctrl, key};

#[test]
fn tab_cycles_identity_edit_fields() {
    // Tab advances through fields: Name → SenderName → Email → IsDefault → Save → Cancel → Name
    assert_eq!(
        resolve(View::IdentityEdit, key(KeyCode::Tab)),
        Action::IdentityEditFieldNext
    );
    assert_eq!(
        resolve(View::IdentityEdit, key(KeyCode::BackTab)),
        Action::IdentityEditFieldPrev
    );
}

#[test]
fn enter_activates_identity_edit_focused_item() {
    // Enter on any field = IdentityEditToggle (which app.rs routes to save/cancel/next)
    assert_eq!(
        resolve(View::IdentityEdit, key(KeyCode::Enter)),
        Action::IdentityEditToggle
    );
}

#[test]
fn tab_cycles_contact_edit_fields() {
    assert_eq!(
        resolve(View::ContactEdit, key(KeyCode::Tab)),
        Action::ContactEditFieldNext
    );
    assert_eq!(
        resolve(View::ContactEdit, key(KeyCode::BackTab)),
        Action::ContactEditFieldPrev
    );
}

#[test]
fn enter_activates_contact_edit_focused_item() {
    // Enter = ContactEditActivate (app.rs checks focused field and dispatches)
    assert_eq!(
        resolve(View::ContactEdit, key(KeyCode::Enter)),
        Action::ContactEditActivate
    );
}

#[test]
fn compose_enter_triggers_enter_insert() {
    assert_eq!(
        resolve(View::Compose, key(KeyCode::Enter)),
        Action::EditorKey(key(KeyCode::Enter))
    );
}

#[test]
fn compose_esc_offers_to_leave_the_message() {
    // From a header field or the body, Esc leaves the message (asking first
    // when there is text to lose); only the action bar steps back to the body.
    assert_eq!(
        resolve(View::Compose, key(KeyCode::Esc)),
        Action::ComposeDiscard
    );
}

#[test]
fn compose_tab_advances_field() {
    assert_eq!(
        resolve(View::Compose, key(KeyCode::Tab)),
        Action::ComposeFieldNext
    );
}

#[test]
fn compose_default_text_is_forwarded() {
    assert_eq!(
        resolve(View::Compose, key(KeyCode::Char('j'))),
        Action::EditorKey(key(KeyCode::Char('j')))
    );
}

#[test]
fn compose_body_forwards_jk_to_editor() {
    let body_ctx = ComposeKeyContext {
        focus: ComposeFocus::Body,
        edit_mode: EditMode::Nav,
        body_search_active: false,
        autocomplete_visible: false,
        confirm_discard_visible: false,
    };
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('j')), body_ctx),
        Action::EditorKey(key(KeyCode::Char('j')))
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('k')), body_ctx),
        Action::EditorKey(key(KeyCode::Char('k')))
    );
}

#[test]
fn compose_header_insert_keeps_char_input() {
    let header_insert_ctx = ComposeKeyContext {
        focus: ComposeFocus::Header,
        edit_mode: EditMode::Insert,
        body_search_active: false,
        autocomplete_visible: false,
        confirm_discard_visible: false,
    };
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('j')), header_insert_ctx),
        Action::EditorKey(key(KeyCode::Char('j')))
    );
}

#[test]
fn compose_confirm_discard_context_intercepts_keys() {
    let confirm_ctx = ComposeKeyContext {
        focus: ComposeFocus::Header,
        edit_mode: EditMode::Nav,
        body_search_active: false,
        autocomplete_visible: false,
        confirm_discard_visible: true,
    };
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('y')), confirm_ctx),
        Action::ComposeConfirmDiscard
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Esc), confirm_ctx),
        Action::ComposeCancelDiscard
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('x')), confirm_ctx),
        Action::None
    );
}

#[test]
fn ctrl_c_still_discards_compose() {
    assert_eq!(
        resolve(View::Compose, ctrl(KeyCode::Char('c'))),
        Action::ComposeDiscard
    );
}

#[test]
fn ctrl_c_still_cancels_identity_edit() {
    assert_eq!(
        resolve(View::IdentityEdit, ctrl(KeyCode::Char('c'))),
        Action::IdentityEditCancel
    );
}

#[test]
fn ctrl_c_still_cancels_contact_edit() {
    assert_eq!(
        resolve(View::ContactEdit, ctrl(KeyCode::Char('c'))),
        Action::ContactEditCancel
    );
}

#[test]
fn schedule_prompt_keys() {
    assert_eq!(
        resolve(View::SchedulePrompt, key(KeyCode::Char('2'))),
        Action::ScheduleInput('2')
    );
    assert_eq!(
        resolve(View::SchedulePrompt, key(KeyCode::Backspace)),
        Action::ScheduleBackspace
    );
    assert_eq!(
        resolve(View::SchedulePrompt, key(KeyCode::Enter)),
        Action::ScheduleSubmit
    );
    assert_eq!(
        resolve(View::SchedulePrompt, key(KeyCode::Esc)),
        Action::ScheduleCancel
    );
}
