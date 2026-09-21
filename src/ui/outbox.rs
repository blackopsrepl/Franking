use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the send-later prompt in the status bar.
pub fn render_schedule_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Send in: ", t.status_key()),
        Span::styled(format!("{}{cursor}", app.schedule_input), t.search_input()),
        Span::styled(
            "  (e.g. 30m, 2h, 1d; Enter schedules, Esc cancels)",
            t.dimmed(),
        ),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}

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
                    item.subject,
                    crate::compose::describe_send_after(
                        item.send_after.as_deref(),
                        &chrono::Local::now().to_rfc3339(),
                    )
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
