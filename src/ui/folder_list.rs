use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::App;
use crate::keys::View;
use crate::mail::types::FolderRole;
use crate::theme::theme;

// Nerd Font icon for well-known folder roles.
fn folder_icon(role: FolderRole) -> &'static str {
    match role {
        FolderRole::Inbox => "󰇰 ",
        FolderRole::Sent => "󰑊 ",
        FolderRole::Drafts => "󰙏 ",
        FolderRole::Trash => "󰆴 ",
        FolderRole::Archive => "󰎞 ",
        FolderRole::Junk => "󰛃 ",
        FolderRole::Flagged => "󰓎 ",
        FolderRole::All | FolderRole::Other => "󰉋 ",
    }
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let focused = app.view == View::FolderList;

    let border_style = if focused {
        t.border_focused()
    } else {
        t.border()
    };

    let title = if app.folder_jump.is_empty() {
        " Folders ".to_string()
    } else {
        format!(" Folders · /{} ", app.folder_jump)
    };
    let block = Block::default()
        .title(title)
        .title_style(if focused {
            t.accent_style().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        })
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .folders
        .iter()
        .enumerate()
        .map(|(i, folder)| {
            let icon = folder_icon(folder.role);
            let unread = app.folder_unread.get(&folder.name).copied().unwrap_or(0);
            let content = if unread > 0 {
                format!("{icon}{} ({})", folder.name, unread)
            } else {
                format!("{icon}{}", folder.name)
            };
            // A dot marks a folder the account is not subscribed to, which is
            // why it does not appear in other mail clients.
            let content = match folder.subscribed {
                Some(false) => format!("{content} ·"),
                _ => content,
            };
            let style = if folder.name == app.current_folder {
                if focused && i == app.folder_index {
                    t.folder_active()
                } else {
                    t.accent_style().add_modifier(Modifier::BOLD)
                }
            } else if focused && i == app.folder_index {
                t.selected()
            } else {
                t.folder_inactive()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let mut state = ListState::default();
    if focused {
        state.select(Some(app.folder_index));
    } else {
        // Highlight the active folder
        let active = app
            .folders
            .iter()
            .position(|f| f.name == app.current_folder);
        state.select(active);
    }

    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut state);
}
