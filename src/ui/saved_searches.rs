/*! Saved-search overlay. */

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the saved searches.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .saved_searches
        .searches
        .iter()
        .enumerate()
        .map(|(index, search)| {
            let scope = search.scope.label();
            let pending =
                app.saved_searches.pending_delete.as_deref() == Some(search.name.as_str());
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let marker = if index == app.saved_searches.index {
                "▸"
            } else {
                " "
            };
            let style = if pending {
                t.error()
            } else if index == app.saved_searches.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{marker} {}{suffix}", search.name),
                    style,
                )),
                Line::from(Span::styled(
                    format!("      {}  ·  {scope}", search.query),
                    t.dimmed(),
                )),
            ])
        })
        .collect();

    let title = if app.saved_searches.searches.is_empty() {
        " Saved searches · none ".to_string()
    } else {
        format!(" Saved searches · {} ", app.saved_searches.searches.len())
    };
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .title(title),
        ),
        popup,
    );
}
