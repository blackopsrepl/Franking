/*! One command catalogue for mail shortcuts, Actions, hints and help. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Action, View};

pub struct Command {
    pub label: &'static str,
    pub action: Action,
    pub key: Option<KeyCode>,
    pub shortcut: &'static str,
    pub hint: Option<&'static str>,
    pub menu: bool,
}

macro_rules! command {
    ($label:literal, $action:expr) => {
        Command {
            label: $label,
            action: $action,
            key: None,
            shortcut: "",
            hint: None,
            menu: true,
        }
    };
    ($label:literal, $action:expr, $key:expr, $shortcut:literal, $hint:expr, $menu:expr) => {
        Command {
            label: $label,
            action: $action,
            key: Some($key),
            shortcut: $shortcut,
            hint: $hint,
            menu: $menu,
        }
    };
}

use KeyCode::{Char, Enter, Esc, Tab};

#[rustfmt::skip]
static LIST: &[Command] = &[
    command!("Read message", Action::OpenMessage, Enter, "Enter", Some("Read"), false),
    command!("Compose message", Action::Compose, Char('c'), "c", Some("Compose"), true),
    command!("Archive", Action::Archive, Char('e'), "e", Some("Archive"), true),
    command!("Search mail", Action::Search, Char('/'), "/", Some("Search"), true),
    command!("Focus sidebar", Action::FocusFolders, Tab, "Tab", None, false),
    command!("Select message", Action::ToggleSelect, Char(' '), "Space", None, false),
    command!("Clear selection", Action::ClearSelection, Esc, "Esc", None, false),
    command!("Next page", Action::PageDown, Char('n'), "n", None, false),
    command!("Previous page", Action::PageUp, Char('p'), "p", None, false),
    command!("Next unread message", Action::NextUnread),
    command!("Previous unread message", Action::PrevUnread),
    command!("Toggle threaded view", Action::ToggleThread),
    command!("Change sort order", Action::CycleSortOrder),
    command!("Mark folder read", Action::MarkFolderRead),
    command!("Empty this folder", Action::EmptyFolder),
    command!("Cache folder for offline use", Action::SyncFolder),
    command!("Mark conversation read", Action::MarkThreadRead),
    command!("Collapse thread", Action::CollapseThread),
    command!("Expand thread", Action::ExpandThread),
    command!("Quit", Action::Quit, Char('q'), "q", None, true),
];

#[rustfmt::skip]
static READER: &[Command] = &[
    command!("Reply", Action::Reply, Char('r'), "r", Some("Reply"), true),
    command!("Reply all", Action::ReplyAll, Char('R'), "R", Some("Reply all"), true),
    command!("Archive", Action::Archive, Char('e'), "e", Some("Archive"), true),
    command!("Back to list", Action::Back, Esc, "Esc", Some("Back"), false),
    command!("Forward", Action::Forward, Char('f'), "f", None, true),
    command!("Move to folder", Action::MoveMessage, Char('m'), "m", None, true),
    command!("Attachments", Action::OpenAttachments, Char('a'), "a", None, true),
    command!("Find in message", Action::SearchMessage, Char('/'), "/", None, true),
    command!("Next match", Action::NextMatch, Char('n'), "n", None, false),
    command!("Previous match", Action::PrevMatch, Char('p'), "p", None, false),
    command!("Page down", Action::PageDown, Char(' '), "Space", None, false),
    command!("Save message as .eml", Action::SaveMessage),
    command!("Save all attachments", Action::DownloadAttachments),
    command!("Save attachments as archive", Action::DownloadAttachmentsZip),
    command!("Show all headers", Action::ToggleHeaders),
    command!("Collapse quoted text", Action::ToggleQuotes),
    command!("Show HTML source", Action::ToggleHtmlSource),
    command!("Open links", Action::OpenLinks),
    command!("Add invitation to calendar", Action::AddToPlanner),
    command!("Unlock encrypted message", Action::UnlockPrompt),
    command!("Trust signer certificate", Action::TrustSigner),
];

#[rustfmt::skip]
static MESSAGE: &[Command] = &[
    command!("Delete", Action::Delete, Char('d'), "d", None, true),
    command!("Move to folder", Action::MoveMessage, Char('m'), "m", None, true),
    command!("Toggle star", Action::ToggleFlag, Char('!'), "!", None, true),
    command!("Toggle read / unread", Action::ToggleRead, Char('N'), "N", None, true),
    command!("Toggle Reply later", Action::ToggleReplyLater, Char('y'), "y", None, true),
    command!("Undo last mail action", Action::Undo, Char('z'), "z", None, true),
    command!("Copy to folder", Action::CopyMessage),
    command!("Mute / unmute conversation", Action::ToggleMuteConversation),
];

#[rustfmt::skip]
static FOLDERS: &[Command] = &[
    command!("Open folder or view", Action::Select, Enter, "Enter", Some("Open"), false),
    command!("Focus messages", Action::FocusEnvelopes, Tab, "Tab", Some("Messages"), false),
    command!("Back to messages", Action::FocusEnvelopes, Esc, "Esc", None, false),
    command!("Create folder", Action::FolderNew),
    command!("Rename folder", Action::FolderRename),
    command!("Delete folder", Action::FolderDelete),
    command!("Toggle folder subscription", Action::FolderSubscribe),
    command!("Manage server filters", Action::OpenSieve),
];

#[rustfmt::skip]
static COMMON: &[Command] = &[
    command!("Actions", Action::OpenActions, Char(':'), ":", Some("Actions"), false),
    command!("Help", Action::ToggleHelp, Char('?'), "?", Some("Help"), false),
    command!("Compose message", Action::Compose),
    command!("Open Reply later", Action::OpenReplyLater),
    command!("Open Starred (cached mail)", Action::OpenStarred),
    command!("Switch account", Action::SwitchAccount),
    command!("Refresh", Action::Refresh),
    command!("Preferences", Action::OpenSettings),
    command!("Sender folder rules", Action::OpenFolderRules),
    command!("Contacts", Action::OpenContacts),
    command!("Sender identities", Action::OpenIdentities),
    command!("Outbox and scheduled mail", Action::OpenOutbox),
    command!("Saved searches", Action::OpenSavedSearches),
    command!("Attachment library", Action::OpenAttachmentLibrary),
    command!("Encryption keys", Action::OpenKeys),
    command!("Quit", Action::Quit),
];

pub fn commands(view: View) -> Vec<&'static Command> {
    let specific = match view {
        View::EnvelopeList => LIST,
        View::MessageView => READER,
        View::FolderList => FOLDERS,
        _ => return Vec::new(),
    };
    let message = if view == View::FolderList {
        &[][..]
    } else {
        MESSAGE
    };
    let mut result: Vec<&Command> = Vec::new();
    for command in specific.iter().chain(message).chain(COMMON) {
        if !result
            .iter()
            .any(|existing| existing.action == command.action)
        {
            result.push(command);
        }
    }
    result
}

pub fn resolve(view: View, key: KeyEvent) -> Action {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return Action::None;
    }
    match key.code {
        Char('q') if view == View::MessageView => return Action::Back,
        Char('q') if view == View::FolderList => return Action::FocusEnvelopes,
        KeyCode::Down | Char('j') => {
            return if view == View::MessageView {
                Action::ScrollDown
            } else {
                Action::MoveDown
            }
        }
        KeyCode::Up | Char('k') => {
            return if view == View::MessageView {
                Action::ScrollUp
            } else {
                Action::MoveUp
            }
        }
        KeyCode::Home | Char('g') if view != View::FolderList => return Action::JumpTop,
        KeyCode::End | Char('G') if view != View::FolderList => return Action::JumpBottom,
        _ => {}
    }
    commands(view)
        .into_iter()
        .find(|command| command.key == Some(key.code))
        .map(|command| command.action.clone())
        .unwrap_or(Action::None)
}
