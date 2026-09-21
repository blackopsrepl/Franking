use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::theme::theme;

/// Render the move-to-folder picker above the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let full = frame.area();
    let candidates = app.move_candidates();

    if !candidates.is_empty() {
        let height = (candidates.len() as u16 + 2).min(10);
        let popup = Rect {
            x: full.x + 2,
            y: area.y.saturating_sub(height + 1),
            width: full.width.saturating_sub(4),
            height,
        };
        frame.render_widget(Clear, popup);

        let items: Vec<ListItem> = candidates
            .iter()
            .map(|name| ListItem::new(Line::from(Span::styled(format!(" {name} "), t.normal()))))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .title(Span::styled(" Move to ", t.popup_title()))
                    .borders(Borders::ALL)
                    .border_style(t.border_focused())
                    .style(t.popup()),
            )
            .highlight_style(t.selected())
            .highlight_symbol("▸");
        let mut state = ListState::default();
        state.select(Some(app.move_index.min(candidates.len() - 1)));
        frame.render_stateful_widget(list, popup, &mut state);
    }

    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };

    let spans = vec![
        Span::styled(" Move to: ", t.status_key()),
        Span::styled(
            format!("{}{cursor_char}", app.move_target),
            t.search_input(),
        ),
        Span::styled("  (j/k pick, Enter to confirm, Esc to cancel)", t.dimmed()),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}
