use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the outbox overlay.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(78, 60, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .outbox
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let protection = match (item.sign, item.encrypt) {
                (true, true) => " · sign + encrypt",
                (true, false) => " · signed",
                (false, true) => " · encrypted",
                (false, false) => "",
            };
            let pending = app.outbox.pending_discard == Some(item.id);
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let marker = if index == app.outbox.index {
                "▸"
            } else {
                " "
            };
            ListItem::new(Line::from(Span::styled(
                format!(
                    " {marker} {}{protection}  ·  {}{suffix} ",
                    item.subject, item.created_at
                ),
                if pending { t.error() } else { t.normal() },
            )))
        })
        .collect();

    let title = if app.outbox.items.is_empty() {
        " Outbox · empty ".to_string()
    } else {
        format!(" Outbox · {} queued ", app.outbox.items.len())
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
    state.select(Some(app.outbox.index));
    frame.render_stateful_widget(list, popup, &mut state);
}
