use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the link list overlay for the current message.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .message_content
        .as_ref()
        .map(|message| {
            message
                .body
                .links
                .iter()
                .map(|link| {
                    let text = if link.text.trim().is_empty() {
                        link.href.clone()
                    } else {
                        format!("{} — {}", link.text, link.href)
                    };
                    ListItem::new(Line::from(Span::styled(format!(" {text} "), t.normal())))
                })
                .collect()
        })
        .unwrap_or_default();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    " Links · Enter open · Esc close ",
                    t.popup_title(),
                ))
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .style(t.popup()),
        )
        .highlight_style(t.selected())
        .highlight_symbol("▸");

    let mut state = ListState::default();
    state.select(Some(app.link_index));
    frame.render_stateful_widget(list, popup, &mut state);
}
