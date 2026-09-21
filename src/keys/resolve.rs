/*! View dispatch for key events. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::{Action, ComposeFocus, ComposeKeyContext, View};
use super::resolve_accounts::{
    resolve_account_edit, resolve_account_list, resolve_file_picker, resolve_outbox,
};
use super::resolve_contacts::{
    resolve_compose, resolve_contact_edit, resolve_contact_search, resolve_contacts,
    resolve_identity_edit, resolve_identity_list,
};
use super::resolve_sieve::{resolve_sieve_edit, resolve_sieve_name, resolve_sieve_scripts};

pub fn resolve(view: View, key: KeyEvent) -> Action {
    match view {
        View::Compose => return resolve_compose(key),
        View::Contacts => return resolve_contacts(key),
        View::ContactSearch => return resolve_contact_search(key),
        View::ContactEdit => return resolve_contact_edit(key),
        View::IdentityList => return resolve_identity_list(key),
        View::IdentityEdit => return resolve_identity_edit(key),
        View::MessageSearch => return resolve_message_search(key),
        View::AccountEdit => return resolve_account_edit(key),
        View::FilePicker => return resolve_file_picker(key),
        View::Outbox => return resolve_outbox(key),
        View::LinkList => return resolve_link_list(key),
        View::SieveScripts => return resolve_sieve_scripts(key),
        View::SieveName => return resolve_sieve_name(key),
        View::SieveEdit => return resolve_sieve_edit(key),
        _ => {}
    }

    // Global keybindings (handled first)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('a') => Action::SwitchAccount,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('b') => Action::OpenContacts,
            _ => Action::None,
        };
    }

    match view {
        View::EnvelopeList => resolve_envelope_list(key),
        View::MessageView => resolve_message_view(key),
        View::FolderList => resolve_folder_list(key),
        View::AccountList => resolve_account_list(key),
        View::Search => resolve_search(key),
        View::Help => resolve_help(key),
        View::MovePrompt => resolve_move_prompt(key),
        View::PassphrasePrompt => resolve_unlock_prompt(key),
        View::AttachmentList => resolve_attachment_list(key),
        View::LinkList => resolve_link_list(key),
        View::MessageSearch => resolve_message_search(key),
        View::FolderPrompt => resolve_folder_prompt(key),
        View::ContactSearch => resolve_contact_search(key),
        View::ContactEdit => resolve_contact_edit(key),
        // Already handled above
        View::Compose
        | View::Contacts
        | View::IdentityList
        | View::IdentityEdit
        | View::SieveScripts
        | View::SieveName
        | View::SieveEdit
        | View::AccountEdit
        | View::FilePicker
        | View::Outbox => Action::None,
    }
}

/// Resolve compose keys with compose-state context.
///
/// Compose is focus-driven: the shell owns modal overlays, field cycling, and
/// action-bar activation, while the focused field handles its own editing.
///
/// Priority order (highest first):
/// 1) discard-confirm modal interception
/// 2) global compose shortcuts (`Ctrl+C` / `Ctrl+Q`)
/// 3) autocomplete popup navigation/accept keys
/// 4) compose shell controls (`Tab`, `Shift+Tab`, action-bar `Enter` / `Esc`)
/// 5) passthrough to the focused compose field
pub fn resolve_compose_with_context(key: KeyEvent, ctx: ComposeKeyContext) -> Action {
    // Discard confirmation modal owns key handling while visible.
    if ctx.confirm_discard_visible {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ComposeConfirmDiscard,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Action::ComposeCancelDiscard,
            _ => Action::None,
        };
    }

    // Allow Ctrl+C / Ctrl+Q globally in compose as quit-discard
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::ComposeDiscard,
            _ => Action::EditorKey(key),
        };
    }

    // If autocomplete popup is open, let app-level popup handler own navigation
    // and acceptance keys.
    if ctx.autocomplete_visible {
        match key.code {
            KeyCode::Down | KeyCode::Up | KeyCode::Enter | KeyCode::Tab | KeyCode::Esc => {
                return Action::EditorKey(key);
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Tab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyNext
        }
        KeyCode::BackTab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyPrev
        }
        KeyCode::Tab => Action::ComposeFieldNext,
        KeyCode::BackTab => Action::ComposeFieldPrev,
        KeyCode::Down if ctx.focus != ComposeFocus::Body => Action::ComposeFieldNext,
        KeyCode::Up if ctx.focus != ComposeFocus::Body => Action::ComposeFieldPrev,
        KeyCode::Enter if ctx.focus == ComposeFocus::ActionBar => Action::ComposeEnterInsert,
        KeyCode::Esc if ctx.focus == ComposeFocus::ActionBar => Action::ComposeExitToNav,
        _ => Action::EditorKey(key),
    }
}

fn resolve_envelope_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Enter => Action::OpenMessage,
        KeyCode::Char('c') => Action::Compose,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('m') => Action::MoveMessage,
        KeyCode::Char('C') => Action::CopyMessage,
        KeyCode::Char('!') => Action::ToggleFlag,
        KeyCode::Char('N') => Action::ToggleRead,
        KeyCode::Char('S') => Action::SyncFolder,
        KeyCode::Char('A') => Action::MarkFolderRead,
        KeyCode::Char('/') => Action::Search,
        KeyCode::Char('n') => Action::PageDown,
        KeyCode::Char('p') => Action::PageUp,
        KeyCode::Char('t') => Action::ToggleThread,
        KeyCode::Char(' ') => Action::ToggleSelect,
        KeyCode::Char('u') => Action::ClearSelection,
        KeyCode::Char('z') => Action::Undo,
        KeyCode::Char('e') => Action::Archive,
        KeyCode::Char('E') => Action::EmptyFolder,
        KeyCode::Char('O') => Action::OpenOutbox,
        KeyCode::Char('[') | KeyCode::Left => Action::CollapseThread,
        KeyCode::Char(']') | KeyCode::Right => Action::ExpandThread,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('I') => Action::OpenIdentities,
        KeyCode::Tab => Action::FocusFolders,
        KeyCode::Esc => Action::Quit,
        _ => Action::None,
    }
}

fn resolve_message_view(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char(' ') => Action::PageDown,
        KeyCode::Char('r') => Action::Reply,
        KeyCode::Char('R') => Action::ReplyAll,
        KeyCode::Char('f') => Action::Forward,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('a') => Action::DownloadAttachments,
        KeyCode::Char('N') => Action::ToggleRead,
        KeyCode::Char('P') => Action::UnlockPrompt,
        KeyCode::Char('T') => Action::TrustSigner,
        KeyCode::Char('o') => Action::OpenAttachments,
        KeyCode::Char('h') => Action::ToggleHeaders,
        KeyCode::Char('Q') => Action::ToggleQuotes,
        KeyCode::Char('s') => Action::SaveMessage,
        KeyCode::Char('z') => Action::Undo,
        KeyCode::Char('e') => Action::Archive,
        KeyCode::Char('C') => Action::CopyMessage,
        KeyCode::Char('l') => Action::OpenLinks,
        KeyCode::Char('/') => Action::SearchMessage,
        KeyCode::Char('n') => Action::NextMatch,
        KeyCode::Char('p') => Action::PrevMatch,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        _ => Action::None,
    }
}

fn resolve_folder_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::FocusEnvelopes,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter => Action::Select,
        KeyCode::Tab => Action::FocusEnvelopes,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('I') => Action::OpenIdentities,
        KeyCode::Char('n') => Action::FolderNew,
        KeyCode::Char('r') => Action::FolderRename,
        KeyCode::Char('d') => Action::FolderDelete,
        KeyCode::Char('F') => Action::OpenSieve,
        _ => Action::None,
    }
}

fn resolve_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SearchSubmit,
        KeyCode::Esc => Action::SearchCancel,
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => Action::SearchInput(c),
        _ => Action::None,
    }
}

fn resolve_help(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        _ => Action::None,
    }
}

fn resolve_link_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::LinkNext,
        KeyCode::Char('k') | KeyCode::Up => Action::LinkPrev,
        KeyCode::Enter | KeyCode::Char('o') => Action::LinkOpen,
        KeyCode::Esc | KeyCode::Char('q') => Action::LinkClose,
        _ => Action::None,
    }
}

fn resolve_message_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::MessageSearchSubmit,
        KeyCode::Esc => Action::MessageSearchCancel,
        KeyCode::Backspace => Action::MessageSearchBackspace,
        KeyCode::Char(c) => Action::MessageSearchInput(c),
        _ => Action::None,
    }
}

fn resolve_folder_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::FolderPromptSubmit,
        KeyCode::Esc => Action::FolderPromptCancel,
        KeyCode::Backspace => Action::FolderPromptBackspace,
        KeyCode::Char(c) => Action::FolderPromptInput(c),
        _ => Action::None,
    }
}

fn resolve_attachment_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::AttachmentNext,
        KeyCode::Char('k') | KeyCode::Up => Action::AttachmentPrev,
        KeyCode::Enter | KeyCode::Char('o') => Action::AttachmentOpen,
        KeyCode::Char('s') => Action::AttachmentSave,
        KeyCode::Esc | KeyCode::Char('q') => Action::AttachmentClose,
        _ => Action::None,
    }
}

fn resolve_unlock_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::UnlockSubmit,
        KeyCode::Esc => Action::UnlockCancel,
        KeyCode::Backspace => Action::UnlockBackspace,
        KeyCode::Char(c) => Action::UnlockInput(c),
        _ => Action::None,
    }
}

fn resolve_move_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::MoveSubmit,
        KeyCode::Esc => Action::MoveCancel,
        KeyCode::Backspace => Action::MoveBackspace,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveNext,
        KeyCode::Char('k') | KeyCode::Up => Action::MovePrev,
        KeyCode::Char(c) => Action::MoveInput(c),
        _ => Action::None,
    }
}
