/*! Searchable, context-specific actions without another key namespace. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::model::App;
use crate::keys::{
    commands::{self, Command},
    Action, View,
};

pub(crate) struct ActionsMenu {
    pub query: String,
    pub index: usize,
    pub origin: View,
    pub server_mailbox: bool,
}

impl ActionsMenu {
    pub fn matches(&self) -> Vec<&'static Command> {
        let query = self.query.to_lowercase();
        commands::commands(self.origin)
            .into_iter()
            .filter(|command| {
                command.menu
                    && (self.server_mailbox
                        || !matches!(
                            command.action,
                            Action::MarkFolderRead
                                | Action::EmptyFolder
                                | Action::SyncFolder
                                | Action::MarkThreadRead
                                | Action::ToggleThread
                                | Action::CollapseThread
                                | Action::ExpandThread
                                | Action::FolderRename
                                | Action::FolderDelete
                                | Action::FolderSubscribe
                        ))
                    && query
                        .split_whitespace()
                        .all(|word| command.label.to_lowercase().contains(word))
            })
            .collect()
    }
}

impl App {
    pub(crate) fn mail_footer_height(&self) -> u16 {
        if matches!(
            self.view,
            View::EnvelopeList | View::MessageView | View::FolderList
        ) && (!self.status_message.is_empty() || self.loading)
        {
            2
        } else {
            1
        }
    }

    pub(crate) fn open_actions_menu(&mut self) {
        if matches!(
            self.view,
            View::EnvelopeList | View::MessageView | View::FolderList
        ) {
            self.actions_menu = Some(ActionsMenu {
                query: String::new(),
                index: 0,
                origin: self.view,
                server_mailbox: if self.view == View::FolderList {
                    self.folders
                        .get(self.folder_index)
                        .is_some_and(|f| f.name != super::model::UNIFIED_INBOX)
                } else {
                    self.local_view.is_none() && !self.is_unified_inbox()
                },
            });
        }
    }

    pub(crate) fn handle_actions_menu_key(&mut self, key: KeyEvent) -> bool {
        let Some(menu) = self.actions_menu.as_mut() else {
            return false;
        };
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
                self.actions_menu = None;
            }
            return true;
        }
        match key.code {
            KeyCode::Esc => self.actions_menu = None,
            KeyCode::Down => {
                menu.index = (menu.index + 1).min(menu.matches().len().saturating_sub(1));
            }
            KeyCode::Up => menu.index = menu.index.saturating_sub(1),
            KeyCode::Backspace => {
                menu.query.pop();
                menu.index = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::ALT) => {
                menu.query.push(c);
                menu.index = 0;
            }
            KeyCode::Enter => {
                let action = menu
                    .matches()
                    .get(menu.index)
                    .map(|command| command.action.clone());
                self.actions_menu = None;
                if let Some(action) = action {
                    self.dispatch_action(action);
                }
            }
            _ => {}
        }
        true
    }
}
