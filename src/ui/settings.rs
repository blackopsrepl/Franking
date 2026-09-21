use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the preferences overlay.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Preferences ", t.popup_title()))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        Line::from(vec![
            Span::styled(" Desktop notifications  ", t.header_label()),
            Span::styled(
                if app.notifications_enabled {
                    "[x] on"
                } else {
                    "[ ] off"
                },
                t.normal(),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Space toggles · Esc closes", t.dimmed())),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}
