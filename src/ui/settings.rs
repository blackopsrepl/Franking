use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the preferences overlay.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Preferences ", t.popup_title()))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let autosave = if app.autosave_seconds == 0 {
        "off".to_string()
    } else {
        format!("{}s", app.autosave_seconds)
    };
    let rows = [
        (
            "Desktop notifications".to_string(),
            app.notification_rule.label().to_string(),
        ),
        (
            "Mark read on open".to_string(),
            toggle_label(app.mark_read_on_open),
        ),
        (
            "Encrypt drafts".to_string(),
            toggle_label(app.encrypt_drafts),
        ),
        ("Page size".to_string(), app.page_size.to_string()),
        ("Compose autosave".to_string(), autosave),
        (
            "Cover previously seen".to_string(),
            toggle_label(app.cover_seen),
        ),
        (
            "Bypass token".to_string(),
            app.bypass_token
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
        ),
    ];

    let mut lines = Vec::new();
    for (index, (label, value)) in rows.iter().enumerate() {
        let focused = app.settings_index == index;
        let style = if focused { t.selected() } else { t.normal() };
        lines.push(Line::from(Span::styled(
            format!(" {} {label:<22} {value}", if focused { "▸" } else { " " }),
            style,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k move · Space changes · Esc closes",
        t.dimmed(),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn toggle_label(enabled: bool) -> String {
    if enabled {
        "[x] on".to_string()
    } else {
        "[ ] off".to_string()
    }
}
