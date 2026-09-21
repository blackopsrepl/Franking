use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::theme::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let block = Block::default()
        .title(" Message ")
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(t.border_focused());

    let Some(message) = app.current_message() else {
        let paragraph = Paragraph::new("No message loaded.")
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    if app.show_all_headers {
        for header in app.all_headers() {
            lines.push(Line::from(Span::styled(header, t.header_value())));
        }
    } else {
        for header in message.header_fields() {
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", header.name), t.header_label()),
                Span::styled(header.value.clone(), t.header_value()),
            ]));
        }
    }

    if let Some(summary) = message.authentication().summary() {
        lines.push(Line::from(Span::styled(summary, t.dimmed())));
    }
    if let Some(protection) = message.protection() {
        lines.push(Line::from(Span::styled(protection.label(), t.dimmed())));
    }
    if let Some(status) = &app.pgp_status {
        lines.push(Line::from(Span::styled(status.clone(), t.dimmed())));
    }
    if let Some(event) = message.invitation() {
        if let Some(summary) = event.summary_line() {
            lines.push(Line::from(Span::styled(
                format!("Invitation: {summary}"),
                t.dimmed(),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(area.width.saturating_sub(4) as usize),
        t.dimmed(),
    )));

    for raw_line in app
        .render_message_body(area.width.saturating_sub(4) as usize)
        .lines()
    {
        let style = if raw_line.starts_with('>') {
            t.dimmed()
        } else {
            t.normal()
        };
        lines.push(Line::from(Span::styled(raw_line.to_string(), style)));
    }

    if !message.body.links.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Links", t.header_label())));
        for link in &message.body.links {
            lines.push(Line::from(Span::styled(
                format!("  {} \u{2014} {}", link.text, link.href),
                t.normal(),
            )));
        }
    }

    if !message.attachments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Attachments", t.header_label())));
        for attachment in &message.attachments {
            let name = attachment
                .file_name
                .clone()
                .unwrap_or_else(|| "(unnamed)".to_string());
            let content_type = attachment
                .content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let disposition = if attachment.is_inline {
                "inline"
            } else {
                "attachment"
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "  {name} [{content_type}, {disposition}, {} bytes]",
                    attachment.size
                ),
                t.normal(),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.message_scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Render the in-message search prompt in the status bar.
pub fn render_search_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Find in message: ", t.status_key()),
        Span::styled(
            format!("{}{cursor_char}", app.message_search),
            t.search_input(),
        ),
        Span::styled("  (Enter to find, Esc to cancel)", t.dimmed()),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}
