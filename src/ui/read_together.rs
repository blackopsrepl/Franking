use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme::theme;

/// Render each selected message in one scrollable column.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let inner_width = area.width.saturating_sub(2) as usize;
    let text = app.render_read_together(inner_width);
    let count = app.read_together.as_ref().map(Vec::len).unwrap_or(0);
    let paragraph = Paragraph::new(text)
        .style(t.normal())
        .block(
            Block::default()
                .title(format!(" Read together · {count} "))
                .title_style(t.accent_style().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .style(t.popup()),
        )
        .scroll((app.read_together_scroll, 0));
    frame.render_widget(paragraph, area);
}
