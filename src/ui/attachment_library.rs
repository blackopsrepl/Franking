use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Human-readable size for one attachment row.
fn size_label(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Render the cross-account attachment library.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .attachment_library
        .items
        .iter()
        .map(|item| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("  {}  ·  {}", item.file_name, size_label(item.size)),
                    t.normal(),
                )),
                Line::from(Span::styled(
                    format!(
                        "      {}  ·  {} · {}  ·  {}",
                        item.subject, item.account, item.folder, item.sender
                    ),
                    t.dimmed(),
                )),
            ])
        })
        .collect();

    let title = if app.attachment_library.items.is_empty() {
        " Attachments · none cached ".to_string()
    } else {
        format!(" Attachments · {} ", app.attachment_library.items.len())
    };
    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, t.popup_title()))
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .style(t.popup()),
        )
        .highlight_style(t.selected());

    let mut state = ListState::default();
    if !app.attachment_library.items.is_empty() {
        state.select(Some(app.attachment_library.index));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}
