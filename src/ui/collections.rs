use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the collections board.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .collections
        .items
        .iter()
        .enumerate()
        .map(|(index, collection)| {
            let pending = app.collections.pending_delete == Some(collection.id);
            let filter = app.collection_filter.as_deref() == Some(collection.name.as_str());
            let member = app.collections.target.iter().any(|anchor| {
                app.collection_of_anchor
                    .get(anchor)
                    .is_some_and(|names| names.iter().any(|name| name == &collection.name))
            });
            let marker = if index == app.collections.index {
                "▸"
            } else {
                " "
            };
            let mut suffix = String::new();
            if member {
                suffix.push_str("  · member");
            }
            if filter {
                suffix.push_str("  · filtering");
            }
            if pending {
                suffix.push_str("  · press d to confirm");
            }
            let style = if pending {
                t.error()
            } else if index == app.collections.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker} {}{suffix}", collection.name),
                style,
            )))
        })
        .collect();

    let title = if app.collections.items.is_empty() {
        " Collections · none "
    } else {
        " Collections "
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
    if !app.collections.items.is_empty() {
        state.select(Some(app.collections.index));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

/// Render the collection-name prompt in the status bar.
pub fn render_name_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Collection name: ", t.status_key()),
        Span::styled(
            format!("{}{cursor}", app.collections.input),
            t.search_input(),
        ),
        Span::styled("  (Enter saves, Esc cancels)", t.dimmed()),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
