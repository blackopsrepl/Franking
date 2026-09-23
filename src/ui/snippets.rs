use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the snippet picker.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .snippets
        .items
        .iter()
        .enumerate()
        .map(|(index, snippet)| {
            let pending = app.snippets.pending_delete.as_deref() == Some(snippet.name.as_str());
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let marker = if index == app.snippets.index {
                "▸"
            } else {
                " "
            };
            let preview = snippet.body.lines().next().unwrap_or_default();
            let style = if pending {
                t.error()
            } else if index == app.snippets.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{marker} {}{suffix}", snippet.name),
                    style,
                )),
                Line::from(Span::styled(format!("      {preview}"), t.dimmed())),
            ])
        })
        .collect();

    let title = if app.snippets.items.is_empty() {
        " Snippets · none "
    } else {
        " Snippets "
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
    if !app.snippets.items.is_empty() {
        state.select(Some(app.snippets.index));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

/// Render the snippet-name prompt in the status bar.
pub fn render_name_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Save body as snippet: ", t.status_key()),
        Span::styled(
            format!("{}{cursor}", app.snippets.name_input),
            t.search_input(),
        ),
        Span::styled("  (Enter saves, Esc cancels)", t.dimmed()),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
