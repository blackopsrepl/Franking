use ratatui::prelude::*;

use crate::app::App;
use crate::theme::theme;

/// Render the per-message placement prompt in the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let spans = vec![
        Span::styled(" Place this message: ", t.status_key()),
        Span::styled("1", t.status_key()),
        Span::styled(" Inbox  ", t.status_desc()),
        Span::styled("2", t.status_key()),
        Span::styled(" Reading  ", t.status_desc()),
        Span::styled("3", t.status_key()),
        Span::styled(" Receipts  ", t.status_desc()),
        Span::styled("4", t.status_key()),
        Span::styled(" Blocked  ", t.status_desc()),
        Span::styled("5", t.status_key()),
        Span::styled(" Screening  ", t.status_desc()),
        Span::styled("(Esc cancels)", t.dimmed()),
    ];
    let _ = app;
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
