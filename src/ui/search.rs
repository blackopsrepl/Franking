use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::theme;

/// Render the search input bar (replaces the status bar when active).
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    }; // blinking block

    let spans = vec![
        Span::styled(" / ", t.status_key()),
        Span::styled(
            format!("{}{cursor_char}", app.search_query),
            t.search_input(),
        ),
        Span::styled(
            "  (matches subject, sender, date, id; use 'flag seen' or 'not flag seen')",
            t.dimmed(),
        ),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}
