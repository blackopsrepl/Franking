use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the clip library.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(80, 65, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .clips
        .items
        .iter()
        .enumerate()
        .map(|(index, clip)| {
            let pending = app.clips.pending_delete == Some(clip.id);
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let marker = if index == app.clips.index { "▸" } else { " " };
            let style = if pending {
                t.error()
            } else if index == app.clips.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{marker} {}{suffix}", clip.body),
                    style,
                )),
                Line::from(Span::styled(format!("      {}", clip.source), t.dimmed())),
            ])
        })
        .collect();

    let title = if app.clips.items.is_empty() {
        " Clips · none "
    } else {
        " Clips "
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
    if !app.clips.items.is_empty() {
        state.select(Some(app.clips.index));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

/// Render the clip capture prompt in the status bar.
pub fn render_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Clip: ", t.status_key()),
        Span::styled(format!("{}{cursor}", app.clips.input), t.search_input()),
        Span::styled("  (Enter saves, Esc cancels)", t.dimmed()),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
