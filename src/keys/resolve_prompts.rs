/*! Key resolution for prompts and small overlays. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

pub(super) fn resolve_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SearchSubmit,
        KeyCode::Tab => Action::ToggleSearchScope,
        KeyCode::Esc => Action::SearchCancel,
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => Action::SearchInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_help(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        _ => Action::None,
    }
}

pub(super) fn resolve_link_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::LinkNext,
        KeyCode::Char('k') | KeyCode::Up => Action::LinkPrev,
        KeyCode::Enter | KeyCode::Char('o') => Action::LinkOpen,
        KeyCode::Esc | KeyCode::Char('q') => Action::LinkClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_message_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::MessageSearchSubmit,
        KeyCode::Esc => Action::MessageSearchCancel,
        KeyCode::Backspace => Action::MessageSearchBackspace,
        KeyCode::Char(c) => Action::MessageSearchInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_folder_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::FolderPromptSubmit,
        KeyCode::Esc => Action::FolderPromptCancel,
        KeyCode::Backspace => Action::FolderPromptBackspace,
        KeyCode::Char(c) => Action::FolderPromptInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_attachment_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::AttachmentNext,
        KeyCode::Char('k') | KeyCode::Up => Action::AttachmentPrev,
        KeyCode::Enter | KeyCode::Char('o') => Action::AttachmentOpen,
        KeyCode::Char('v') => Action::AttachmentView,
        KeyCode::Char('s') => Action::AttachmentSave,
        KeyCode::Char('S') => Action::AttachmentSaveAs,
        KeyCode::Esc | KeyCode::Char('q') => Action::AttachmentClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_unlock_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::UnlockSubmit,
        KeyCode::Esc => Action::UnlockCancel,
        KeyCode::Backspace => Action::UnlockBackspace,
        KeyCode::Char(c) => Action::UnlockInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_move_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::MoveSubmit,
        KeyCode::Esc => Action::MoveCancel,
        KeyCode::Backspace => Action::MoveBackspace,
        KeyCode::Down => Action::MoveNext,
        KeyCode::Up => Action::MovePrev,
        KeyCode::Char(c) => Action::MoveInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_attachment_library(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::AttachmentLibraryNext,
        KeyCode::Char('k') | KeyCode::Up => Action::AttachmentLibraryPrev,
        KeyCode::Enter | KeyCode::Char('o') => Action::AttachmentLibraryOpen,
        KeyCode::Esc | KeyCode::Char('q') => Action::AttachmentLibraryClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_snippets(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::SnippetsNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SnippetsPrev,
        KeyCode::Enter => Action::SnippetsInsert,
        KeyCode::Char('s') => Action::SnippetNew,
        KeyCode::Char('d') => Action::SnippetsDelete,
        KeyCode::Esc | KeyCode::Char('q') => Action::SnippetsClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_snippet_name(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SnippetNameSubmit,
        KeyCode::Esc => Action::SnippetNameCancel,
        KeyCode::Backspace => Action::SnippetNameBackspace,
        KeyCode::Char(c) => Action::SnippetNameInput(c),
        _ => Action::None,
    }
}
