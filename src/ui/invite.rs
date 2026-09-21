use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::theme;

/// Render the invitation response prompt in the status bar.
pub fn render_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let event = app
        .invite_pending
        .as_ref()
        .and_then(|event| event.summary_line())
        .unwrap_or_else(|| "Invitation".to_string());
    let spans = vec![
        Span::styled(" Invitation: ", t.status_key()),
        Span::styled(event, t.normal()),
        Span::styled(
            "  (a accept · t tentative · d decline · Esc cancel)",
            t.dimmed(),
        ),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
