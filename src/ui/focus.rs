use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme::theme;

/// Render one message of the sequential reply queue.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let Some(focus) = app.focus.as_ref() else {
        return;
    };
    let total = focus.queue.len();
    let position = if total == 0 { 0 } else { focus.index + 1 };
    let envelope = focus.queue.get(focus.index);
    let title = match envelope {
        Some(envelope) => format!(
            " Focus \u{00b7} {position}/{total} \u{00b7} {} \u{00b7} {} ",
            envelope.sender_display(),
            envelope.subject
        ),
        None => format!(" Focus \u{00b7} {position}/{total} "),
    };

    let inner_width = area.width.saturating_sub(2) as usize;
    let body = match focus.document.as_ref() {
        Some(document) => document.render(inner_width),
        None => String::new(),
    };
    let paragraph = Paragraph::new(body)
        .style(t.normal())
        .block(
            Block::default()
                .title(title)
                .title_style(t.accent_style().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .style(t.popup()),
        )
        .scroll((focus.scroll, 0));
    frame.render_widget(paragraph, area);
}
