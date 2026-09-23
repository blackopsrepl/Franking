use ratatui::prelude::*;

use crate::app::App;
use crate::keys::View;
use crate::theme::theme;

/// Render the note or subject-alias prompt in the status bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let (label, hint) = if app.view == View::MessageNote {
        (
            " Note: ",
            "  (private; blank clears; Enter saves, Esc cancels)",
        )
    } else {
        (
            " Rename subject: ",
            "  (yours only; blank restores; Enter saves, Esc cancels)",
        )
    };
    let spans = vec![
        Span::styled(label, t.status_key()),
        Span::styled(
            format!("{}{cursor}", app.annotation_input),
            t.search_input(),
        ),
        Span::styled(hint, t.dimmed()),
    ];
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
