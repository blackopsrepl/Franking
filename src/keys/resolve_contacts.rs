/*! Resolvers for compose, contact, and identity views. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::compose_context::{ComposeFocus, ComposeKeyContext, EditMode};
use super::resolve_compose::resolve_compose_with_context;

pub(super) fn resolve_compose(key: KeyEvent) -> Action {
    resolve_compose_with_context(
        key,
        ComposeKeyContext {
            focus: ComposeFocus::Header,
            edit_mode: EditMode::Nav,
            body_search_active: false,
            autocomplete_visible: false,
            confirm_discard_visible: false,
        },
    )
}

pub(super) fn resolve_contact_edit(key: KeyEvent) -> Action {
    // Ctrl+C / Ctrl+Q cancel without saving
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::ContactEditCancel,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc => Action::ContactEditCancel,
        KeyCode::Tab => Action::ContactEditFieldNext,
        KeyCode::BackTab => Action::ContactEditFieldPrev,
        KeyCode::Enter => Action::ContactEditActivate,
        KeyCode::Backspace => Action::ContactEditBackspace,
        KeyCode::Char(c) => Action::ContactEditInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_contact_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::ContactSearchCancel,
        KeyCode::Enter => Action::ContactSearchCancel,
        KeyCode::Backspace => Action::ContactSearchBackspace,
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ContactSearchInput(c)
        }
        _ => Action::None,
    }
}

pub(super) fn resolve_identity_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::IdentityListClose,
        KeyCode::Char('j') | KeyCode::Down => Action::IdentityListDown,
        KeyCode::Char('k') | KeyCode::Up => Action::IdentityListUp,
        KeyCode::Char('n') => Action::IdentityNew,
        KeyCode::Char('e') | KeyCode::Enter => Action::IdentityEditSelected,
        KeyCode::Char('d') => Action::IdentityDelete,
        KeyCode::Char('s') => Action::IdentitySetDefault,
        _ => Action::None,
    }
}

pub(super) fn resolve_identity_edit(key: KeyEvent) -> Action {
    // Ctrl+C / Ctrl+Q cancel without saving
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::IdentityEditCancel,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc => Action::IdentityEditCancel,
        KeyCode::Tab => Action::IdentityEditFieldNext,
        KeyCode::BackTab => Action::IdentityEditFieldPrev,
        KeyCode::Enter => Action::IdentityEditToggle,
        KeyCode::Backspace => Action::IdentityEditBackspace,
        KeyCode::Char(c) => Action::IdentityEditInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_contacts(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('b') => Action::Back,
            KeyCode::Char('c') | KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Char('t') => Action::CycleContactTag,
        KeyCode::Esc | KeyCode::Char('q') => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Char('n') => Action::ContactNew,
        KeyCode::Char('d') => Action::ContactDelete,
        KeyCode::Char('e') => Action::ContactEdit,
        KeyCode::Char('/') => Action::ContactSearch,
        KeyCode::Char('c') => Action::Compose,
        _ => Action::None,
    }
}
