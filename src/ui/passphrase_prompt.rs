use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::theme;

/// Render the passphrase input bar (replaces the status bar when active).
///
/// The typed passphrase is masked so it never appears on screen.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };

    let masked: String = std::iter::repeat_n('*', app.unlock_input.chars().count()).collect();

    let spans = vec![
        Span::styled(" PGP passphrase: ", t.status_key()),
        Span::styled(format!("{masked}{cursor_char}"), t.search_input()),
        Span::styled("  (Enter to unlock, Esc to cancel)", t.dimmed()),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}
