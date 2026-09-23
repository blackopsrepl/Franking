use ratatui::prelude::*;

use crate::app::App;
use crate::theme::theme;

/// Render the screening bypass-token prompt in the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Bypass token: ", t.status_key()),
        Span::styled(format!("{}{cursor}", app.bypass_input), t.search_input()),
        Span::styled(
            "  (Tab generates; blank clears; Enter saves, Esc cancels)",
            t.dimmed(),
        ),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
