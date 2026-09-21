use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::theme;

/// Render the folder-management input bar (replaces the status bar).
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let Some(prompt) = app.folder_prompt.as_ref() else {
        return;
    };
    let t = theme();

    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };

    let spans = vec![
        Span::styled(prompt.label(), t.status_key()),
        Span::styled(format!("{}{cursor_char}", prompt.input), t.search_input()),
        Span::styled(prompt.hint(), t.dimmed()),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}
