use ratatui::prelude::*;

use crate::app::App;
use crate::theme::theme;

/// Render the resurface-delay prompt in the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Resurface in: ", t.status_key()),
        Span::styled(format!("{}{cursor}", app.resurface_input), t.search_input()),
        Span::styled(
            "  (30m, 2h, 1d; blank or 'off' clears; Enter confirms, Esc cancels)",
            t.dimmed(),
        ),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
