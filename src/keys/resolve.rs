/*! View dispatch for key events. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::resolve_accounts::{
    resolve_account_edit, resolve_account_list, resolve_attachment_view, resolve_file_picker,
    resolve_outbox, resolve_schedule, resolve_settings,
};
use super::resolve_contacts::{
    resolve_compose, resolve_contact_edit, resolve_contact_search, resolve_contacts,
    resolve_identity_edit, resolve_identity_list,
};
use super::resolve_keys::{resolve_keys, resolve_keys_prompt};
use super::resolve_message_view::resolve_message_view;
use super::resolve_saved::{resolve_save_search, resolve_saved_searches};
use super::resolve_sieve::{resolve_sieve_edit, resolve_sieve_name, resolve_sieve_scripts};
use super::view::View;

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
        View::Settings => return resolve_settings(key),
        View::SchedulePrompt => return resolve_schedule(key),
        View::AttachmentView => return resolve_attachment_view(key),
        View::LinkList => return resolve_link_list(key),
        View::SavedSearches => return resolve_saved_searches(key),
        View::SaveSearch => return resolve_save_search(key),
        View::Search => {
            // While naming a search the prompt owns the keys.
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Action::SaveSearch;
            }
            return resolve_search(key);
        }
        View::Keys => return resolve_keys(key),
        View::KeysPrompt => return resolve_keys_prompt(key),
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
        | View::Outbox
        | View::Settings
        | View::SchedulePrompt
        | View::AttachmentView
        | View::Keys
        | View::KeysPrompt
        | View::SavedSearches
        | View::SaveSearch => Action::None,
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
        KeyCode::Char('s') => Action::OpenSavedSearches,
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
        KeyCode::Char('>') => Action::NextUnread,
        KeyCode::Char('<') => Action::PrevUnread,
        KeyCode::Char('e') => Action::Archive,
        KeyCode::Char('E') => Action::EmptyFolder,
        KeyCode::Char('O') => Action::OpenOutbox,
        KeyCode::Char('o') => Action::CycleSortOrder,
        KeyCode::Char('r') => Action::MarkThreadRead,
        KeyCode::Char('P') => Action::OpenSettings,
        KeyCode::Char('K') => Action::OpenKeys,
        KeyCode::Char('[') | KeyCode::Left => Action::CollapseThread,
        KeyCode::Char(']') | KeyCode::Right => Action::ExpandThread,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('I') => Action::OpenIdentities,
        KeyCode::Tab => Action::FocusFolders,
        KeyCode::Esc => Action::Quit,
        _ => Action::None,
    }
}
fn resolve_folder_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::FocusEnvelopes,
        KeyCode::Esc => Action::FolderJumpClear,
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
        KeyCode::Char('s') => Action::FolderSubscribe,
        _ => Action::None,
    }
}

fn resolve_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SearchSubmit,
        KeyCode::Tab => Action::ToggleSearchScope,
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
        KeyCode::Char('v') => Action::AttachmentView,
        KeyCode::Char('s') => Action::AttachmentSave,
        KeyCode::Char('S') => Action::AttachmentSaveAs,
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
