use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::theme::theme;

/// Render the move-to-folder picker above the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let full = frame.area();
    let candidates = app.move_candidates();
    if !app.move_is_copy {
        let owner = app
            .selected_envelope()
            .and_then(|e| e.account.as_deref())
            .unwrap_or("?");
        let sender = app
            .move_rule_sender()
            .unwrap_or_else(|| "select one message with a sender mailbox".into());
        let checkbox = Rect::new(
            full.x + 2,
            area.y.saturating_sub(2),
            full.width.saturating_sub(4),
            2,
        );
        frame.render_widget(Clear, checkbox);
        let lines = vec![
            Line::raw(format!(" {owner} · {sender}")),
            Line::raw(format!(
                " [{}] Also move future messages from this sender · Tab toggle",
                if app.folder_rules.also_future {
                    "x"
                } else {
                    " "
                }
            )),
        ];
        frame.render_widget(Paragraph::new(lines).style(t.popup()), checkbox);
    }

    if !candidates.is_empty() {
        let height = (candidates.len() as u16 + 2).min(10);
        let popup = Rect {
            x: full.x + 2,
            y: area.y.saturating_sub(height + 2),
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
                    .title(Span::styled(
                        if app.move_is_copy {
                            " Copy to "
                        } else {
                            " Move to "
                        },
                        t.popup_title(),
                    ))
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
        Span::styled(
            if app.move_is_copy {
                " Copy to: "
            } else {
                " Move to: "
            },
            t.status_key(),
        ),
        Span::styled(
            format!("{}{cursor_char}", app.move_target),
            t.search_input(),
        ),
        Span::styled("  (↑/↓ pick, Enter confirm, Esc cancel)", t.dimmed()),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}
