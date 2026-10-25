/*! View dispatch for key events. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::commands;
use super::resolve_accounts::{
    resolve_account_edit, resolve_account_list, resolve_attachment_view, resolve_file_picker,
    resolve_outbox, resolve_schedule, resolve_settings,
};
use super::resolve_contacts::{
    resolve_compose, resolve_contact_edit, resolve_contact_search, resolve_contacts,
    resolve_identity_edit, resolve_identity_list,
};
use super::resolve_keys::{resolve_keys, resolve_keys_prompt};
use super::resolve_prompts::{
    resolve_attachment_library, resolve_attachment_list, resolve_folder_prompt, resolve_help,
    resolve_link_list, resolve_message_search, resolve_move_prompt, resolve_search,
    resolve_snippet_name, resolve_snippets, resolve_unlock_prompt,
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
        View::AttachmentLibrary => return resolve_attachment_library(key),
        View::Snippets => return resolve_snippets(key),
        View::SnippetName => return resolve_snippet_name(key),
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
            _ => Action::None,
        };
    }
    match view {
        View::EnvelopeList | View::MessageView | View::FolderList => commands::resolve(view, key),
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
        | View::AttachmentLibrary
        | View::Snippets
        | View::SnippetName
        | View::AttachmentView
        | View::Keys
        | View::KeysPrompt
        | View::SavedSearches
        | View::SaveSearch => Action::None,
    }
}
