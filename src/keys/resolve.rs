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
use super::resolve_prompts::resolve_bypass;
use super::resolve_prompts::resolve_focus;
use super::resolve_prompts::resolve_read_together;
use super::resolve_prompts::{
    resolve_annotation, resolve_attachment_library, resolve_attachment_list, resolve_folder_prompt,
    resolve_help, resolve_link_list, resolve_message_search, resolve_move_prompt, resolve_place,
    resolve_resurface, resolve_search, resolve_unlock_prompt,
};
use super::resolve_prompts::{
    resolve_clip_prompt, resolve_clips, resolve_snippet_name, resolve_snippets,
};
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
        View::ResurfacePrompt => return resolve_resurface(key),
        View::PlacePrompt => return resolve_place(key),
        View::AttachmentLibrary => return resolve_attachment_library(key),
        View::ReadTogether => return resolve_read_together(key),
        View::BypassPrompt => return resolve_bypass(key),
        View::FocusReply => return resolve_focus(key),
        View::Snippets => return resolve_snippets(key),
        View::SnippetName => return resolve_snippet_name(key),
        View::Clips => return resolve_clips(key),
        View::ClipPrompt => return resolve_clip_prompt(key),
        View::MessageNote | View::SubjectAlias => return resolve_annotation(key),
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
            KeyCode::Char('l') => Action::OpenAttachmentLibrary,
            KeyCode::Char('k') => Action::OpenClips,
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
        | View::ResurfacePrompt
        | View::PlacePrompt
        | View::MessageNote
        | View::SubjectAlias
        | View::AttachmentLibrary
        | View::ReadTogether
        | View::BypassPrompt
        | View::FocusReply
        | View::Snippets
        | View::SnippetName
        | View::Clips
        | View::ClipPrompt
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
        KeyCode::Char('v') => Action::CycleTriageLane,
        KeyCode::Char('L') => Action::OpenFollowup(crate::db::message_markers::Marker::ReplyLater),
        KeyCode::Char('D') => Action::OpenFollowup(crate::db::message_markers::Marker::Saved),
        KeyCode::Char('y') => Action::ToggleMarker(crate::db::message_markers::Marker::ReplyLater),
        KeyCode::Char('Y') => Action::ToggleMarker(crate::db::message_markers::Marker::Saved),
        KeyCode::Char('M') => Action::ToggleMuteConversation,
        KeyCode::Char('b') => Action::OpenResurface,
        KeyCode::Char('x') => Action::OpenPlacePrompt,
        KeyCode::Char('i') => Action::OpenNote,
        KeyCode::Char('%') => Action::OpenSubjectAlias,
        KeyCode::Char('1') => Action::RouteSender(crate::db::sender_routes::Route::Inbox),
        KeyCode::Char('2') => Action::RouteSender(crate::db::sender_routes::Route::Reading),
        KeyCode::Char('3') => Action::RouteSender(crate::db::sender_routes::Route::Receipts),
        KeyCode::Char('4') => Action::RouteSender(crate::db::sender_routes::Route::Blocked),
        KeyCode::Char('5') => Action::RouteSender(crate::db::sender_routes::Route::Screening),
        KeyCode::Char('T') => Action::OpenReadTogether,
        KeyCode::Char('V') => Action::ToggleCoverReveal,
        KeyCode::Char('F') => Action::OpenFocusReply,
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
