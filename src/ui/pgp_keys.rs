/*! Key material overlay: PGP keys and their storage. */

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::app::pgp_keys::KeyPrompt;
use crate::app::App;
use crate::ui::util::centered_rect;

use crate::theme::theme;

/// Render the key list, or the prompt it raised.
pub fn render(app: &App, frame: &mut Frame) {
    if let Some(prompt) = app.keys.prompt {
        render_prompt(app, frame, prompt);
        return;
    }

    let t = theme();
    let popup = centered_rect(84, 70, frame.area());
    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = app
        .keys
        .keys
        .iter()
        .enumerate()
        .map(|(index, key)| {
            let kind = if key.secret { "secret" } else { "public" };
            let emails = if key.emails.is_empty() {
                String::new()
            } else {
                format!("  ·  {}", key.emails.join(", "))
            };
            let pending = app.keys.pending_delete.as_deref() == Some(key.fingerprint.as_str());
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let marker = if index == app.keys.index { "▸" } else { " " };
            let line = format!("{marker} {kind:<6} {}{emails}{suffix}", key.identity);
            let style = if pending {
                t.error()
            } else if index == app.keys.index {
                t.accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let detail = if index == app.keys.index {
                Line::from(Span::styled(
                    format!("      {}", key.fingerprint_display()),
                    t.dimmed(),
                ))
            } else {
                Line::from("")
            };
            ListItem::new(vec![Line::from(Span::styled(line, style)), detail])
        })
        .collect();

    let title = if app.keys.keys.is_empty() {
        " Keys · none found ".to_string()
    } else {
        format!(" Keys · {} ", app.keys.keys.len())
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.border_focused())
            .title(title),
    );
    frame.render_widget(list, popup);
}

/// Render the import or key-generation prompt.
fn render_prompt(app: &App, frame: &mut Frame, prompt: KeyPrompt) {
    let t = theme();
    let popup = centered_rect(72, 24, frame.area());
    frame.render_widget(Clear, popup);

    let (title, hint) = match prompt {
        KeyPrompt::ImportPath => (
            " Import public key ",
            "Path of an armored PGP public key file",
        ),
        KeyPrompt::GenerateIdentity => (
            " Generate key pair ",
            "Identity, e.g. Alice <alice@example.com>",
        ),
    };
    let text = vec![
        Line::from(Span::styled(hint, t.dimmed())),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", t.accent_style()),
            Span::raw(app.keys.input.as_str()),
            Span::styled("█", t.accent_style()),
        ]),
        Line::from(""),
        Line::from(Span::styled("Enter to confirm · Esc to cancel", t.dimmed())),
    ];
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(t.border_focused())
                .title(title),
        ),
        popup,
    );
}
