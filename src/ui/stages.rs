use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the workflow stage board.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .stages
        .items
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let pending = app.stages.pending_delete == Some(stage.id);
            let filter = app.stage_filter.as_deref() == Some(stage.name.as_str());
            let marker = if index == app.stages.index {
                "▸"
            } else {
                " "
            };
            let mut suffix = String::new();
            if filter {
                suffix.push_str("  · filtering");
            }
            if pending {
                suffix.push_str("  · press d to confirm");
            }
            let style = if pending {
                t.error()
            } else if index == app.stages.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker} {}{suffix}", stage.name),
                style,
            )))
        })
        .collect();

    let title = if app.stages.items.is_empty() {
        " Stages · none "
    } else {
        " Stages "
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
    if !app.stages.items.is_empty() {
        state.select(Some(app.stages.index));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

/// Render the stage-name prompt in the status bar.
pub fn render_name_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Stage name: ", t.status_key()),
        Span::styled(format!("{}{cursor}", app.stages.input), t.search_input()),
        Span::styled("  (Enter saves, Esc cancels)", t.dimmed()),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
